//! dumpx: the across-builds check of continuation dumps (0178, TESTING.md "Dumps across
//! builds"). scripts/dumps-across-builds.sh builds it twice: against an older commit's
//! djs to export a dump at every idle point of the codemode corpus, and against this
//! commit's to import each dump, finish the run and compare it with the older build's
//! uninterrupted run. It uses only the dump API of 0172 (`Vm::dump` with `DumpOptions {
//! base, host_fn_name, handles }`, `Vm::load_dump` with `LoadOptions { host_fn }`).
//!
//!   dumpx export CORPUS OUT   writes OUT/<case>.<k>.js (the dump at idle point k),
//!                             OUT/<case>.<k>.prefix (the transcript so far) and
//!                             OUT/<case>.full (the whole run)
//!   dumpx import OUT          loads every dump, finishes the run, prints counts

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use djs_core::compiler::CompileOptions;
use djs_core::heap::Handle;
use djs_core::value::Value;
use djs_core::vm::dump::DumpOptions;
use djs_core::vm::dump_load::LoadOptions;
use djs_core::vm::{Status, Vm, VmOptions};

const OUTPUT: u32 = 0;
const DONE: u32 = 1;
const LATER: u32 = 2;

fn host_fn_name(id: u32) -> Option<String> {
    match id {
        OUTPUT => Some("output".into()),
        DONE => Some("done".into()),
        LATER => Some("later".into()),
        _ => None,
    }
}

fn host_fn(name: &str) -> Option<u32> {
    match name {
        "output" => Some(OUTPUT),
        "done" => Some(DONE),
        "later" => Some(LATER),
        _ => None,
    }
}

/// A run: the VM, the prelude's api, the transcript, whether it reported its end.
struct Run {
    vm: Vm,
    api: Option<Handle>,
    out: Vec<String>,
    done: bool,
}

impl Run {
    /// Runs until the entry call and its jobs are done, settling `later` calls at idle
    /// points (oldest first, as the difftest runner does); `at_idle` sees each idle
    /// point first.
    fn drive(&mut self, at_idle: &mut dyn FnMut(&mut Run)) -> Result<Value, String> {
        let mut slices = 0;
        loop {
            match self.vm.run(10_000_000) {
                Status::Done(v) => return Ok(v),
                Status::Threw(e) => return Err(format!("threw {}", self.vm.display(e))),
                Status::OutOfFuel => {
                    slices += 1;
                    if slices > 100 {
                        return Err("out of fuel".into());
                    }
                }
                Status::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).ok_or("no call")?;
                    let s = |vm: &Vm, v: Value| {
                        v.as_string()
                            .map(|t| vm.heap().str_to_string(t))
                            .unwrap_or_default()
                    };
                    match fn_id {
                        OUTPUT => {
                            let t = s(&self.vm, info.args[0]);
                            self.out.push(t);
                        }
                        DONE => {
                            let t = format!(
                                "=== {} {}",
                                s(&self.vm, info.args[0]),
                                s(&self.vm, info.args[1])
                            );
                            self.out.push(t);
                            self.done = true;
                        }
                        _ => return Err(format!("host function {fn_id}")),
                    }
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .map_err(|e| format!("{e:?}"))?;
                }
                Status::Idle => {
                    at_idle(self);
                    let Some(&id) = self.vm.pending_host_calls().iter().min() else {
                        return Err("idle with nothing pending".into());
                    };
                    let info = self.vm.host_call(id).ok_or("no call")?;
                    let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
                    let rejected = info.args.get(1) == Some(&Value::TRUE);
                    self.vm
                        .resolve_host_call(id, if rejected { Err(v) } else { Ok(v) })
                        .map_err(|e| format!("{e:?}"))?;
                }
                s => return Err(format!("{s:?}")),
            }
        }
    }

    fn call(&mut self, f: Value, this: Value, args: &[Value], at_idle: &mut dyn FnMut(&mut Run)) -> Result<Value, String> {
        self.vm.call(f, this, args).map_err(|e| format!("{e:?}"))?;
        self.drive(at_idle)
    }

    fn api_call(&mut self, name: &str, args: &[Value], at_idle: &mut dyn FnMut(&mut Run)) -> Result<(), String> {
        let api = self.vm.heap().get_handle(self.api.ok_or("no api")?);
        let o = api.as_object().ok_or("api")?;
        let k = self.vm.heap_mut().atom_str(name).map_err(|e| format!("{e:?}"))?;
        let f = self
            .vm
            .heap()
            .find_property(o, k)
            .and_then(|(_, p)| p.value())
            .ok_or("no api method")?;
        self.call(f, api, args, at_idle).map(|_| ())
    }
}

