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
        let desc: String = words.split(' ').cycle().skip(i * 3).take(40 + 5 * i).collect::<Vec<_>>().join(" ");
        tools.push(format!(r#"{{"name":"{t}","jsName":"{t}","description":"{desc}"}}"#));
    }
    for t in ["mcp__issues__create", "mcp__issues__comment", "mcp__browser__open", "mcp__browser__shot"] {
        let js = t.replace("__", "_");
        tools.push(format!(r#"{{"name":"{t}","jsName":"{js}","description":"{words}"}}"#));
    }
    let globals = r#"[{"name":"searchTools","spread":true},{"name":"describeTool","spread":false},{"name":"models.getAvailableOfType","spread":false},{"name":"models.classify","spread":true},{"name":"subagent","spread":false},{"name":"fetchUrl","spread":false}]"#;
    let mut store = String::from("{");
    for k in 0..6 {
        if k > 0 {
            store.push(',');
        }
        let _ = write!(store, r#""key{k}":"{{\"ids\":[{}],\"note\":\"summary {k}\"}}""#, (0..40).map(|i| (i * 7919 + k) % 10007).map(|n| n.to_string()).collect::<Vec<_>>().join(","));
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
    s.vm.seal().unwrap();
    let img = s.vm.bytes().to_vec();
    (s, img)
}

/// Runs `s` (configured) through `code`, calling `at_idle` with the collected image at every
/// idle point.
fn drive(mut s: pi::Session, code: &str, mut at_idle: impl FnMut(usize, &[u8])) -> pi::Session {
    s.start(code, 42);
    let mut tools = Tools { failed_page2: false };
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
        std::fs::write(format!("{dir}/inputs.json"), format!("{}\n{}\n{}\n", inputs.tools, inputs.globals, inputs.store)).unwrap();
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
        for (n, b) in [("exact", &exact), ("empty", &empty), ("stage1", &stage1), ("base", &base)] {
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
            let name = std::path::Path::new(case).file_stem().unwrap().to_string_lossy().to_string();
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
            let _ = writeln!(times, "  {name}: today and layered give the same output and result: {same}");
        }
    }
    std::fs::write(format!("{out}/times.txt"), &times).unwrap();
    print!("{times}");
}
