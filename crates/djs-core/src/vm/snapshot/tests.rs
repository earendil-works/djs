//! Logical snapshots on the harvested codemode corpus (0300): every case of
//! tests/corpus/codemode-harvest runs as the difftest runs it, and
//! at idle points the VM is exported, imported into a fresh VM, and the run goes on there.
//!
//! `DJS_SNAP=1 cargo test -p djs-core --features snapshot --lib vm::snapshot -- --ignored --nocapture`
//! (`DJS_SNAP_OUT=file`: one line per idle point; `DJS_SNAP_DIR=dir`: the snapshots
//! themselves, for compressed sizes).

extern crate std;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::time::Instant;

use super::*;
use crate::compiler::CompileOptions;
use crate::heap::Handle;
use crate::interp::Status;
use crate::value::{Obj, Value};

const OUTPUT: u32 = 0;
const DONE: u32 = 1;
const LATER: u32 = 2;

/// The base image of this build, made once.
pub(super) fn base() -> &'static [u8] {
    static BASE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let mut v = Vm::new(VmOptions::default()).unwrap();
        v.prepare_base_image().unwrap();
        v.bytes().to_vec()
    })
}

/// The VMs' options: frozen builtins, as pi runs; with `DJS_LAZY=1` lazy functions
/// (D2990).
pub(super) fn vm_opts() -> VmOptions {
    VmOptions {
        frozen_builtins: true,
        #[cfg(feature = "lazy")]
        lazy_functions: std::env::var("DJS_LAZY").is_ok_and(|v| v == "1"),
        ..VmOptions::default()
    }
}

pub(super) fn info() -> &'static BaseInfo {
    static INFO: std::sync::OnceLock<BaseInfo> = std::sync::OnceLock::new();
    INFO.get_or_init(|| BaseInfo::new(base(), vm_opts()).unwrap())
}

fn get(vm: &mut Vm, o: Obj, name: &str) -> Value {
    let k = vm.heap_mut().atom_str(name).unwrap();
    vm.heap()
        .find_property(o, k)
        .and_then(|(_, p)| p.value())
        .unwrap_or(Value::UNDEFINED)
}

/// What happens at an idle point: `k` counts them from 0 over the whole run.
type AtIdle<'a> = &'a mut dyn FnMut(&mut Vm, Handle, usize);

/// Runs `f` as the difftest's djs engine does: output and
/// done recorded, `later` calls settled at idle points, oldest first.
#[allow(clippy::too_many_arguments)]
fn idle_call(
    vm: &mut Vm,
    api: Handle,
    f: Value,
    this: Value,
    args: &[Value],
    out: &mut Vec<String>,
    mut at_idle: Option<AtIdle<'_>>,
    k: &mut usize,
) -> Result<Value, String> {
    vm.call(f, this, args).map_err(|e| format!("{e:?}"))?;
    let mut slices = 0;
    loop {
        match vm.run(10_000_000) {
            Status::Done(v) => return Ok(v),
            Status::Threw(e) => return Err(format!("threw {}", vm.display(e))),
            Status::OutOfFuel => {
                slices += 1;
                if slices > 100 {
                    return Err("out of fuel".into());
                }
            }
            Status::HostCall { call_id, fn_id } => {
                let info = vm.host_call(call_id).ok_or("no call")?;
                let s = |v: Value| {
                    v.as_string()
                        .map(|t| vm.heap().str_to_string(t))
                        .unwrap_or_default()
                };
                match fn_id {
                    OUTPUT => out.push(s(info.args[0])),
                    DONE => out.push(format!("=== {} {}", s(info.args[0]), s(info.args[1]))),
                    _ => return Err("unknown host function".into()),
                }
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED))
                    .map_err(|e| format!("{e:?}"))?;
            }
            Status::Idle => {
                if let Some(h) = at_idle.as_deref_mut() {
                    h(vm, api, *k);
                }
                *k += 1;
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    return Err("idle with nothing pending".into());
                };
                let info = vm.host_call(id).ok_or("no call")?;
                let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
                let rejected = info.args.get(1) == Some(&Value::TRUE);
                vm.resolve_host_call(id, if rejected { Err(v) } else { Ok(v) })
                    .map_err(|e| format!("{e:?}"))?;
            }
            s => return Err(format!("{s:?}")),
        }
    }
}

/// A case's transcript; `at_idle` is called at each idle point of the script's run.
fn run(prelude: &str, case: &str, at_idle: Option<AtIdle<'_>>) -> Result<Vec<String>, String> {
    let mut vm = Vm::from_base(base(), vm_opts()).unwrap();
    let mut out = Vec::new();
    let mut k = 0;
    let p = vm
        .compile(prelude.as_bytes(), &CompileOptions::script("prelude.js"))
        .map_err(|e| e.message)?;
    let g = Value::object(vm.global());
    let none = vm.heap_mut().new_handle(Value::UNDEFINED).unwrap();
    let setup = idle_call(&mut vm, none, p, g, &[], &mut out, None, &mut k)?;
    let host = vm.new_object().map_err(|e| format!("{e:?}"))?;
    for (id, name) in [(OUTPUT, "output"), (DONE, "done"), (LATER, "later")] {
        let f = if id == LATER {
            vm.new_async_host_function(id, name)
        } else {
            vm.new_host_function(id, name)
        }
        .map_err(|e| format!("{e:?}"))?;
        vm.define(host, name, f).map_err(|e| format!("{e:?}"))?;
    }
    let apiv = idle_call(
        &mut vm,
        none,
        setup,
        Value::UNDEFINED,
        &[Value::object(host)],
        &mut out,
        None,
        &mut k,
    )?;
    let api = vm
        .heap_mut()
        .new_handle(apiv)
        .map_err(|e| format!("{e:?}"))?;
    let src = format!("(async (tools, console) => {{{case}\n}})");
    let script = vm
        .compile(src.as_bytes(), &CompileOptions::script("case.js"))
        .map_err(|e| e.message)?;
    let g = Value::object(vm.global());
    let f = idle_call(&mut vm, api, script, g, &[], &mut out, None, &mut k)?;
    let apiv = vm.heap().get_handle(api);
    let runf = get(&mut vm, apiv.as_object().unwrap(), "run");
    k = 0;
    idle_call(&mut vm, api, runf, apiv, &[f], &mut out, at_idle, &mut k)?;
    let apiv = vm.heap().get_handle(api);
    let stalled = get(&mut vm, apiv.as_object().unwrap(), "stalled");
    idle_call(&mut vm, api, stalled, apiv, &[], &mut out, None, &mut k)?;
    Ok(out
        .into_iter()
        .map(|o| {
            o.lines()
                .filter(|l| !l.trim_start().starts_with("at "))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect())
}

/// Exports and imports `vm` in place.
fn roundtrip(vm: &mut Vm) -> Result<usize, String> {
    let bytes = vm
        .snapshot(info())
        .map_err(|e| format!("export: {}", e.0.join(" | ")))?;
    let n = bytes.len();
    *vm = Vm::from_snapshot(info(), &bytes).map_err(|e| format!("import: {}", e.0.join(" | ")))?;
    Ok(n)
}

fn corpus() -> (String, Vec<(String, String)>) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let prelude = std::fs::read_to_string(root.join("prelude.js")).unwrap();
    let dir = std::env::var_os("DJS_SNAP_CASES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("codemode-harvest"));
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "js"))
        .collect();
    files.sort();
    let filter = std::env::var("DJS_SNAP_FILTER").unwrap_or_default();
    let cases = files
        .into_iter()
        .filter(|f| f.to_string_lossy().contains(&filter))
        .map(|f| {
            (
                f.file_stem().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&f).unwrap(),
            )
        })
        .collect();
    (prelude, cases)
}

