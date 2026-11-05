//! End-to-end tests: every `tests/data/e2e/*.js` case is compiled and run through the
//! `Vm` in three modes, and each must print exactly its `.expected` output (generated
//! from QuickJS by `tests/data/e2e/gen-expected.sh`):
//!
//! 1. plain: all the fuel at once;
//! 2. suspension: fuel 1 per `run`, and at every stop (out of fuel or a host call) the VM
//!    continues in a fresh one restored from its heap image;
//! 3. GC stress: a collection at every safepoint.
//!
//! The plain and suspension modes must also leave identical heap images, both the raw
//! image at the end and its canonical hash (0072, D1720, D1725; 0101, D2040): collections
//! happen at the same instructions under any slicing and restore (D1601, D1710), so the raw
//! arenas are byte-identical, and the canonical hash (a copying collection of a copy,
//! D1730) is a function of the object graph alone, so a stop or an image round trip that
//! changes anything shows. GC stress is a different collection policy: weak tables lose
//! their dead entries at other times, so its images are not compared.
//!
//! Cases run in parallel (`DJS_JOBS` threads, else one per CPU). Each run has a fuel
//! budget and a time limit: a case that loops forever fails its plain run within seconds
//! (and then skips the stress modes) instead of hanging the test.
//! 4. dump (0172, D2830): the run with frozen builtins, which dumps need (D2836), then a
//!    dump of the finished VM (its globals and whatever is still in flight), run as a
//!    Script in a fresh VM and dumped again, twice: the third dump must be the second.
//!    The cases have no idle points before their end (their awaits settle through jobs),
//!    so this checks the exporter on every construct the cases leave behind. Its output is
//!    not compared (cases that change builtins fail there on purpose); refusals are
//!    counted, not failures.
//!
//! `DJS_E2E_MODE=plain|suspend|gc|dump` runs one mode, `DJS_E2E_CASE=<substring>` only
//! the matching cases.

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
    Dump,
}

/// The options of a dump mode VM: frozen builtins, which dumps need (D2836).
fn frozen() -> VmOptions {
    VmOptions {
        frozen_builtins: true,
        ..VmOptions::default()
    }
}

/// A VM of the dump mode with `console.log`, and its console object.
fn dump_vm() -> (Vm, djs_core::value::Obj) {
    let mut vm = djs_prelude::new_vm(frozen()).unwrap();
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    (vm, console)
}

/// A dump's hooks (D2890), above every case's host function id.
const SEED: u32 = 0x7fff_fff0;
const RESUME: u32 = 0x7fff_fff1;
const HOST: u32 = 0x7fff_fff2;

