//! Raw heap images do not depend on the fuel slice (0071, D1710): a script run with all
//! its fuel at once, or in slices of any size (with or without a restore from the image
//! at every stop), leaves byte-identical arenas, not just identical images after a final
//! collection. The GC runs only at natural safepoints, and an intrinsic suspended mid-way
//! frees its progress record when it resumes.

use std::path::{Path, PathBuf};

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

const LOG: u32 = 1;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `src` in slices of `slice` fuel; returns its output, the final raw image and the
/// total fuel used.
fn run(src: &str, slice: u64, restore: bool) -> (String, Vec<u8>, u64) {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    vm.set_now(1_700_000_000_000.0);
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let body = src.starts_with("// mode: body");
    let copts = if body {
        CompileOptions::async_body("case.js", &["tools", "console"])
    } else {
        CompileOptions::script("case.js")
    };
    let main = vm.compile(src.as_bytes(), &copts).expect("compiles");
    let args = if body {
        vec![Value::UNDEFINED, Value::object(console)]
    } else {
        vec![]
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &args).unwrap();
    let mut out = String::new();
    let mut fuel = 0;
    loop {
        let (st, used) = vm.run_metered(slice);
        fuel += used;
        match st {
            Status::Done(_) | Status::Idle => break,
            Status::Threw(e) => {
                out += &format!("Uncaught {}\n", vm.display(e));
                break;
            }
            Status::HostOutOfMemory => panic!("host out of memory"),
            Status::OutOfFuel => {}
            Status::HostCall { call_id, .. } => {
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                out += &parts.join(" ");
                out.push('\n');
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
        }
        if restore {
            let o = vm.options();
            vm = Vm::from_bytes(vm.bytes().to_vec(), o).unwrap();
        }
    }
    (out, vm.bytes().to_vec(), fuel)
}

/// Every schedule of the script at `path` against all the fuel at once: the same
/// output, arena and total fuel.
fn check(path: &Path, schedules: &[(u64, bool)]) -> Result<(), String> {
    let src = std::fs::read_to_string(path).unwrap();
    check_src(&path.display().to_string(), &src, schedules)
}

fn check_src(name: &str, src: &str, schedules: &[(u64, bool)]) -> Result<(), String> {
    let src = src.to_string();
    let (want_out, want_raw, want_fuel) = run(&src, u64::MAX, false);
    for &(slice, restore) in schedules {
        let t = std::time::Instant::now();
        let (out, raw, fuel) = run(&src, slice, restore);
        eprintln!(
            "TIME {name} {slice} {restore} {:?} fuel {fuel}",
            t.elapsed()
        );
        if out != want_out {
            return Err(format!(
                "{}: slice {slice} restore {restore}: output differs",
                name
            ));
        }
        if fuel != want_fuel {
            return Err(format!(
                "{}: slice {slice} restore {restore}: fuel {fuel} vs {want_fuel}",
                name
            ));
        }
        if raw != want_raw {
            let at = raw.iter().zip(&want_raw).position(|(a, b)| a != b);
            return Err(format!(
                "{}: slice {slice} restore {restore}: raw image differs (len {} vs {}, first at {at:?})",
                name,
                raw.len(),
                want_raw.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn raw_images_independent_of_slice() {
    // 0055's cases: a pending collection at a slice end (alloc_only), suspended matchers
    // and their progress records (regexp, regexp_props), iterator builtins.
    // Restoring copies the whole image at every stop, so alloc_only (2 MB) restores
    // at a larger slice.
    let slices = [(997, false), (101, false), (1, false)];
    let cases: [(&str, (u64, bool)); 4] = [
        ("scripts/determinism/js/alloc_only.js", (100_003, true)),
        ("tests/data/e2e/regexp.js", (101, true)),
        ("tests/data/e2e/regexp_props.js", (101, true)),
        ("tests/data/e2e/iteration_runtime.js", (101, true)),
    ];
    let errors: Vec<String> = cases
        .iter()
        .filter_map(|&(c, restore)| {
            let mut schedules = slices.to_vec();
            schedules.push(restore);
            check(&repo().join(c), &schedules).err()
        })
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// Workloads whose fuel used to depend on where the GC ran (append buffers, D1610; JSON;
/// Array.from: 0074) or on how much fuel a slice had left (regexp backreferences, D1711).
#[test]
fn fuel_independent_of_slice() {
    let cases = [
        (
            "append",
            r#"let s = ""; let g = 0; const keep = [];
               for (let i = 0; i < 30000; i++) {
                 s += "abcdefgh" + (i % 10);
                 const o = { a: i, b: [i, i] }; g += o.b.length;
                 if (i % 5000 == 4999) { keep.push(s); s = s.slice(0, 100); }
               }
               console.log(s.length, g, keep.length);"#,
        ),
        (
            "json",
            r#"const o = []; for (let i = 0; i < 3000; i++) o.push({ i, s: "v" + i, a: [i, i * 2] });
               let n = 0; for (let k = 0; k < 4; k++) n += JSON.parse(JSON.stringify(o)).length;
               console.log(n);"#,
        ),
        (
            "array-from",
            r#"let n = 0; for (let k = 0; k < 40; k++) {
                 n += Array.from({ length: 2000 }, (_, i) => i * k).length;
                 n += Array.from(new Set(Array.from("abcdefghij".repeat(50)))).length;
               }
               console.log(n);"#,
        ),
        (
            "backref",
            r#"const s = "a".repeat(2000) + "b"; let n = 0;
               for (let i = 0; i < 3; i++) { n += /(a{500})\1/.exec(s)[0].length; n += /(a+)\1b/.test(s) ? 1 : 0; }
               console.log(n);"#,
        ),
    ];
    // Restores copy the image at every stop: at a larger slice, to stay fast.
    let schedules = [(997, false), (101, false), (20_011, true), (1, false)];
    let errors: Vec<String> = cases
        .iter()
        .filter_map(|(name, src)| check_src(name, src, &schedules).err())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// Every e2e case at a few slices (slow; `cargo test --test slices -- --ignored`).
#[test]
#[ignore]
fn raw_images_independent_of_slice_all_e2e() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(repo().join("tests/data/e2e"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "js"))
        .collect();
    paths.sort();
    if let Ok(extra) = std::env::var("DJS_SLICES_CASES") {
        paths = extra.split(',').map(PathBuf::from).collect();
    }
    let schedules = [(997, false), (101, true), (13, false), (1, false)];
    let errors: Vec<String> = paths
        .iter()
        .filter_map(|p| check(p, &schedules).err())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