/// The first reason, without the record's position: "Object.Foo: why".
fn reason(e: &str) -> String {
    let e = e.split(" | ").next().unwrap_or(e);
    let e = match e.split_once("record ") {
        Some((a, b)) => format!("{a}record {}", b.split_once(' ').map_or(b, |x| x.1)),
        None => e.to_string(),
    };
    e.chars().take(140).collect()
}

#[derive(Default)]
struct Stats {
    cases: usize,
    with_idle: usize,
    idle: usize,
    exported: usize,
    imported: usize,
    single_same: usize,
    single_differ: usize,
    chain_same: usize,
    chain_differ: usize,
    chain_failed: usize,
    refusals: BTreeMap<String, usize>,
    import_refusals: BTreeMap<String, usize>,
    differ: Vec<String>,
}

#[test]
#[ignore]
fn snapshot_harvest() {
    if std::env::var_os("DJS_SNAP").is_none() {
        return;
    }
    let (prelude, cases) = corpus();
    let out_path = std::env::var_os("DJS_SNAP_OUT");
    let dir = std::env::var_os("DJS_SNAP_DIR").map(std::path::PathBuf::from);
    if let Some(d) = &dir {
        std::fs::create_dir_all(d).unwrap();
    }
    let measure = std::env::var_os("DJS_SNAP_MEASURE").is_some();
    let mut lines = String::from(
        "case\tk\tsnap\texport_us\timport_us\timage\tsources\trecords\tnosrc\tfresh_us\tcompile_us\n",
    );
    let mut st = Stats::default();
    for (name, case) in &cases {
        st.cases += 1;
        let Ok(want) = run(&prelude, case, None) else {
            continue;
        };
        // Every idle point: export, and, measured, the image beside it.
        let mut points = 0;
        let mut first_err: Vec<(usize, Result<(), String>)> = Vec::new();
        let r = run(
            &prelude,
            case,
            Some(&mut |vm: &mut Vm, _api: Handle, k: usize| {
                points += 1;
                let t0 = Instant::now();
                let snap = vm.snapshot(info());
                let export_us = t0.elapsed().as_micros();
                let snap = match snap {
                    Ok(s) => s,
                    Err(e) => {
                        first_err.push((k, Err(format!("export: {}", e.0.join(" | ")))));
                        return;
                    }
                };
                let t0 = Instant::now();
                let loaded = Vm::from_snapshot_stats(info(), &snap);
                let import_us = t0.elapsed().as_micros();
                let (records, ok) = match &loaded {
                    Ok((_, s)) => (s.records, Ok(())),
                    Err(e) => (0, Err(format!("import: {}", e.0.join(" | ")))),
                };
                first_err.push((k, ok));
                if measure || dir.is_some() {
                    let mut img = vm.clone_for_image();
                    let image = img.image().len();
                    let sources = sources_len(&snap);
                    let nosrc = vm
                        .snapshot_with(
                            info(),
                            &SnapshotOptions {
                                sources: false,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    let wrapped = format!("(async (tools, console) => {{{case}\n}})");
                    Vm::from_snapshot_with(
                        info(),
                        &nosrc,
                        &[prelude.as_bytes(), wrapped.as_bytes()],
                    )
                    .unwrap();
                    // What an import costs before its records: a fresh VM, then the
                    // scripts compiled into it.
                    let t0 = Instant::now();
                    let mut fresh = Vm::from_base(base(), vm_opts()).unwrap();
                    let fresh_us = t0.elapsed().as_micros();
                    let t0 = Instant::now();
                    fresh
                        .compile(prelude.as_bytes(), &CompileOptions::script("prelude.js"))
                        .unwrap();
                    fresh
                        .compile(wrapped.as_bytes(), &CompileOptions::script("case.js"))
                        .unwrap();
                    let compile_us = t0.elapsed().as_micros();
                    lines += &format!(
                        "{name}\t{k}\t{}\t{export_us}\t{import_us}\t{image}\t{sources}\t{records}\t{}\t{fresh_us}\t{compile_us}\n",
                        snap.len(),
                        nosrc.len(),
                    );
                    if let Some(d) = &dir {
                        std::fs::write(d.join(format!("{name}-{k}.snap")), &snap).unwrap();
                        std::fs::write(d.join(format!("{name}-{k}.nosrc")), &nosrc).unwrap();
                    }
                }
            }),
        );
        if r.as_ref() != Ok(&want) {
            st.differ
                .push(format!("{name}: the run with exports differs"));
        }
        if points == 0 {
            continue;
        }
        st.with_idle += 1;
        st.idle += points;
        for (_, e) in &first_err {
            match e {
                Ok(()) => {
                    st.exported += 1;
                    st.imported += 1;
                }
                Err(m) if m.starts_with("export") => {
                    *st.refusals.entry(reason(m)).or_default() += 1;
                }
                Err(m) => {
                    st.exported += 1;
                    *st.import_refusals.entry(reason(m)).or_default() += 1;
                }
            }
        }
        // Each idle point alone: continue in the imported VM.
        for (k, e) in &first_err {
            if e.is_err() {
                continue;
            }
            let got = run(
                &prelude,
                case,
                Some(&mut |vm: &mut Vm, _: Handle, j: usize| {
                    if j == *k {
                        roundtrip(vm).unwrap();
                    }
                }),
            );
            if got.as_ref() == Ok(&want) {
                st.single_same += 1;
            } else {
                st.single_differ += 1;
                st.differ
                    .push(format!("{name}@{k}: {:?}", diff(&got, &want)));
            }
        }
        // A chain: through a snapshot at every idle point.
        let mut failed = None;
        let got = run(
            &prelude,
            case,
            Some(&mut |vm: &mut Vm, _: Handle, k: usize| {
                if failed.is_none()
                    && let Err(e) = roundtrip(vm)
                {
                    failed = Some(format!("{k}: {e}"));
                }
            }),
        );
        if failed.is_some() {
            st.chain_failed += 1;
        } else if got.as_ref() == Ok(&want) {
            st.chain_same += 1;
        } else {
            st.chain_differ += 1;
            st.differ
                .push(format!("{name} (chain): {:?}", diff(&got, &want)));
        }
    }
    std::println!(
        "cases: {}, with idle points: {}, idle points: {}, exported: {}, imported: {}",
        st.cases,
        st.with_idle,
        st.idle,
        st.exported,
        st.imported
    );
    std::println!(
        "single idle point, transcript as uninterrupted: {} same, {} differ",
        st.single_same,
        st.single_differ
    );
    std::println!(
        "chains (every idle point): {} same, {} differ, {} with a refusal",
        st.chain_same,
        st.chain_differ,
        st.chain_failed
    );
    std::println!("export refusals (idle points, first reason):");
    for (r, n) in &st.refusals {
        std::println!("  {n:5}  {r}");
    }
    std::println!("import refusals:");
    for (r, n) in &st.import_refusals {
        std::println!("  {n:5}  {r}");
    }
    for d in st.differ.iter().take(30) {
        std::println!("differs: {d}");
    }
    if let Some(p) = out_path {
        std::fs::write(p, lines).unwrap();
    }
    // The gate's contract (D3703): every idle point of the corpus exports, imports and
    // continues like the uninterrupted run, alone and chained.
    assert!(
        st.refusals.is_empty() && st.import_refusals.is_empty(),
        "snapshots refused"
    );
    assert!(st.differ.is_empty(), "runs differ after a snapshot");
}

fn diff(got: &Result<Vec<String>, String>, want: &[String]) -> String {
    match got {
        Err(e) => e.chars().take(200).collect(),
        Ok(g) => {
            let i = g
                .iter()
                .zip(want)
                .position(|(a, b)| a != b)
                .unwrap_or(g.len().min(want.len()));
            format!(
                "line {i}: got {:?}, want {:?}",
                g.get(i).map(|s| s.chars().take(120).collect::<String>()),
                want.get(i).map(|s| s.chars().take(120).collect::<String>())
            )
        }
    }
}

/// The bytes of the scripts' sources in a snapshot (what a host that keeps the sources
/// itself would not need stored).
fn sources_len(snap: &[u8]) -> usize {
    super::import::sources_len(snap).unwrap_or(0)
}

impl Vm {
    /// A copy of this VM (for its image, without disturbing the run).
    fn clone_for_image(&self) -> Vm {
        Vm::from_bytes(self.bytes().to_vec(), self.opts).unwrap()
    }
}

#[test]
fn a_small_run_round_trips() {
    let prelude = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/prelude.js"),
    )
    .unwrap();
    let case = r#"
const m = new Map([["a", 1]]);
const s = new Set([1, 2]);
const re = /a(b+)c/g;
let total = 0;
for (const x of [1, 2, 3]) {
  const r = await later({ x, y: [x, x * 2] });
  total += r.x;
  m.set("k" + x, r.y);
  text(`${x}: ${JSON.stringify(r)} ${re.exec("xabbc") && re.lastIndex}`);
}
const all = await Promise.all([later(1), later(2), later("three")]);
text(JSON.stringify([...m], null, 0) + " " + s.size + " " + all.join(","));
class P { #x = 1; get x() { return this.#x; } }
const p = new P();
const tag = (s) => s;
const site = () => tag`a${1}b`;
const t1 = site();
await later(0);
text(String(p.x) + " " + Symbol.for("k").toString() + " " + Math.random() + " " + (t1 === site()));
return total;
"#;
    let want = run(&prelude, case, None).unwrap();
    let got = run(
        &prelude,
        case,
        Some(&mut |vm: &mut Vm, _: Handle, _k: usize| {
            roundtrip(vm).unwrap();
        }),
    )
    .unwrap();
    assert_eq!(got, want);
}

// ---- The snapshot test programs (tests/snapshot/programs) -----------------------------

const P_LATER: u32 = 1;
const P_PRINT: u32 = 2;

fn program_vm() -> Vm {
    let mut vm = Vm::from_base(base(), vm_opts()).unwrap();
    let later = vm.new_async_host_function(P_LATER, "later").unwrap();
    vm.define_global("later", later).unwrap();
    let print = vm.new_host_function(P_PRINT, "print").unwrap();
    vm.define_global("print", print).unwrap();
    vm
}

/// Runs program `src`, calling `at_idle(vm, k)` at idle point `k` (from 0): its lines.
fn run_program(src: &str, at_idle: &mut dyn FnMut(&mut Vm, usize)) -> Vec<String> {
    let mut vm = program_vm();
    let f = vm
        .compile(src.as_bytes(), &CompileOptions::script("src.js"))
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    let mut k = 0;
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, P_PRINT);
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                if vm.pending_host_calls().is_empty() {
                    break;
                }
                at_idle(&mut vm, k);
                k += 1;
                let id = *vm.pending_host_calls().iter().min().unwrap();
                let info = vm.host_call(id).unwrap();
                let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
                let rejected = info.args.get(1).copied() == Some(Value::TRUE);
                vm.resolve_host_call(id, if rejected { Err(v) } else { Ok(v) })
                    .unwrap();
            }
            Status::Threw(e) => {
                let m = vm.display(e);
                lines.push(format!("threw {m}"));
                break;
            }
            s => panic!("{s:?}"),
        }
    }
    lines
}