fn new_vm() -> Vm {
    djs_prelude::new_vm(VmOptions::default()).expect("new_vm")
}

/// The difftest's run of a case (D1100, D2590): the prelude, its host object, the case
/// as `async (tools, console) => {...}`, `api.run`, `api.stalled`.
fn run_case(prelude: &str, case: &str, at_idle: &mut dyn FnMut(&mut Run)) -> Result<Vec<String>, String> {
    let mut r = Run { vm: new_vm(), api: None, out: Vec::new(), done: false };
    let p = r
        .vm
        .compile(prelude.as_bytes(), &CompileOptions::script("prelude.js"))
        .map_err(|e| e.message)?;
    let g = Value::object(r.vm.global());
    let setup = r.call(p, g, &[], &mut |_| {})?;
    let host = r.vm.new_object().map_err(|e| format!("{e:?}"))?;
    for (id, name) in [(OUTPUT, "output"), (DONE, "done"), (LATER, "later")] {
        let f = if id == LATER {
            r.vm.new_async_host_function(id, name)
        } else {
            r.vm.new_host_function(id, name)
        }
        .map_err(|e| format!("{e:?}"))?;
        r.vm.define(host, name, f).map_err(|e| format!("{e:?}"))?;
    }
    let api = r.call(setup, Value::UNDEFINED, &[Value::object(host)], &mut |_| {})?;
    r.api = Some(r.vm.heap_mut().new_handle(api).map_err(|e| format!("{e:?}"))?);
    let src = format!("(async (tools, console) => {{{case}\n}})");
    let script = r
        .vm
        .compile(src.as_bytes(), &CompileOptions::script("case.js"))
        .map_err(|e| e.message)?;
    let g = Value::object(r.vm.global());
    let f = r.call(script, g, &[], &mut |_| {})?;
    r.api_call("run", &[f], at_idle)?;
    if !r.done {
        r.api_call("stalled", &[], &mut |_| {})?;
    }
    Ok(clean(r.out))
}

