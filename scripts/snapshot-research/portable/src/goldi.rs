//! Issue 0142 (docs/goldilocks-snapshots.md): pi sessions on djs (pi's real prelude, the
//! snapshot-research cases, snaplab's fake tools) with different snapshot references.
//!
//! portable goldi-images OUT PRELUDE.js FACTORED.js CASE.js...
//!   writes, per case and start-data variant, the references and the collected image at
//!   every idle point, for zstd and `djs heap`:
//!   - `exact.img`: today's reference, the prelude image of these exact inputs;
//!   - `empty.img`: the same prelude run with no tools, globals or store;
//!   - `stage1.img`: the factored prelude before `configure` (independent of the inputs);
//!   - `base.img`: the engine's base image;
//!   - `today-K.img`: a session started from `exact.img` (as the adapter does today);
//!   - `layered-K.img`: a session started from `stage1.img` and configured per run.
//!   Also `times.txt`: start costs of each way.

use std::fmt::Write as _;
use std::time::Instant;

use djs_core::vm::Vm;

use crate::pi::{self, Inputs};
use crate::pitools::{T0, Tools, tools_json};

/// Realistic start data: tools with descriptions of a few hundred characters (pi lists
/// each tool's description in `ALL_TOOLS`), namespaced globals, a small store.
pub fn realistic_inputs() -> Inputs {
    let words = "read files search the repository run shell commands with a timeout edit a file by \
                 replacing an exact string write a whole file list a directory find files by glob \
                 pattern grep for a regular expression returns matching lines with their numbers";
    let mut tools = vec![];
    for (i, t) in crate::pitools::TOOLS.iter().enumerate() {
        let desc: String = words
            .split(' ')
            .cycle()
            .skip(i * 3)
            .take(40 + 5 * i)
            .collect::<Vec<_>>()
            .join(" ");
        tools.push(format!(
            r#"{{"name":"{t}","jsName":"{t}","description":"{desc}"}}"#
        ));
    }
    for t in [
        "mcp__issues__create",
        "mcp__issues__comment",
        "mcp__browser__open",
        "mcp__browser__shot",
    ] {
        let js = t.replace("__", "_");
        tools.push(format!(
            r#"{{"name":"{t}","jsName":"{js}","description":"{words}"}}"#
        ));
    }
    let globals = r#"[{"name":"searchTools","spread":true},{"name":"describeTool","spread":false},{"name":"models.getAvailableOfType","spread":false},{"name":"models.classify","spread":true},{"name":"subagent","spread":false},{"name":"fetchUrl","spread":false}]"#;
    let mut store = String::from("{");
    for k in 0..6 {
        if k > 0 {
            store.push(',');
        }
        let _ = write!(
            store,
            r#""key{k}":"{{\"ids\":[{}],\"note\":\"summary {k}\"}}""#,
            (0..40)
                .map(|i| (i * 7919 + k) % 10007)
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    store.push('}');
    Inputs {
        tools: format!("[{}]", tools.join(",")),
        globals: globals.into(),
        store,
    }
}

pub fn small_inputs() -> Inputs {
    Inputs::tools(&tools_json())
}

fn empty_inputs() -> Inputs {
    Inputs {
        tools: "[]".into(),
        globals: "[]".into(),
        store: "{}".into(),
    }
}

fn sealed(mut s: pi::Session) -> (pi::Session, Vec<u8>) {
    // Older trees (before D2041) have no seal: a collection is the closest.
    #[cfg(not(feature = "old"))]
    s.vm.seal().unwrap();
    #[cfg(feature = "old")]
    s.vm.collect().unwrap();
    let img = s.vm.bytes().to_vec();
    (s, img)
}

/// Runs `s` (configured) through `code`, calling `at_idle` with the collected image at every
/// idle point.
fn drive(mut s: pi::Session, code: &str, mut at_idle: impl FnMut(usize, &[u8])) -> pi::Session {
    s.start(code, 42);
    let mut tools = Tools {
        failed_page2: false,
    };
    let mut now = T0;
    let mut k = 0;
    while s.ev.done.is_none() && !s.ev.calls.is_empty() {
        let mut vm = Vm::from_bytes(s.vm.bytes().to_vec(), pi::opts()).unwrap();
        vm.collect().unwrap();
        at_idle(k, vm.bytes());
        k += 1;
        let call = s.ev.calls.remove(0);
        let (ok, payload) = tools.call(&call.name, call.args.as_deref().unwrap_or(""));
        now += 1500.0;
        s.deliver(call.id, ok, &payload, now);
    }
    s
}

pub fn images(args: &[String]) {
    let out = &args[0];
    let prelude = std::fs::read_to_string(&args[1]).unwrap();
    let factored = std::fs::read_to_string(&args[2]).unwrap();
    let mut times = String::new();
    for (vname, inputs) in [("small", small_inputs()), ("realistic", realistic_inputs())] {
        let dir = format!("{out}/{vname}");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{dir}/inputs.json"),
            format!("{}\n{}\n{}\n", inputs.tools, inputs.globals, inputs.store),
        )
        .unwrap();
        let _ = writeln!(
            times,
            "{vname}: tools JSON {} B, globals {} B, store {} B",
            inputs.tools.len(),
            inputs.globals.len(),
            inputs.store.len()
        );
        // The references.
        let t = Instant::now();
        let (exact_s, exact) = sealed(pi::Session::prepare_with(&prelude, &inputs, T0));
        let t_exact = t.elapsed();
        let (_, empty) = sealed(pi::Session::prepare_with(&prelude, &empty_inputs(), T0));
        let t = Instant::now();
        let (stage1_s, stage1) = sealed(pi::Session::stage1(&factored, true));
        let t_stage1 = t.elapsed();
        let base = djs_prelude::new_vm(pi::opts()).unwrap().bytes().to_vec();
        for (n, b) in [
            ("exact", &exact),
            ("empty", &empty),
            ("stage1", &stage1),
            ("base", &base),
        ] {
            std::fs::write(format!("{dir}/{n}.img"), b).unwrap();
        }
        // Start costs: restore a reference image (verified), plus `configure` when layered.
        let t = Instant::now();
        let n = 20;
        for _ in 0..n {
            let _ = Vm::from_bytes(exact.clone(), pi::opts()).unwrap();
        }
        let t_restore_exact = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n {
            let mut s = pi::Session::from_image(&stage1, &stage1_s);
            s.factory = stage1_s.factory;
            s.bridge = stage1_s.bridge;
            s.getter = stage1_s.getter;
            s.configure(&inputs, T0);
        }
        let t_layered = t.elapsed() / n;
        let _ = writeln!(
            times,
            "  computing the exact prelude image {t_exact:.2?}, the stage-1 image {t_stage1:.2?}; \
             a start: restore the exact image {t_restore_exact:.2?}, restore stage 1 + configure {t_layered:.2?}"
        );
        let _ = writeln!(
            times,
            "  images: exact {} B, empty {} B, stage1 {} B, base {} B",
            exact.len(),
            empty.len(),
            stage1.len(),
            base.len()
        );
        drop(exact_s);
        for case in &args[3..] {
            let name = std::path::Path::new(case)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let code = std::fs::read_to_string(case).unwrap();
            let today = pi::Session::from_image(&exact, &{
                // handles: as prepared
                pi::Session::prepare_with(&prelude, &inputs, T0)
            });
            let a = drive(today, &code, |k, img| {
                std::fs::write(format!("{dir}/{name}-today-{k}.img"), img).unwrap();
            });
            let mut layered = pi::Session::from_image(&stage1, &stage1_s);
            layered.factory = stage1_s.factory;
            layered.bridge = stage1_s.bridge;
            layered.getter = stage1_s.getter;
            layered.configure(&inputs, T0);
            let b = drive(layered, &code, |k, img| {
                std::fs::write(format!("{dir}/{name}-layered-{k}.img"), img).unwrap();
            });
            let same = a.ev.output == b.ev.output && a.ev.done == b.ev.done;
            let _ = writeln!(
                times,
                "  {name}: today and layered give the same output and result: {same}"
            );
        }
    }
    std::fs::write(format!("{out}/times.txt"), &times).unwrap();
    print!("{times}");
}

// ---------------------------------------------------------------- tier 2: by meaning

use std::collections::HashMap;

use djs_core::heap::cells::BoxCell;
use djs_core::value::Value;
use serde_json::{Value as J, json};

use crate::export::Exporter;
use crate::heapx::*;
use crate::import::{Importer, collect_templates};

/// The prelude state the snapshot carries by name: what pi's prelude would hand out
/// through a `state()`/`restore()` pair. Emulated here by the prelude's top-level binding
/// names; a field the prelude does not have is skipped (and defaults on import).
const SCALARS: &[&str] = &[
    "nextId",
    "finished",
    "outputChars",
    "outputItems",
    "storedChars",
];
const MAPS: &[&str] = &["stored", "writes"];

fn prelude_roots(s: &pi::Session) -> Vec<Value> {
    let h = s.vm.heap();
    let g = s.vm.global();
    let mut roots = vec![
        h.get_handle(s.api),
        h.get_handle(s.settle),
        h.get_handle(s.run),
        h.get_handle(s.stalled),
    ];
    for k in h.own_keys(g) {
        if let Some(p) = h.get_own_property(g, k).and_then(|p| p.value()) {
            roots.push(p);
        }
    }
    roots
}

fn user_source(code: &str) -> String {
    format!("{}{code}\n}})", pi::SCRIPT_PREFIX)
}

/// Exports a waiting pi session by meaning: the script's own state as a graph, builtins
/// and the prelude's objects by path, pending calls by id, the prelude's state by name.
pub fn export_pi(
    s: &mut pi::Session,
    code: &str,
    inputs: &Inputs,
    failed_page2: bool,
) -> Result<J, Vec<String>> {
    s.vm.collect().unwrap();
    let vm = &s.vm;
    let heap = vm.heap();
    let mut ex = Exporter::new(vm);
    // Builtins: what a fresh realm has at the same path (the Exporter's check); the rest
    // reachable from the global object is the prelude's.
    ex.prelude = prelude_names(heap, &ex.builtins);
    ex.prelude_script = Some("codemode-prelude.js".into());
    let bindings = captured_bindings(heap, &prelude_roots(s));
    let mut state = serde_json::Map::new();
    for name in SCALARS {
        if let Some(&b) = bindings.get(*name) {
            let v: Value = heap.arena().get(b, BoxCell::VALUE);
            state.insert((*name).into(), ex.value(v));
        }
    }
    for name in MAPS {
        if let Some(&b) = bindings.get(*name) {
            let m: Value = heap.arena().get(b, BoxCell::VALUE);
            let entries: Vec<J> = heap
                .collection_entries_list(m.as_object().unwrap())
                .into_iter()
                .map(|(k, v)| json!([ex.value(k), ex.value(v)]))
                .collect();
            state.insert((*name).into(), J::Array(entries));
        }
    }
    // Pending calls: the prelude's map id -> {resolve, reject}; each call's promise is the
    // resolve function's data (a PromiseResolveFunction builtin closure).
    let Some(&pb) = bindings.get("pending") else {
        return Err(vec!["the prelude has no `pending`".into()]);
    };
    let pm: Value = heap.arena().get(pb, BoxCell::VALUE);
    let mut pending = Vec::new();
    for (k, entry) in heap.collection_entries_list(pm.as_object().unwrap()) {
        let e = entry.as_object().unwrap();
        let resolve = heap
            .own_keys(e)
            .into_iter()
            .find(|k| heap.atom_name(*k) == "resolve")
            .and_then(|a| heap.get_own_property(e, a))
            .and_then(|p| p.value());
        let promise = resolve
            .and_then(|r| r.as_object())
            .filter(|r| heap.object_class(*r) == djs_core::heap::ObjectClass::BuiltinClosure)
            .map(|r| heap.object_slot(r, 1));
        let Some(promise) = promise else {
            return Err(vec![
                "a pending entry without a promise resolve function".into(),
            ]);
        };
        let call =
            s.ev.calls
                .iter()
                .find(|c| Some(c.id as f64) == k.as_number());
        let pv = ex.value(promise);
        pending.push(json!({"id": k.as_number(), "promise": pv,
            "name": call.map(|c| c.name.clone()), "args": call.and_then(|c| c.args.clone())}));
    }
    // The prelude interface the script was started against: the global names the prelude
    // defined. The rest of the script may use any of them, so an importing prelude must
    // still define them all (checked on import).
    let mut interface: Vec<String> = ex
        .prelude
        .values()
        .filter(|p| p.matches('/').count() == 1)
        .map(|p| p.trim_start_matches("globalThis/").to_string())
        .collect();
    interface.sort();
    ex.drain();
    if !ex.errors.is_empty() {
        let mut e = ex.errors.clone();
        e.sort();
        e.dedup();
        return Err(e);
    }
    // The engine that exported it (images no longer record it, D2265).
    let build = format!("{:08x}", djs_core::vm::BUILD_HASH);
    Ok(json!({
        "format": "djs-goldilocks-prototype/1",
        "exportedBy": build,
        "script": user_source(code),
        "inputs": {"tools": inputs.tools, "globals": inputs.globals, "store": inputs.store},
        "preludeState": state,
        "interface": interface,
        "pending": pending,
        "host": {"now": s.now, "output": s.ev.output, "issued": s.ev.issued, "failedPage2": failed_page2},
        "nodes": ex.into_nodes(),
    }))
}

/// Imports a by-meaning pi snapshot onto a fresh run of `prelude` (any version that keeps
/// the names) on this build.
pub fn import_pi(snap: &J, prelude: &str) -> Result<(pi::Session, bool), String> {
    let inp = &snap["inputs"];
    let inputs = Inputs {
        tools: inp["tools"].as_str().unwrap().into(),
        globals: inp["globals"].as_str().unwrap().into(),
        store: inp["store"].as_str().unwrap().into(),
    };
    let t0 = Instant::now();
    let mut s = pi::Session::prepare_with(prelude, &inputs, T0);
    let t_prelude = t0.elapsed();
    for name in snap["interface"].as_array().unwrap() {
        let path = format!("globalThis/{}", name.as_str().unwrap());
        if resolve_prelude(s.vm.heap_mut(), &path).is_none_or(|v| v.is_undefined()) {
            return Err(format!(
                "this prelude does not define {path}, which the session's did"
            ));
        }
    }
    // The script's code, compiled again; nothing of it runs.
    let src = snap["script"].as_str().unwrap();
    let f =
        s.vm.compile(
            src.as_bytes(),
            &djs_core::compiler::CompileOptions::async_body("codemode.js", &[]),
        )
        .map_err(|e| e.message)?;
    let mut templates = HashMap::new();
    let root = closure_template(s.vm.heap(), f.as_object().unwrap()).unwrap();
    collect_templates(s.vm.heap(), root, &mut templates);
    let nodes = snap["nodes"].as_array().unwrap().clone();
    let mut imp = Importer {
        templates,
        cells: vec![0; nodes.len()],
        kinds: nodes
            .iter()
            .map(|n| n["t"].as_str().unwrap_or("?").to_string())
            .collect(),
    };
    // Pending calls: fresh promises registered in the new prelude's map (what an
    // `adopt(id)` in pi's prelude would do), standing in for the old ones. Running the VM
    // may collect, which moves cells: offsets are read only after the last run, before the
    // graph is built (building allocates but never collects).
    let pb0 = *captured_bindings(s.vm.heap(), &prelude_roots(&s))
        .get("pending")
        .ok_or("the prelude has no `pending`")?;
    let pm_handle = {
        let pm: Value = s.vm.heap().arena().get(pb0, BoxCell::VALUE);
        s.vm.heap_mut().new_handle(pm).unwrap()
    };
    let adopt_src = "return (pending, id) => new Promise((resolve, reject) => pending.set(id, { resolve, reject }))";
    let adopt =
        s.vm.compile(
            adopt_src.as_bytes(),
            &djs_core::compiler::CompileOptions::async_body("adopt.js", &[]),
        )
        .unwrap();
    let adopt = s.evaluate_pub(adopt)?;
    let mut calls = Vec::new();
    let mut adopted = Vec::new();
    for p in snap["pending"].as_array().unwrap() {
        let id = p["id"].as_f64().unwrap();
        let pm = s.vm.heap().get_handle(pm_handle);
        let a = s.vm.heap().get_handle(adopt);
        let h = s.invoke(a, Value::UNDEFINED, &[pm, Value::number(id)])?;
        let node = p["promise"]["ref"]
            .as_u64()
            .ok_or("pending promise is not a node")? as usize;
        adopted.push((node, h));
        calls.push(pi::Call {
            id: id as u32,
            name: p["name"].as_str().unwrap_or("").into(),
            args: p["args"].as_str().map(|a| a.to_string()),
        });
    }
    let thunk =
        s.vm.compile(
            b"return (p) => () => p",
            &djs_core::compiler::CompileOptions::async_body("adopt.js", &[]),
        )
        .unwrap();
    let thunk = s.evaluate_pub(thunk)?;
    // No more runs until the graph is built and held by handles.
    for (node, h) in &adopted {
        imp.cells[*node] = s.vm.heap().get_handle(*h).as_object().unwrap().offset();
    }
    let bindings = captured_bindings(s.vm.heap(), &prelude_roots(&s));
    let run_subscribed = crate::import::build(&mut imp, &mut s.vm, &nodes)?;
    // The prelude's state, by name.
    for (name, j) in snap["preludeState"].as_object().unwrap() {
        let Some(&b) = bindings.get(name.as_str()) else {
            // A field this prelude does not have: it must be at a neutral value.
            continue;
        };
        if MAPS.contains(&name.as_str()) {
            let m: Value = s.vm.heap().arena().get(b, BoxCell::VALUE);
            let m = m.as_object().unwrap();
            s.vm.heap_mut()
                .collection_clear(m)
                .map_err(|e| format!("{e:?}"))?;
            for e in j.as_array().unwrap() {
                let k = imp.value(s.vm.heap_mut(), &e[0])?;
                let v = imp.value(s.vm.heap_mut(), &e[1])?;
                s.vm.heap_mut()
                    .collection_set(m, k, v)
                    .map_err(|e| format!("{e:?}"))?;
            }
        } else {
            let v = imp.value(s.vm.heap_mut(), j)?;
            s.vm.heap_mut().arena_mut().set(b, BoxCell::VALUE, v);
        }
    }
    // The script's result promise: subscribed by the new prelude's `run`, which calls the
    // function it gets and attaches its handlers to the promise that function returns.
    let held: Vec<_> = run_subscribed
        .iter()
        .map(|&c| {
            s.vm.heap_mut()
                .new_handle(Value::object(obj_at(c)))
                .unwrap()
        })
        .collect();
    for h in held {
        let p = s.vm.heap().get_handle(h);
        let t = s.vm.heap().get_handle(thunk);
        let fh = s.invoke(t, Value::UNDEFINED, &[p])?;
        let fv = s.vm.heap().get_handle(fh);
        let run = s.vm.heap().get_handle(s.run);
        let api = s.vm.heap().get_handle(s.api);
        s.invoke(run, api, &[fv])?;
    }
    if std::env::var("GOLDI_TIME").is_ok() {
        eprintln!(
            "import: prelude run {t_prelude:?}, the rest {:?}",
            t0.elapsed() - t_prelude
        );
    }
    let host = &snap["host"];
    s.now = host["now"].as_f64().unwrap();
    s.ev.calls = calls;
    s.ev.issued = host["issued"].as_u64().unwrap() as u32;
    s.ev.output = host["output"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_str().unwrap().to_string())
        .collect();
    Ok((s, host["failedPage2"].as_bool().unwrap_or(false)))
}

/// Delivers the remaining results (as `drive` does) and returns the transcript.
fn finish(mut s: pi::Session, failed_page2: bool) -> String {
    let mut tools = Tools { failed_page2 };
    let mut now = s.now;
    while s.ev.done.is_none() && !s.ev.calls.is_empty() {
        let call = s.ev.calls.remove(0);
        let (ok, payload) = tools.call(&call.name, call.args.as_deref().unwrap_or(""));
        now += 1500.0;
        s.deliver(call.id, ok, &payload, now);
    }
    format!("{}\n[{:?}]", s.ev.output.join("\n"), s.ev.done)
}

/// portable goldi-tier2 [--save DIR] EXPORT_PRELUDE IMPORT_PRELUDE CASE...
/// At every idle point: export by meaning on EXPORT_PRELUDE, import onto a fresh run of
/// IMPORT_PRELUDE, finish, and compare with the uninterrupted runs.
pub fn tier2(args: &[String]) {
    let mut args = args.to_vec();
    let mut save = None;
    if args.first().map(|a| a.as_str()) == Some("--save") {
        save = Some(args[1].clone());
        args.drain(..2);
        std::fs::create_dir_all(save.as_ref().unwrap()).unwrap();
    }
    let p_export = std::fs::read_to_string(&args[0]).unwrap();
    let p_import = std::fs::read_to_string(&args[1]).unwrap();
    let inputs = realistic_inputs();
    let (mut n, mut ok, mut as_import) = (0, 0, 0);
    let mut fails: Vec<String> = Vec::new();
    let mut sizes = Vec::new();
    let (mut t_export, mut t_import) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for case in &args[2..] {
        let name = std::path::Path::new(case)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let code = std::fs::read_to_string(case).unwrap();
        let whole = |p: &str| {
            let s = pi::Session::prepare_with(p, &inputs, T0);
            let s = drive(s, &code, |_, _| {});
            format!("{}\n[{:?}]", s.ev.output.join("\n"), s.ev.done)
        };
        let want = whole(&p_export);
        let want_import = whole(&p_import);
        if let Some(d) = &save {
            std::fs::write(format!("{d}/{name}.transcript"), &want).unwrap();
        }
        // Idle points: replay to each (k results delivered), export there.
        let mut k = 0;
        loop {
            let mut s = pi::Session::prepare_with(&p_export, &inputs, T0);
            s.start(&code, 42);
            let mut tools = Tools {
                failed_page2: false,
            };
            let mut now = T0;
            for _ in 0..k {
                if s.ev.done.is_some() || s.ev.calls.is_empty() {
                    break;
                }
                let call = s.ev.calls.remove(0);
                let (o, payload) = tools.call(&call.name, call.args.as_deref().unwrap_or(""));
                now += 1500.0;
                s.deliver(call.id, o, &payload, now);
            }
            if s.ev.done.is_some() || s.ev.calls.is_empty() {
                break;
            }
            n += 1;
            let t = Instant::now();
            let snap = export_pi(&mut s, &code, &inputs, tools.failed_page2);
            t_export += t.elapsed();
            match snap {
                Err(e) => fails.push(format!("{name}@{k}: export: {e:?}")),
                Ok(snap) => {
                    let text = serde_json::to_string(&snap).unwrap();
                    sizes.push(text.len());
                    if let Some(d) = &save {
                        std::fs::write(format!("{d}/{name}@{k}.json"), &text).unwrap();
                    }
                    let t = Instant::now();
                    let r = import_pi(&snap, &p_import);
                    t_import += t.elapsed();
                    match r {
                        Err(e) => fails.push(format!("{name}@{k}: import: {e}")),
                        Ok((r, f2)) => {
                            let got = finish(r, f2);
                            if got == want {
                                ok += 1;
                            } else if got == want_import {
                                as_import += 1;
                            } else {
                                fails.push(format!("{name}@{k}: transcript differs"));
                            }
                        }
                    }
                }
            }
            k += 1;
        }
    }
    sizes.sort();
    println!(
        "idle points {n}: continued as the exporting prelude's run {ok}, as the importing prelude's own run {as_import}, failed {}",
        fails.len()
    );
    println!(
        "snapshot JSON median {} B, max {} B; export {:.2?}, import {:.2?} per point (native dev build)",
        sizes.get(sizes.len() / 2).copied().unwrap_or(0),
        sizes.last().copied().unwrap_or(0),
        t_export / n.max(1),
        t_import / n.max(1)
    );
    for f in fails.iter().take(15) {
        println!("  {f}");
    }
}

/// portable goldi-import-all IMPORT_PRELUDE DIR: imports every snapshot `goldi-tier2 --save`
/// wrote (possibly by another build and prelude) and compares with the saved transcript.
pub fn import_all(args: &[String]) {
    let prelude = std::fs::read_to_string(&args[0]).unwrap();
    let dir = &args[1];
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let (mut n, mut same) = (0, 0);
    let mut fails = Vec::new();
    for f in files {
        n += 1;
        let stem = f.file_stem().unwrap().to_string_lossy().to_string();
        let case = stem.split('@').next().unwrap().to_string();
        let want = std::fs::read_to_string(format!("{dir}/{case}.transcript")).unwrap();
        let snap: J = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        match import_pi(&snap, &prelude) {
            Ok((s, f2)) => {
                if finish(s, f2) == want {
                    same += 1;
                } else {
                    fails.push(format!("{stem}: transcript differs"));
                }
            }
            Err(e) => fails.push(format!("{stem}: {e}")),
        }
    }
    println!("snapshots {n}: same transcript as the exporting build's run {same}");
    for f in fails.iter().take(10) {
        println!("  {f}");
    }
}

/// portable goldi-reloc IMAGE...: for session images taken from a sealed prelude image
/// (`goldi-images`' `*-today-K.img`), what an "image with symbolic relocations" would have
/// to carry: the session's own cells (above `OLD_END`) by kind, and their references into
/// the reference (the prelude image) by target: builtins by path, prelude objects by path,
/// and engine-internal cells (shapes, templates, the realm...) that have no name.
#[cfg(not(feature = "old"))]
pub fn reloc(args: &[String]) {
    use djs_core::heap::cells::Roots;
    use djs_core::heap::{CellKind, ObjectClass};
    use std::collections::BTreeMap;
    for path in args {
        let bytes = std::fs::read(path).unwrap();
        let vm = Vm::from_bytes(bytes, pi::opts()).unwrap();
        let h = vm.heap();
        let old_end: u32 = h.arena().get(0, Roots::OLD_END);
        let mut ex = Exporter::new(&vm);
        ex.prelude = prelude_names(h, &ex.builtins);
        let (builtins, prelude) = (ex.builtins.clone(), ex.prelude.clone());
        let mut kinds: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        let mut targets: BTreeMap<String, u32> = BTreeMap::new();
        let mut distinct: BTreeMap<String, std::collections::BTreeSet<u32>> = BTreeMap::new();
        for c in h.cells() {
            if c.offset < old_end {
                continue;
            }
            let name = if c.kind == CellKind::Object {
                format!("Object/{:?}", h.object_class(obj_at(c.offset)))
            } else {
                format!("{:?}", c.kind)
            };
            let e = kinds.entry(name).or_default();
            e.0 += 1;
            e.1 += c.size;
            if c.kind == CellKind::FuncTemplate {
                // Numbers in the session's own bytecode that name things of the build:
                // atom ids and intrinsic ids.
                let t = template_info(h, c.offset);
                let mut pc = 0u32;
                while (pc as usize) < t.code.len() {
                    let insn = djs_core::bytecode::decode::decode(&t.code, pc).unwrap();
                    let info = insn.info();
                    for k in info.imms {
                        if *k == djs_core::bytecode::ImmKind::Atom {
                            *targets
                                .entry("atom id in session bytecode".into())
                                .or_default() += 1;
                        }
                    }
                    if info.name == "call_intrinsic" {
                        *targets
                            .entry("intrinsic id in session bytecode".into())
                            .or_default() += 1;
                    }
                    pc = insn.next_pc();
                }
            }
            if c.kind == CellKind::Shape {
                *targets
                    .entry("atom id in a session shape".into())
                    .or_default() += 1;
            }
            h.trace_cell(c.offset, |slot| {
                let Some(t) = h.slot_target(slot) else { return };
                if t >= old_end {
                    return;
                }
                let class = if h.cell_kind(t) == CellKind::Object {
                    if builtins.contains_key(&t) {
                        "builtin object (by path)".to_string()
                    } else if prelude.contains_key(&t) {
                        "prelude object (by path)".to_string()
                    } else {
                        format!("unnamed object {:?}", h.object_class(obj_at(t)))
                    }
                } else {
                    format!("engine cell {:?}", h.cell_kind(t))
                };
                *targets.entry(class.clone()).or_default() += 1;
                distinct.entry(class).or_default().insert(t);
            });
        }
        let _ = ObjectClass::Ordinary;
        let young: u32 = kinds.values().map(|v| v.1).sum();
        println!(
            "== {path}: image {} B, reference part {old_end} B, session cells {} B in {} cells",
            h.arena().len(),
            young,
            kinds.values().map(|v| v.0).sum::<u32>()
        );
        let mut ks: Vec<_> = kinds.into_iter().collect();
        ks.sort_by(|a, b| b.1.1.cmp(&a.1.1));
        for (k, (n, b)) in ks.iter().take(12) {
            println!("  {k:24} {n:6} cells {b:8} B");
        }
        println!("  references from session cells into the reference (count / distinct targets):");
        for (k, n) in &targets {
            let d = distinct
                .get(k)
                .map(|d| d.len().to_string())
                .unwrap_or("-".into());
            println!("    {k:40} {n:6} / {d}");
        }
    }
}