fn programs() -> Vec<(String, String)> {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshot/programs");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".src.js"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|f| {
            let name = f
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".src.js", "");
            (name, std::fs::read_to_string(&f).unwrap())
        })
        .collect()
}

/// Every idle point of the snapshot test programs, alone and chained.
#[test]
#[ignore]
fn snapshot_programs() {
    if std::env::var_os("DJS_SNAP").is_none() {
        return;
    }
    let (mut points, mut same, mut differ) = (0, 0, 0);
    let (mut chain_same, mut chain_differ, mut chain_refused) = (0, 0, 0);
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut notes = Vec::new();
    let dir = std::env::var_os("DJS_SNAP_DIR").map(std::path::PathBuf::from);
    for (name, src) in programs() {
        let want = run_program(&src, &mut |_, _| {});
        let mut n = 0;
        let mut errs = Vec::new();
        run_program(&src, &mut |vm, k| {
            n += 1;
            match vm.snapshot(info()) {
                Err(e) => errs.push((k, format!("export: {}", e.0.join(" | ")))),
                Ok(b) => {
                    if let Some(d) = &dir {
                        let f = format!("prog-{}-{k}.snap", name.replace('/', "_"));
                        std::fs::write(d.join(f), &b).unwrap();
                    }
                    if let Err(e) = Vm::from_snapshot(info(), &b) {
                        errs.push((k, format!("import: {}", e.0.join(" | "))));
                    }
                }
            }
        });
        points += n;
        for (k, e) in &errs {
            *refusals.entry(reason(e)).or_default() += 1;
            notes.push(format!(
                "{name}@{k}: {}",
                e.chars().take(300).collect::<String>()
            ));
        }
        for k in 0..n {
            if errs.iter().any(|(j, _)| *j == k) {
                continue;
            }
            let got = run_program(&src, &mut |vm, j| {
                if j == k {
                    roundtrip(vm).unwrap();
                }
            });
            if got == want {
                same += 1;
            } else {
                differ += 1;
                notes.push(format!("{name}@{k} differs: {:?}", diff(&Ok(got), &want)));
            }
        }
        if n == 0 {
            continue;
        }
        let mut refused = false;
        let got = run_program(&src, &mut |vm, _| {
            if !refused && roundtrip(vm).is_err() {
                refused = true;
            }
        });
        if refused {
            chain_refused += 1;
        } else if got == want {
            chain_same += 1;
        } else {
            chain_differ += 1;
            notes.push(format!(
                "{name} (chain) differs: {:?}",
                diff(&Ok(got), &want)
            ));
        }
    }
    std::println!(
        "programs: idle points {points}: {same} continue the same, {differ} differ, {} refused",
        refusals.values().sum::<usize>()
    );
    std::println!("chains: {chain_same} same, {chain_differ} differ, {chain_refused} refused");
    for (r, n) in &refusals {
        std::println!("  {n:5}  {r}");
    }
    for l in notes.iter().take(40) {
        std::println!("{l}");
    }
    assert!(
        notes.is_empty(),
        "snapshots of the test programs refused or differ"
    );
}