/// Without stack frames (they name the dump's script after a load), as the difftest
/// compares.
fn clean(out: Vec<String>) -> Vec<String> {
    out.into_iter()
        .map(|o| {
            o.lines()
                .filter(|l| !l.trim_start().starts_with("at "))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

fn cases(corpus: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for area in ["codemode", "codemode-harvest"] {
        let Ok(dir) = std::fs::read_dir(corpus.join(area)) else { continue };
        let mut files: Vec<PathBuf> = dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "js"))
            .collect();
        files.sort();
        for f in files {
            let name = format!("{area}__{}", f.file_stem().unwrap().to_string_lossy());
            out.push((name, std::fs::read_to_string(&f).unwrap()));
        }
    }
    out
}

fn sep(lines: &[String]) -> String {
    lines.join("\n\u{1e}\n")
}

fn export(corpus: &Path, out: &Path) {
    std::fs::create_dir_all(out).unwrap();
    let prelude = std::fs::read_to_string(corpus.join("prelude.js")).unwrap();
    let (mut dumps, mut refused) = (0, BTreeMap::<String, usize>::new());
    for (name, case) in cases(corpus) {
        let mut k = 0;
        let mut at_idle = |r: &mut Run| {
            k += 1;
            let api = r.vm.heap().get_handle(r.api.unwrap());
            let handles = [("api", api)];
            let opts = DumpOptions {
                base: djs_prelude::BASE,
                host_fn_name: &host_fn_name,
                handles: &handles,
                ..DumpOptions::default()
            };
            match r.vm.dump(&opts, None) {
                Ok(text) => {
                    dumps += 1;
                    std::fs::write(out.join(format!("{name}.{k}.js")), text).unwrap();
                    let prefix = clean(r.out.clone());
                    std::fs::write(out.join(format!("{name}.{k}.prefix")), sep(&prefix)).unwrap();
                }
                Err(e) => {
                    let why = e.0.first().cloned().unwrap_or_default();
                    let why = why.split_once(": ").map_or(why.clone(), |(_, w)| w.to_string());
                    *refused.entry(why).or_default() += 1;
                }
            }
        };
        match run_case(&prelude, &case, &mut at_idle) {
            Ok(full) => std::fs::write(out.join(format!("{name}.full")), sep(&full)).unwrap(),
            Err(e) => eprintln!("{name}: {e}"),
        }
    }
    println!("export: {dumps} dumps; refused at the export: {refused:?}");
}

fn import(dir: &Path) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "js"))
        .collect();
    files.sort();
    let (mut same, mut differ) = (0, Vec::new());
    let mut refused = BTreeMap::<String, usize>::new();
    for f in &files {
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let case = stem.rsplit_once('.').unwrap().0.to_string();
        let text = std::fs::read_to_string(f).unwrap();
        let prefix = std::fs::read_to_string(f.with_extension("prefix")).unwrap();
        let Ok(full) = std::fs::read_to_string(dir.join(format!("{case}.full"))) else { continue };
        let mut r = Run { vm: new_vm(), api: None, out: Vec::new(), done: false };
        let loaded = r.vm.load_dump(&text, &LoadOptions { host_fn: &host_fn, ..LoadOptions::default() });
        let l = match loaded {
            Ok(l) => l,
            Err(e) => {
                let m = e.message();
                let m = m.split('\n').next().unwrap_or("").to_string();
                *refused.entry(m).or_default() += 1;
                continue;
            }
        };
        r.api = l.handles.iter().find(|h| h.0 == "api").map(|h| h.1);
        r.done = prefix.split("\n\u{1e}\n").any(|l| l.starts_with("=== "));
        let mut out = Vec::new();
        let res = (|| -> Result<(), String> {
            // The load left the VM idle: settle the oldest call and go on.
            let Some(&id) = r.vm.pending_host_calls().iter().min() else { return Ok(()) };
            let info = r.vm.host_call(id).ok_or("no call")?;
            let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
            let rejected = info.args.get(1) == Some(&Value::TRUE);
            r.vm.resolve_host_call(id, if rejected { Err(v) } else { Ok(v) }).map_err(|e| format!("{e:?}"))?;
            r.drive(&mut |_| {})?;
            if !r.done {
                r.api_call("stalled", &[], &mut |_| {})?;
            }
            Ok(())
        })();
        if let Err(e) = res {
            differ.push(format!("{stem}: {e}"));
            continue;
        }
        out.extend(clean(r.out));
        let got = if prefix.is_empty() { sep(&out) } else if out.is_empty() { prefix.clone() } else { format!("{prefix}\n\u{1e}\n{}", sep(&out)) };
        if got == full {
            same += 1;
        } else {
            differ.push(stem);
        }
    }
    println!(
        "import: {} dumps, {same} finished the same, {} differ, refused at the load: {refused:?}",
        files.len(),
        differ.len()
    );
    for d in differ.iter().take(20) {
        println!("  differs: {d}");
    }
    if !differ.is_empty() || !refused.is_empty() {
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("export") if args.len() == 4 => export(Path::new(&args[2]), Path::new(&args[3])),
        Some("import") if args.len() == 3 => import(Path::new(&args[2])),
        _ => {
            eprintln!("usage: dumpx export CORPUS OUT | dumpx import OUT");
            std::process::exit(2);
        }
    }
}
