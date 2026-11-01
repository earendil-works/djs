//! End-to-end tests: every `tests/data/e2e/*.js` case is compiled and run through the
//! `Vm` in three modes, and each must print exactly its `.expected` output (generated
//! from QuickJS by `tests/data/e2e/gen-expected.sh`):
//!
//! 1. plain: all the fuel at once;
//! 2. suspension: fuel 1 per `run`, and at every stop (out of fuel or a host call) the VM
//!    continues in a fresh one restored from its heap image;
//! 3. GC stress: a collection at every safepoint.
//!
//! The plain and suspension modes must also leave identical heap images after a final
//! collection (0072, D1720): the copying GC lays the heap out as a function of the object
//! graph (D1090) and collects at the same points under any slicing and restore (D1601), so
//! a stop or an image round trip that changes the graph shows. GC stress is a different
//! collection policy: weak tables lose their dead entries at other times (and compact
//! differently), so its image is not compared.

use std::path::{Path, PathBuf};

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const LOG: u32 = 1;

/// The image of a VM with the prelude and `console.log`, and its console object (the
/// same offset in every copy).
fn base() -> &'static (Vec<u8>, djs_core::value::Obj) {
    static BASE: std::sync::OnceLock<(Vec<u8>, djs_core::value::Obj)> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
        let console = vm.new_object().unwrap();
        let log = vm.new_host_function(LOG, "log").unwrap();
        vm.define(console, "log", log).unwrap();
        vm.define_global("console", Value::object(console)).unwrap();
        (vm.bytes().to_vec(), console)
    })
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Plain,
    Suspend,
    GcStress,
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/e2e")
}

/// FNV-1a, 64 bits.
fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Runs one case and returns its output (console.log lines, then `Uncaught ...` or a
/// compile error if there is one) and the hash of the heap image after a final collection
/// (`None` for a compile error).
fn run_case(src: &str, name: &str, mode: Mode) -> (String, Option<u64>) {
    let exec = ExecOptions {
        gc_stress: matches!(mode, Mode::GcStress),
        ..Default::default()
    };
    let opts = VmOptions {
        exec,
        ..Default::default()
    };
    // Every case starts from one base image (prelude + console), as a base snapshot
    // would: building the prelude per case dominated the run time.
    let (image, console) = base();
    let mut vm = Vm::from_bytes(image.clone(), opts).unwrap();
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
            let msg = format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col);
            return (msg, None);
        }
    };
    let args = if body {
        vec![Value::UNDEFINED, Value::object(*console)]
    } else {
        vec![]
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &args).unwrap();
    let mut out = String::new();
    let mut stops: u64 = 0;
    loop {
        // Suspension: fuel 1 (a stop at every instruction) for the first 1000 stops, then
        // larger slices, so long-running cases stay fast while every case still gets
        // thousands of image round trips.
        let fuel = match mode {
            Mode::Suspend if stops < 1000 => 1,
            Mode::Suspend => 997,
            _ => u64::MAX,
        };
        stops += 1;
        let status = vm.run(fuel);
        match status {
            Status::Done(v) => {
                let rejections = vm.take_unhandled_rejections();
                // A body is an async function (D008): gen-expected.sh catches its
                // rejected promise and prints `Uncaught <reason>`.
                if body && let Some(Some(Err(e))) = vm.promise_state(v) {
                    out += &format!("Uncaught {}\n", vm.display(e));
                    break;
                }
                // qjs reports these on stderr, and gen-expected.sh keeps its first line.
                if let Some(&r) = rejections.first() {
                    out += &format!(
                        "Uncaught Possibly unhandled promise rejection: {}\n",
                        vm.display(r)
                    );
                }
                break;
            }
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
    vm.collect().unwrap();
    (out, Some(fnv64(vm.bytes())))
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
        let mut images: Vec<(Mode, Option<u64>)> = Vec::new();
        // GC stress is not compared (see the module docs).
        for mode in modes {
            let (got, image) = run_case(&src, &name, mode);
            if got != want {
                failures.push(format!(
                    "{name} ({mode:?}):\n--- want\n{want}--- got\n{got}"
                ));
            }
            if !matches!(mode, Mode::GcStress) {
                images.push((mode, image));
            }
        }
        if images.iter().any(|(_, h)| *h != images[0].1) {
            failures.push(format!(
                "{name}: the heap images after a collection differ between the plain and suspension modes: {images:x?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