/// `DJS_SNAP_DESCRIBE=file cargo test ... describe_file -- --ignored --nocapture`
#[test]
#[ignore]
fn describe_file() {
    let Some(f) = std::env::var_os("DJS_SNAP_DESCRIBE") else {
        return;
    };
    let f = std::path::PathBuf::from(f);
    let files: Vec<std::path::PathBuf> = if f.is_dir() {
        let mut v: Vec<_> = std::fs::read_dir(&f)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "snap"))
            .collect();
        v.sort();
        v
    } else {
        alloc::vec![f]
    };
    for f in files {
        let b = std::fs::read(&f).unwrap();
        std::println!("== {}\n{}", f.display(), describe(&b).unwrap());
    }
}

// ---- Large live state --------------------------------------------------------------

const LARGE: &[(&str, &str)] = &[
    (
        "object-100k-strings",
        "const o = {}; for (let i = 0; i < 1000; i++) o['k' + i] = 'x'.repeat(96) + String(i).padStart(4, '0'); const use = () => Object.keys(o).length + o.k7;",
    ),
    (
        "array-1e6-numbers",
        "const a = []; for (let i = 0; i < 1e6; i++) a.push(i % 3 ? i : i * 1.5); const use = () => a.length + a[999999];",
    ),
    (
        "map-10k",
        "const m = new Map(); for (let i = 0; i < 10000; i++) m.set('key' + i, { i, s: 'v' + i }); const use = () => m.size + m.get('key77').s;",
    ),
    (
        "log-20k-lines",
        "const log = []; for (let i = 0; i < 20000; i++) log.push(`[${i}] request ${i * 7 % 1000} served in ${i % 97} ms`); const use = () => log.length + log[12345];",
    ),
];

fn percentile(xs: &mut [u128], p: f64) -> u128 {
    xs.sort();
    xs[((xs.len() as f64 - 1.0) * p).round() as usize]
}

