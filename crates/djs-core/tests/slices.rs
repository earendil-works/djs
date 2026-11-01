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

/// Runs `src` in slices of `slice` fuel; returns its output and the final raw image.
fn run(src: &str, slice: u64, restore: bool) -> (String, Vec<u8>) {
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
    loop {
        match vm.run(slice) {
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
    (out, vm.bytes().to_vec())
}

/// Every schedule of `path` against all the fuel at once: the same output and arena.
fn check(path: &Path, schedules: &[(u64, bool)]) -> Result<(), String> {
    let src = std::fs::read_to_string(path).unwrap();
    let (want_out, want_raw) = run(&src, u64::MAX, false);
    for &(slice, restore) in schedules {
        let (out, raw) = run(&src, slice, restore);
        if out != want_out {
            return Err(format!(
                "{}: slice {slice} restore {restore}: output differs",
                path.display()
            ));
        }
        if raw != want_raw {
            let at = raw.iter().zip(&want_raw).position(|(a, b)| a != b);
            return Err(format!(
                "{}: slice {slice} restore {restore}: raw image differs (len {} vs {}, first at {at:?})",
                path.display(),
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
    let schedules = [(997, false), (101, true), (13, false), (1, false)];
    let errors: Vec<String> = paths
        .iter()
        .filter_map(|p| check(p, &schedules).err())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