/// Runs dump `text` in `vm`, a fresh one, as a host does: `$djs` is the dump runtime with
/// the hooks `seed`, `resume` (a finished VM has no open call to resume) and `host` (the
/// host's values are the globals of their names, D2971), and the
/// Script runs to its end. Why it failed, if it did.
fn run_dump(vm: &mut Vm, text: &str) -> Result<(), String> {
    let hooks = vm
        .dump_runtime_for(text)?
        .as_object()
        .ok_or("no dump runtime")?;
    let seed = vm.new_host_function(SEED, "seed").unwrap();
    vm.define(hooks, "seed", seed).unwrap();
    let resume = vm.new_host_function(RESUME, "resume").unwrap();
    vm.define(hooks, "resume", resume).unwrap();
    let host = vm.new_host_function(HOST, "host").unwrap();
    vm.define(hooks, "host", host).unwrap();
    vm.define_global("$djs", Value::object(hooks)).unwrap();
    let f = vm
        .compile(text.as_bytes(), &CompileOptions::script("dump.js"))
        .map_err(|e| format!("{} at {}:{}", e.message, e.line, e.col))?;
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).map_err(|e| format!("{e:?}"))?;
    loop {
        match vm.run(FUEL_BUDGET) {
            // (The host deletes `$djs` and `$dump` once the Script ran, D2916.)
            Status::Done(_) | Status::Idle => {
                return vm.end_dump_load().map_err(|e| format!("{e:?}"));
            }
            Status::HostCall { call_id, fn_id } if fn_id == SEED => {
                let args = vm.host_call(call_id).unwrap().args;
                // (`$djs.seed(lo, hi[, next])` or `$djs.seed(next)`, D2837.)
                let half = |i: usize| args.get(i).and_then(|v| v.as_int()).unwrap_or(0) as u32;
                if args.get(1).is_some_and(|v| !v.is_undefined()) {
                    vm.restore_math_random(u64::from(half(0)) | (u64::from(half(1)) << 32));
                }
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::HostCall { call_id, fn_id } if fn_id == HOST => {
                let args = vm.host_call(call_id).unwrap().args;
                let name = args.first().and_then(|v| v.as_string());
                let name = name.map(|s| vm.heap().str_to_string(s)).unwrap_or_default();
                let v = vm
                    .global_path(&name)
                    .ok_or(format!("host: unknown {name}"))?;
                vm.resolve_host_call(call_id, Ok(v)).unwrap();
            }
            Status::HostCall { call_id, .. } => {
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Threw(e) => return Err(format!("threw {}", vm.display(e))),
            s => return Err(format!("{s:?}")),
        }
    }
}

/// Dumps a finished VM and runs the dump in a fresh VM, twice: a failure if a run fails
/// or the second round does not give the same text again (the first may order the global
/// bindings differently: a loaded dump declares them in its own order). Refusals are
/// counted in `DUMP_REFUSED`.
fn dump_round_trip(vm: &Vm) -> Option<String> {
    use djs_core::vm::dump::DumpOptions;
    let name = |id: u32| (id == LOG).then(|| String::from("log"));
    // `console` is the host's: re-bound by name, its `log` written as `console.log`.
    let name = |id: u32| name(id).map(|n| format!("console.{n}"));
    let opts = DumpOptions {
        base: djs_prelude::BASE,
        host: &["console"],
        host_fn_name: &name,
        ..DumpOptions::default()
    };
    let first = match vm.dump(&opts, None) {
        Ok(t) => t,
        Err(e) => {
            DUMP_REFUSED.fetch_add(1, Ordering::Relaxed);
            if std::env::var_os("DJS_E2E_DUMP_REFUSALS").is_some() {
                eprintln!("{}", e.message());
            }
            return None;
        }
    };
    let mut texts = vec![first];
    for round in 0..2 {
        let text = texts.last().unwrap();
        let (mut b, _) = dump_vm();
        if let Err(e) = run_dump(&mut b, text) {
            return Some(format!(
                "<e2e: dump {} does not run: {e}>\n{text}\n",
                round + 1
            ));
        }
        match b.dump(&opts, None) {
            Ok(again) => texts.push(again),
            Err(e) => {
                return Some(format!(
                    "<e2e: the loaded dump is refused: {}>\n{text}\n",
                    e.message()
                ));
            }
        }
    }
    if texts[1] != texts[2] {
        return Some(format!(
            "<e2e: a loaded dump dumps differently>\n--- first\n{}\n--- second\n{}\n",
            texts[1], texts[2]
        ));
    }
    DUMP_OK.fetch_add(1, Ordering::Relaxed);
    None
}

static DUMP_OK: AtomicUsize = AtomicUsize::new(0);
static DUMP_REFUSED: AtomicUsize = AtomicUsize::new(0);

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

/// The hashes of a run's final heap image: as it is (raw), and its canonical hash (the
/// graph alone, `Vm::canonical_hash`, D1730).
type Images = (u64, u64);

/// Runs one case and returns its output (console.log lines, then `Uncaught ...` or a
/// compile error if there is one) and its final images (`None` for a compile error or a
/// run over budget).
fn run_case(src: &str, name: &str, mode: Mode) -> (String, Option<Images>) {
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
    let (mut vm, console) = if matches!(mode, Mode::Dump) {
        dump_vm()
    } else {
        (Vm::from_bytes(image.clone(), opts).unwrap(), *console)
    };
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
        vec![Value::UNDEFINED, Value::object(console)]
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
                if matches!(mode, Mode::Dump)
                    && let Some(f) = dump_round_trip(&vm)
                {
                    out += &f;
                }
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
            Status::HostOutOfMemory => panic!("out of host memory"),
        }
        if matches!(mode, Mode::Suspend) {
            let o = vm.options();
            vm = Vm::from_bytes(vm.image().to_vec(), o).unwrap();
        }
    }
    vm.heap().verify().unwrap();
    let raw = fnv64(vm.image());
    (out, Some((raw, vm.canonical_hash().unwrap())))
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
        Ok("dump") => (false, vec![Mode::Dump]),
        _ => (true, vec![Mode::GcStress, Mode::Suspend, Mode::Dump]),
    };
    let mut failures = Vec::new();
    // The image hash of each case's plain and suspension runs (D1720).
    let mut images: Vec<Vec<(Mode, Option<Images>)>> = vec![Vec::new(); paths.len()];
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
        if matches!(mode, Mode::Suspend) || matches!(mode, Mode::Plain) {
            images[index(path)].push((mode, image));
        }
        failures.extend(f);
    }
    for (path, images) in paths.iter().zip(&images) {
        if images.iter().any(|(_, h)| *h != images[0].1) {
            let name = path.file_name().unwrap().to_string_lossy();
            failures.push(format!(
                "{name}: the heap images (raw, and the canonical hash) differ between the plain and suspension modes: {images:x?}"
            ));
        }
    }
    if stress.iter().any(|m| matches!(m, Mode::Dump)) {
        eprintln!(
            "dump: {} cases round-trip, {} refused",
            DUMP_OK.load(Ordering::Relaxed),
            DUMP_REFUSED.load(Ordering::Relaxed)
        );
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
fn run_parallel(jobs: &[(&Path, Mode)]) -> Vec<(Option<String>, Option<Images>)> {
    let threads = std::env::var("DJS_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<(Option<String>, Option<Images>)>> =
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
fn check_case(path: &Path, mode: Mode) -> (Option<String>, Option<Images>) {
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
    // The dump mode runs with frozen builtins: only its round trips count.
    let failed = match mode {
        Mode::Dump => got.contains("<e2e: "),
        _ => got != want,
    };
    let failure = failed.then(|| format!("{name} ({mode:?}):\n--- want\n{want}--- got\n{got}"));
    (failure, image)
}