/// `DJS_SNAP=1 cargo test --release ... large_state -- --ignored --nocapture`
#[test]
#[ignore]
fn large_state() {
    if std::env::var_os("DJS_SNAP").is_none() {
        return;
    }
    let dir = std::env::var_os("DJS_SNAP_DIR").map(std::path::PathBuf::from);
    let reps: usize = std::env::var("DJS_SNAP_REPS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(15);
    let _ = info();
    std::println!(
        "case\tsnap_bytes\texport_med_us\texport_p99_us\timport_med_us\timport_p99_us\timage_bytes\timage_med_us\trestore_med_us"
    );
    for (name, setup) in LARGE {
        let src = format!(
            "(async () => {{ {setup}\n const r = await later(1); print(typeof r, use());\n await later(2); print('done', use()); }})();"
        );
        let mut vm = program_vm();
        let f = vm
            .compile(src.as_bytes(), &CompileOptions::script("src.js"))
            .unwrap();
        let this = Value::object(vm.global());
        vm.call(f, this, &[]).unwrap();
        loop {
            match vm.run(u64::MAX / 2) {
                Status::Idle | Status::Done(_) => break,
                Status::OutOfFuel => {}
                s => panic!("{s:?}"),
            }
        }
        vm.collect().unwrap();
        let (mut ex, mut im, mut ig, mut rs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut snap = Vec::new();
        let mut image = 0;
        for _ in 0..reps {
            let t = Instant::now();
            snap = vm.snapshot(info()).unwrap();
            ex.push(t.elapsed().as_micros());
            let t = Instant::now();
            let v = Vm::from_snapshot(info(), &snap).unwrap();
            im.push(t.elapsed().as_micros());
            drop(v);
            let t = Instant::now();
            let bytes = vm.image().to_vec();
            ig.push(t.elapsed().as_micros());
            image = bytes.len();
            let t = Instant::now();
            let v = Vm::from_bytes(bytes, vm.opts).unwrap();
            rs.push(t.elapsed().as_micros());
            drop(v);
        }
        // The imported VM finishes like the original.
        let mut a = Vm::from_snapshot(info(), &snap).unwrap();
        let mut lines = Vec::new();
        for v in [&mut vm, &mut a] {
            let mut l = Vec::new();
            while let Some(&id) = v.pending_host_calls().iter().min() {
                v.resolve_host_call(id, Ok(Value::int(1))).unwrap();
                loop {
                    match v.run(u64::MAX / 2) {
                        Status::HostCall { call_id, .. } => {
                            let args = v.host_call(call_id).unwrap().args;
                            l.push(v.display(args[0]));
                            v.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
                        }
                        Status::Idle | Status::Done(_) => break,
                        s => panic!("{s:?}"),
                    }
                }
            }
            lines.push(l);
        }
        assert_eq!(lines[0], lines[1], "{name}");
        // The next idle point's snapshot, for a delta against this one.
        let mut next = None;
        {
            let mut v = Vm::from_snapshot(info(), &snap).unwrap();
            let id = *v.pending_host_calls().iter().min().unwrap();
            v.resolve_host_call(id, Ok(Value::int(1))).unwrap();
            loop {
                match v.run(u64::MAX / 2) {
                    Status::HostCall { call_id, .. } => {
                        v.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
                    }
                    Status::Idle | Status::Done(_) => break,
                    s => panic!("{s:?}"),
                }
            }
            if !v.pending_host_calls().is_empty() {
                next = Some(v.snapshot(info()).unwrap());
            }
        }
        if let Some(d) = &dir {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join(format!("large-{name}.snap")), &snap).unwrap();
            if let Some(n) = &next {
                std::fs::write(d.join(format!("large-{name}.next.snap")), n).unwrap();
            }
        }
        std::println!(
            "{name}\t{}\t{}\t{}\t{}\t{}\t{image}\t{}\t{}",
            snap.len(),
            percentile(&mut ex, 0.5),
            percentile(&mut ex, 0.99),
            percentile(&mut im, 0.5),
            percentile(&mut im, 0.99),
            percentile(&mut ig, 0.5),
            percentile(&mut rs, 0.5),
        );
    }
}

/// Snapshots another build wrote (`DJS_SNAP_IMPORT=dir`, the files of `DJS_SNAP_DIR`):
/// each is loaded at its idle point of this build's run, which goes on from it; the
/// transcript must be this build's uninterrupted one.
#[test]
#[ignore]
fn snapshot_import_dir() {
    let Some(dir) = std::env::var_os("DJS_SNAP_IMPORT").map(std::path::PathBuf::from) else {
        return;
    };
    let (prelude, cases) = corpus();
    let (mut tried, mut same, mut differ, mut unreached) = (0, 0, 0, 0);
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut notes = Vec::new();
    let mut check = |name: &str,
                     want: &[String],
                     got: Result<Vec<String>, String>,
                     err: Option<Option<String>>| {
        tried += 1;
        match err {
            // This build's run never got to that idle point.
            None => unreached += 1,
            Some(Some(e)) => {
                *refused.entry(reason(&e)).or_default() += 1;
                if notes.len() < 20 {
                    notes.push(format!(
                        "{name}: {}",
                        e.chars().take(300).collect::<String>()
                    ));
                }
            }
            Some(None) if got.as_deref() == Ok(want) => same += 1,
            Some(None) => {
                differ += 1;
                notes.push(format!("{name} differs: {}", diff(&got, want)));
            }
        }
    };
    for (name, case) in &cases {
        let Ok(want) = run(&prelude, case, None) else {
            continue;
        };
        for k in 0.. {
            let Ok(bytes) = std::fs::read(dir.join(format!("{name}-{k}.snap"))) else {
                break;
            };
            let mut err = None;
            let got = run(
                &prelude,
                case,
                Some(&mut |vm: &mut Vm, _: Handle, j: usize| {
                    if j == k {
                        match Vm::from_snapshot(info(), &bytes) {
                            Ok(v) => {
                                *vm = v;
                                err = Some(None);
                            }
                            Err(e) => err = Some(Some(e.0.join(" | "))),
                        }
                    }
                }),
            );
            check(&format!("{name}@{k}"), &want, got, err);
        }
    }
    for (name, src) in programs() {
        let want = run_program(&src, &mut |_, _| {});
        for k in 0.. {
            let f = format!("prog-{}-{k}.snap", name.replace('/', "_"));
            let Ok(bytes) = std::fs::read(dir.join(f)) else {
                break;
            };
            let mut err = None;
            let got = run_program(&src, &mut |vm, j| {
                if j == k {
                    match Vm::from_snapshot(info(), &bytes) {
                        Ok(v) => {
                            *vm = v;
                            err = Some(None);
                        }
                        Err(e) => err = Some(Some(e.0.join(" | "))),
                    }
                }
            });
            check(&format!("{name}@{k}"), &want, Ok(got), err);
        }
    }
    std::println!(
        "imported from {}: {tried} snapshots, {same} continue as this build's run, {differ} differ, {} refused, {unreached} at idle points this build's run does not reach",
        dir.display(),
        refused.values().sum::<usize>()
    );
    for (r, n) in &refused {
        std::println!("  {n:5}  {r}");
    }
    for l in &notes {
        std::println!("{l}");
    }
}

/// What a build's internals look like, for the portability experiment.
#[test]
#[ignore]
fn probe_build() {
    let i = info();
    let h = Vm::from_base(base(), vm_opts()).unwrap();
    let h = h.heap();
    let id = |n: &str| crate::bytecode::intrinsics::names().position(|x| x == n);
    std::println!(
        "build {:08x} layout {:08x} base_end {} base bytes {} | intrinsic ids: AsyncAwait {:?} PromiseResolveFunction {:?} | atom ids: length {} then {} | BoxCell.VALUE @{} HostCall.ID @{} HostCall.FN @{} Interp.NEXT_CALL_ID @{} | JSON.parse at {:?}, Promise.prototype.then at {:?}",
        super::super::BUILD_HASH,
        crate::heap::cells::LAYOUT_HASH,
        h.base_end(),
        base().len(),
        id("AsyncAwait"),
        id("PromiseResolveFunction"),
        crate::heap::atoms::LENGTH.id(),
        crate::heap::atoms::PROTOTYPE.id(),
        crate::heap::cells::BoxCell::VALUE.offset,
        crate::heap::cells::HostCall::ID.offset,
        crate::heap::cells::HostCall::FN.offset,
        crate::heap::cells::Interp::NEXT_CALL_ID.offset,
        i.cells.get("JSON.parse"),
        i.cells.get("Promise.prototype.then"),
    );
}

/// Classes the corpus does not hold at idle points, round-tripped at every idle point.
#[test]
fn more_classes_round_trip() {
    let prelude = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/prelude.js"),
    )
    .unwrap();
    let case = r#"
const buf = new ArrayBuffer(16);
const u8 = new Uint8Array(buf, 2, 8);
u8.set([1, 2, 3, 250]);
const dv = new DataView(buf);
dv.setFloat64(8, Math.PI);
const f64 = new Float64Array([1.5, -0, NaN]);
const wm = new WeakMap();
const key = {};
wm.set(key, "weak");
const ws = new WeakSet([key]);
const d = new Date(86400000 * 365);
const big = 12345678901234567890123n * -3n;
const sym = Symbol("local");
const o = { [sym]: 1, get g() { return this.v * 2; }, set g(x) { this.v = x; }, v: 4 };
Object.defineProperty(o, "hidden", { value: 7, enumerable: false });
const sparse = [1, , 3];
sparse[100] = "far";
const frozen = Object.freeze({ a: [1, 2] });
const err = new TypeError("boom", { cause: 1 });
const px = new Proxy({ t: 1 }, { get: (t, k) => (k in t ? t[k] : `no ${String(k)}`) });
function* gen() { let i = 0; while (true) yield i++; }
const g = gen();
g.next();
async function* agen() { yield 1; yield await later(2); yield 3; }
const ag = agen();
const first = await ag.next();
const bound = function (a, b) { return this.k + a + b; }.bind({ k: 10 }, 1);
const it = new Map([[1, "a"], [2, "b"]]).entries();
it.next();
const strIt = "héllo"[Symbol.iterator]();
strIt.next();
const re = /(?<w>\w+)\s/gy;
re.lastIndex = 0;
re.exec("ab cd ");
const counter = (() => { let n = 0; return () => ++n; })();
counter();
const second = await ag.next();
text(JSON.stringify([...u8], null, 0) + " " + dv.getFloat64(8) + " " + [...f64].map(String).join("/"));
text([wm.get(key), ws.has(key), d.toISOString(), String(big), o[sym], o.g, (o.g = 5, o.g), o.hidden].join(" "));
text([sparse.length, 1 in sparse, sparse[100], Object.isFrozen(frozen), frozen.a.length, err.name, err.message, err.cause].join(" "));
text([px.t, px.zz, g.next().value, first.value, second.value, (await ag.next()).value, bound(2), it.next().value.join(":"), strIt.next().value, re.lastIndex, counter()].join(" "));
text(String(re.exec("ab cd ")?.groups.w) + " " + Object.getOwnPropertySymbols(o).length);
"#;
    let want = run(&prelude, case, None).unwrap();
    assert!(!want.iter().any(|l| l.contains("throw")), "{want:?}");
    let mut n = 0;
    let got = run(
        &prelude,
        case,
        Some(&mut |vm: &mut Vm, _: Handle, _k: usize| {
            let b = vm.snapshot(info()).unwrap();
            let d = describe(&b).unwrap();
            for c in [
                "Object.TypedArray",
                "Object.WeakMap",
                "BigInt",
                "Object.Proxy",
                "Object.Generator",
            ] {
                assert!(d.contains(c), "{c} not in the snapshot");
            }
            *vm = Vm::from_snapshot(info(), &b).unwrap();
            n += 1;
        }),
    )
    .unwrap();
    assert!(n > 0);
    assert_eq!(got, want);
}

