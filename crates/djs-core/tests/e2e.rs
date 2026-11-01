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
//!
//! Cases run in parallel (`DJS_JOBS` threads, else one per CPU). Each run has a fuel
//! budget and a time limit: a case that loops forever fails its plain run within seconds
//! (and then skips the stress modes) instead of hanging the test.
//! `DJS_E2E_MODE=plain|suspend|gc` runs one mode, `DJS_E2E_CASE=<substring>` only the
//! matching cases.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const LOG: u32 = 1;

/// Fuel budget per run: ~30 times what the largest case uses (about 0.7M), so only a
/// runaway loop reaches it; a plain run gets there in well under a second.
const FUEL_BUDGET: u64 = 20_000_000;
/// Output budget per run: catches a loop that only calls console.log.
const MAX_LINES: usize = 100_000;
/// Wall-clock limit per run, for the stress modes, which are much slower per fuel.
const TIME_LIMIT: Duration = Duration::from_secs(120);
/// Fuel per `run` in the plain and GC stress modes; the budgets are checked between
/// slices.
const SLICE: u64 = 1 << 16;
/// What a run that exceeds its budget prints.
const OVER_BUDGET: &str = "<e2e: over budget>";

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
/// (`None` for a compile error or a run over budget).
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
    let (mut spent, mut lines, start) = (0u64, 0usize, Instant::now());
    loop {
        // Suspension: fuel 1 (a stop at every instruction) for the first 1000 stops, then
        // larger slices, so long-running cases stay fast while every case still gets
        // thousands of image round trips.
        let fuel = match mode {
            Mode::Suspend if stops < 1000 => 1,
            Mode::Suspend => 997,
            _ => SLICE,
        };
        stops += 1;
        if spent >= FUEL_BUDGET || lines >= MAX_LINES || start.elapsed() >= TIME_LIMIT {
            out += &format!(
                "{OVER_BUDGET} {spent} fuel, {lines} lines, {:.0?}\n",
                start.elapsed()
            );
            return (out, None);
        }
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
            Status::OutOfFuel => spent += fuel,
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                out += &parts.join(" ");
                out.push('\n');
                lines += 1;
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
    let filter = std::env::var("DJS_E2E_CASE").unwrap_or_default();
    paths.retain(|p| p.to_string_lossy().contains(&filter));
    assert!(!paths.is_empty(), "no case matches DJS_E2E_CASE={filter}");
    let (plain, stress): (bool, Vec<Mode>) = match std::env::var("DJS_E2E_MODE").as_deref() {
        Ok("plain") => (true, vec![]),
        Ok("suspend") => (false, vec![Mode::Suspend]),
        Ok("gc") => (false, vec![Mode::GcStress]),
        _ => (true, vec![Mode::GcStress, Mode::Suspend]),
    };
    let mut failures = Vec::new();
    // The image hash of each case's plain and suspension runs (D1720).
    let mut images: Vec<Vec<(Mode, Option<u64>)>> = vec![Vec::new(); paths.len()];
    let index = |p: &Path| paths.iter().position(|q| q == p).unwrap();
    // The plain runs first (all of them take well under a second): a case that runs over
    // its budget there is not run again in the much slower stress modes.
    let mut runaway = Vec::new();
    if plain {
        let jobs: Vec<(&Path, Mode)> = paths.iter().map(|p| (p.as_path(), Mode::Plain)).collect();
        for (&(path, mode), (f, image)) in jobs.iter().zip(run_parallel(&jobs)) {
            images[index(path)].push((mode, image));
            if let Some(f) = f {
                if f.contains(OVER_BUDGET) {
                    runaway.push(path);
                }
                failures.push(f);
            }
        }
    }
    // The slow GC stress runs first, so the longest jobs start early.
    let jobs: Vec<(&Path, Mode)> = stress
        .iter()
        .flat_map(|&m| paths.iter().map(move |p| (p.as_path(), m)))
        .filter(|(p, _)| !runaway.contains(p))
        .collect();
    for (&(path, mode), (f, image)) in jobs.iter().zip(run_parallel(&jobs)) {
        // GC stress is not compared (see the module docs).
        if !matches!(mode, Mode::GcStress) {
            images[index(path)].push((mode, image));
        }
        failures.extend(f);
    }
    for (path, images) in paths.iter().zip(&images) {
        if images.iter().any(|(_, h)| *h != images[0].1) {
            let name = path.file_name().unwrap().to_string_lossy();
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

/// Runs `jobs` on `DJS_JOBS` threads (else one per CPU); the failure and image hash of
/// each, in order.
fn run_parallel(jobs: &[(&Path, Mode)]) -> Vec<(Option<String>, Option<u64>)> {
    let threads = std::env::var("DJS_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<(Option<String>, Option<u64>)>> =
        jobs.iter().map(|_| Mutex::new((None, None))).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.min(jobs.len()) {
            // The compiler and interpreter recurse on deep inputs: give the workers a
            // generous stack.
            std::thread::Builder::new()
                .stack_size(16 << 20)
                .spawn_scoped(s, || {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(path, mode)) = jobs.get(i) else {
                            break;
                        };
                        *results[i].lock().unwrap() = check_case(path, mode);
                    }
                })
                .unwrap();
        }
    });
    results
        .into_iter()
        .map(|r| r.into_inner().unwrap())
        .collect()
}

/// Runs one case in one mode: a failure description if its output is not the expected
/// one (an engine panic counts as a failure of that case, not of the whole run), and the
/// image hash.
fn check_case(path: &Path, mode: Mode) -> (Option<String>, Option<u64>) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let src = std::fs::read_to_string(path).unwrap();
    let Ok(want) = std::fs::read_to_string(path.with_extension("expected")) else {
        return (
            Some(format!("{name}: no .expected (run gen-expected.sh)")),
            None,
        );
    };
    let (got, image) =
        std::panic::catch_unwind(|| run_case(&src, &name, mode)).unwrap_or_else(|p| {
            let msg = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            (format!("<e2e: panicked: {msg}>\n"), None)
        });
    let failure =
        (got != want).then(|| format!("{name} ({mode:?}):\n--- want\n{want}--- got\n{got}"));
    (failure, image)
}
