//! End-to-end tests: every `tests/data/e2e/*.js` case is compiled and run through the
//! `Vm` in three modes, and each must print exactly its `.expected` output (generated
//! from QuickJS by `tests/data/e2e/gen-expected.sh`):
//!
//! 1. plain: all the fuel at once;
//! 2. suspension: fuel 1 per `run`, and at every stop (out of fuel or a host call) the VM
//!    continues in a fresh one restored from its heap image;
//! 3. GC stress: a collection at every safepoint.

use std::path::{Path, PathBuf};

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const LOG: u32 = 1;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Plain,
    Suspend,
    GcStress,
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/e2e")
}

/// Runs one case and returns its output: console.log lines, then `Uncaught ...` or a
/// compile error if there is one.
fn run_case(src: &str, name: &str, mode: Mode) -> String {
    let exec = ExecOptions {
        gc_stress: matches!(mode, Mode::GcStress),
        ..Default::default()
    };
    let opts = VmOptions {
        exec,
        ..Default::default()
    };
    let mut vm = Vm::new(opts).unwrap();
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let body = src.starts_with("// mode: body");
    let params = ["tools", "console"];
    let copts = if body {
        CompileOptions::async_body(name, &params)
    } else {
        CompileOptions::script(name)
    };
    let main = match vm.compile(src.as_bytes(), &copts) {
        Ok(f) => f,
        Err(e) => {
            assert_eq!(e.kind, CompileErrorKind::Syntax);
            return format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col);
        }
    };
    let args = if body {
        vec![Value::UNDEFINED, Value::object(console)]
    } else {
        vec![]
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &args).unwrap();
    let mut out = String::new();
    let mut stops: u64 = 0;
    loop {
        // Suspension: fuel 1 (a stop at every instruction) for the first 5000 stops, then
        // larger slices, so long-running cases stay fast while every case still gets
        // thousands of image round trips.
        let fuel = match mode {
            Mode::Suspend if stops < 5000 => 1,
            Mode::Suspend => 997,
            _ => u64::MAX,
        };
        stops += 1;
        let status = vm.run(fuel);
        match status {
            Status::Done(_) => break,
            Status::Threw(e) => {
                out += &format!("Uncaught {}\n", vm.display(e));
                break;
            }
            Status::OutOfFuel => {}
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                out += &parts.join(" ");
                out.push('\n');
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle => panic!("idle"),
        }
        if matches!(mode, Mode::Suspend) {
            let o = vm.options();
            vm = Vm::from_bytes(vm.bytes().to_vec(), o).unwrap();
        }
    }
    vm.heap().verify().unwrap();
    out
}

#[test]
fn e2e_cases() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "js"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 20, "found only {} cases", paths.len());
    let mut failures = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        let want = std::fs::read_to_string(path.with_extension("expected"))
            .unwrap_or_else(|_| panic!("{name}: no .expected (run gen-expected.sh)"));
        let modes: Vec<Mode> = match std::env::var("DJS_E2E_MODE").as_deref() {
            Ok("plain") => vec![Mode::Plain],
            Ok("suspend") => vec![Mode::Suspend],
            Ok("gc") => vec![Mode::GcStress],
            _ => vec![Mode::Plain, Mode::Suspend, Mode::GcStress],
        };
        for mode in modes {
            let got = run_case(&src, &name, mode);
            if got != want {
                failures.push(format!(
                    "{name} ({mode:?}):\n--- want\n{want}--- got\n{got}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