// ---- Malformed snapshots refuse (D3702) -------------------------------------------------

/// Snapshots at every idle point of a few programs, for the fuzz.
fn fuzz_seeds() -> Vec<Vec<u8>> {
    let mut seeds = Vec::new();
    for (name, src) in programs() {
        if ![
            "plan",
            "generator",
            "asyncgen",
            "combinators",
            "golden/class-fields",
            "golden/using",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        run_program(&src, &mut |vm, _| seeds.push(vm.snapshot(info()).unwrap()));
    }
    seeds
}

/// Loads `bytes`; if that works, runs the VM to the end, answering its calls: a panic
/// anywhere is the failure.
fn load_and_run(bytes: &[u8]) -> bool {
    let Ok(mut vm) = Vm::from_snapshot(info(), bytes) else {
        return false;
    };
    for _ in 0..50 {
        match vm.run(2_000_000) {
            Status::HostCall { call_id, .. } => {
                let _ = vm.resolve_host_call(call_id, Ok(Value::UNDEFINED));
            }
            Status::Idle | Status::Done(_) => {
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    break;
                };
                let _ = vm.resolve_host_call(id, Ok(Value::int(1)));
            }
            _ => break,
        }
    }
    true
}

/// A small xorshift, so that the fuzz is the same every run.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn fuzz(rounds: usize) -> (usize, usize) {
    let seeds = fuzz_seeds();
    assert!(!seeds.is_empty());
    hook();
    let mut rng = Rng(0x0300_0301);
    let (mut loaded, mut tried) = (0, 0);
    for round in 0..rounds {
        let seed = &seeds[round % seeds.len()];
        let mut b = seed.clone();
        match rng.next() % 4 {
            0 => b.truncate((rng.next() as usize) % b.len()),
            1 => {
                let i = (rng.next() as usize) % b.len();
                b[i] ^= 1 << (rng.next() % 8);
            }
            2 => {
                for _ in 0..4 {
                    let i = (rng.next() as usize) % b.len();
                    b[i] = rng.next() as u8;
                }
            }
            _ => {
                // A byte in the records or the `Vm` record, where the structure is.
                let start = b.len() / 2;
                let i = start + (rng.next() as usize) % (b.len() - start);
                b[i] ^= 1 << (rng.next() % 8);
            }
        }
        tried += 1;
        let r = std::panic::catch_unwind(|| load_and_run(&b));
        match r {
            Ok(true) => loaded += 1,
            Ok(false) => {}
            Err(_) if std::env::var_os("DJS_SNAP_FUZZ_ALL").is_some() => {
                *PANICS
                    .lock()
                    .unwrap()
                    .entry(LAST.lock().unwrap().clone())
                    .or_default() += 1;
            }
            Err(_) => panic!("round {round}: a malformed snapshot panicked"),
        }
    }
    for (at, n) in PANICS.lock().unwrap().iter() {
        std::println!("{n:6} panics at {at}");
    }
    (tried, loaded)
}

static PANICS: std::sync::Mutex<BTreeMap<String, usize>> = std::sync::Mutex::new(BTreeMap::new());
static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// With `DJS_SNAP_FUZZ_ALL`: remember where each panic came from (the first djs frame).
fn hook() {
    if std::env::var_os("DJS_SNAP_FUZZ_ALL").is_none() {
        return;
    }
    std::panic::set_hook(alloc::boxed::Box::new(|_| {
        let bt = std::backtrace::Backtrace::force_capture().to_string();
        let at = bt
            .lines()
            .filter(|l| {
                l.contains("djs_core::")
                    && !l.contains("fail::")
                    && !l.contains("object_slot")
                    && !l.contains("snapshot::tests")
            })
            .take(9)
            .map(|l| l.trim().split("::").last().unwrap_or("?").to_string())
            .collect::<Vec<_>>()
            .join(" < ");
        *LAST.lock().unwrap() = at;
    }));
}

/// Truncations and bit flips refuse or load; they never panic.
#[test]
fn malformed_snapshots_refuse() {
    let (tried, loaded) = fuzz(400);
    std::println!("fuzz: {tried} malformed snapshots, {loaded} loaded and ran");
}

/// `DJS_SNAP_FUZZ=N`: a longer fuzz.
#[test]
#[ignore]
fn malformed_snapshots_refuse_long() {
    let n = std::env::var("DJS_SNAP_FUZZ")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(20_000);
    let (tried, loaded) = fuzz(n);
    std::println!("fuzz: {tried} malformed snapshots, {loaded} loaded and ran");
}

/// The negative control (D3703): an importer that loses dense elements must make the
/// corpus runs differ, or the harvest's comparison sees nothing.
#[test]
fn negative_control_catches_a_broken_import() {
    let (prelude, cases) = corpus();
    super::import::BROKEN.with(|b| b.set(true));
    let mut caught = 0;
    for (_, case) in cases.iter().take(12) {
        let Ok(want) = run(&prelude, case, None) else {
            continue;
        };
        let got = run(
            &prelude,
            case,
            Some(&mut |vm: &mut Vm, _: Handle, _k: usize| {
                let _ = roundtrip(vm);
            }),
        );
        if got.as_ref() != Ok(&want) {
            caught += 1;
        }
    }
    super::import::BROKEN.with(|b| b.set(false));
    assert!(caught > 0, "a broken importer went unnoticed");
}

// ---- The pin of the corpus's waiting functions (D3704) ---------------------------------

fn pin_file() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshot/frame-bytecode.txt")
}

/// The bytecode hash (the snapshot's, hash.rs) of every corpus function that can hold a
/// frame (async functions, generators), as `script<TAB>start..end<TAB>hash`.
fn frame_pins() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for d in ["codemode", "codemode-harvest"] {
        files.extend(
            std::fs::read_dir(root.join(d))
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "js")),
        );
    }
    files.sort();
    let mut out = String::from(
        "# The normalized bytecode (vm/snapshot/hash.rs) of every function of the codemode corpus\n\
         # that can hold a frame at an idle point (D3704): a snapshot waiting in one loads only\n\
         # into a build that compiles it to the same bytecode. A change here is a decision:\n\
         # regenerate with DJS_UPDATE_FRAME_PIN=1 cargo test -p djs-core --lib frame_bytecode_pin\n",
    );
    let mut vm = program_vm();
    for f in files {
        let case = std::fs::read_to_string(&f).unwrap();
        let name = f.strip_prefix(&root).unwrap().to_string_lossy().to_string();
        let src = format!("(async (tools, console) => {{{case}\n}})");
        let Ok(v) = vm.compile(src.as_bytes(), &CompileOptions::script("case.js")) else {
            continue;
        };
        let h = vm.heap();
        let mut stack = alloc::vec![h.closure_template(v.as_object().unwrap())];
        let mut rows = Vec::new();
        while let Some(t) = stack.pop() {
            let flags = u32::from(
                h.arena()
                    .get::<u16>(t, crate::heap::cells::FuncTemplate::FLAGS),
            );
            use crate::bytecode::template::func_flags::{ASYNC, GENERATOR};
            let d = crate::bytecode::template::debug_info(h, t);
            if flags & (ASYNC | GENERATOR) != 0 {
                rows.push((d.src_start, d.src_end, super::hash::template_hash(h, t)));
            }
            for i in 0..crate::bytecode::template::child_count(h, t) {
                if let Some(c) = crate::bytecode::template::child(h, t, i).and_then(|r| r.get()) {
                    stack.push(c);
                }
            }
        }
        rows.sort();
        for (a, b, hash) in rows {
            out += &format!("{name}\t{a}..{b}\t{hash:016x}\n");
        }
    }
    out
}

/// The pin is current: a compiler change that changes a waiting function's bytecode shows
/// here (a snapshot waiting in it no longer loads into the new build).
#[test]
fn frame_bytecode_pin() {
    let got = frame_pins();
    if std::env::var_os("DJS_UPDATE_FRAME_PIN").is_some() {
        std::fs::create_dir_all(pin_file().parent().unwrap()).unwrap();
        std::fs::write(pin_file(), &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(pin_file()).unwrap_or_default();
    if got != want {
        let changed = got
            .lines()
            .filter(|l| !want.lines().any(|w| w == *l))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "the bytecode of waiting corpus functions changed (tests/snapshot/frame-bytecode.txt); \
             snapshots waiting in them will not load into this build. If that is intended, \
             regenerate with DJS_UPDATE_FRAME_PIN=1. New or changed:\n{changed}"
        );
    }
}

// ---- Records and constants by name (D3710, D3712) ---------------------------------------

/// State the corpus seldom holds at an idle point, all of it in named records: Promise.all
/// and allSettled elements, finally, an async generator's queue, iterator helpers of every
/// kind, a for-in over a proxy, `using` and `await using`, Array.fromAsync,
/// disposeAsync.
const RECORDS_CASE: &str = r#"
const all = Promise.all([later(1), later(2), 3]);
const settled = Promise.allSettled([later(4), Promise.reject(5)]);
const any = Promise.any([Promise.reject(6), later(7)]);
const fin = later(8).finally(() => text("finally"));
async function* agen() { yield 1; yield await later(2); yield 3; }
const ag = agen();
const q = [ag.next(), ag.next(), ag.next(), ag.return(9)];
function* nat() { let i = 0; while (true) yield i++; }
const helpers = [
  nat().map((x) => x * 2), nat().filter((x) => x % 2), nat().take(5), nat().drop(2),
  nat().flatMap((x) => [x, x]), Iterator.concat([1, 2], [3]), Iterator.from({ next() { return { done: false, value: 1 }; } }),
];
for (const h of helpers) h.next();
const inner = helpers[4];
inner.next();
const proxied = new Proxy({ a: 1, b: 2, c: 3 }, {});
const keys = [];
async function walk() { for (const k in Object.create(proxied)) { keys.push(k); await later(0); } }
const walking = walk();
async function uses() {
  using a = { [Symbol.dispose]() { text("dispose a"); } };
  await using b = { async [Symbol.asyncDispose]() { text("dispose b"); } };
  await later(1);
  return "used";
}
const using1 = uses();
const fromAsync = Array.fromAsync([later(1), later(2)], async (x) => x + (await later(10)));
const stack = new AsyncDisposableStack();
stack.defer(async () => { await later(0); text("deferred"); });
const disposing = stack.disposeAsync();
await later(0);
text(JSON.stringify(await all) + JSON.stringify(await settled) + (await any) + (await fin));
text(JSON.stringify(await Promise.all(q)));
text(JSON.stringify(helpers.map((h) => h.next().value)) + inner.next().value);
await walking;
text(keys.join(",") + (await using1) + JSON.stringify(await fromAsync));
await disposing;
"#;

fn records_case_with(skew: super::export::Skew) -> (Vec<String>, Vec<String>, Vec<String>) {
    let prelude = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/prelude.js"),
    )
    .unwrap();
    let want = run(&prelude, RECORDS_CASE, None).unwrap();
    let (mut descs, mut refusals) = (Vec::new(), Vec::new());
    let got = run(
        &prelude,
        RECORDS_CASE,
        Some(&mut |vm: &mut Vm, _: Handle, _k: usize| {
            super::export::SKEW.with(|s| s.set(skew));
            let b = vm.snapshot(info());
            super::export::SKEW.with(|s| s.set(super::export::Skew::None));
            let b = b.unwrap();
            descs.push(describe(&b).unwrap());
            match Vm::from_snapshot(info(), &b) {
                Ok(v) => *vm = v,
                Err(e) => refusals.push(e.0.join(" | ")),
            }
        }),
    )
    .unwrap();
    assert_eq!(got.len(), want.len());
    if refusals.is_empty() {
        assert_eq!(got, want);
    }
    assert!(!want.iter().any(|l| l.contains("throw")), "{want:?}");
    (descs, refusals, want)
}

/// Records are written by kind and named field, constants by name: a snapshot of a build
/// whose records list their fields in another order and whose constants list their
/// members in another order continues the same.
#[test]
fn records_and_constants_travel_by_name() {
    let (descs, refusals, _) = records_case_with(super::export::Skew::None);
    assert!(refusals.is_empty(), "{refusals:?}");
    let all = descs.join("\n");
    for t in [
        "Object.Record.ElementRecord",
        "Object.Record.RemainingCount",
        "Object.Record.FinallyData",
        "Object.Record.AsyncGeneratorRequest",
        "Object.IteratorHelper.CallbackHelper",
        "Object.IteratorHelper.LimitHelper",
        "Object.IteratorHelper.FlatMapHelper",
        "Object.IteratorHelper.ConcatHelper",
        "Object.IteratorHelper.IteratorHelper",
        "Object.Record.ForInState",
        "Object.Record.UsingRecord",
        "Object.Record.FromAsyncState",
        "Object.Record.AsyncDisposeState",
        " GeneratorStates",
        " HelperStates",
        " PromiseFlags",
    ] {
        assert!(all.contains(t), "{t} not in any snapshot");
    }
    let (_, refusals, _) = records_case_with(super::export::Skew::Reverse);
    assert!(refusals.is_empty(), "{refusals:?}");
}

/// A field or a constant this build does not have refuses the snapshot, naming the record
/// kind and the field.
#[test]
fn a_missing_or_renamed_field_refuses() {
    use super::export::Skew;
    let cases = [
        (
            Skew::RenameField("called"),
            "ElementRecord: no slot calledRenamed",
        ),
        (
            Skew::DropField("keys"),
            "ElementRecord: the snapshot has no field keys",
        ),
        (
            Skew::RenameMember("fulfilled"),
            "no PromiseStates member renamed",
        ),
        (
            Skew::RenameMember("suspendedYield"),
            "no HelperStates member renamed",
        ),
    ];
    for (skew, want) in cases {
        let (_, refusals, _) = records_case_with(skew);
        assert!(
            refusals.iter().any(|r| r.contains(want)),
            "{want} not among {refusals:?}"
        );
    }
}
