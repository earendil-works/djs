//! Regex matching is fuel-metered and suspendable (0033, D1332): a catastrophic pattern
//! runs out of fuel instead of hanging, and a match suspended mid-way continues in a VM
//! restored from the heap image with the same result.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

/// Starts the expression `src` in a fresh VM; the result goes into `globalThis.result`.
fn start(src: &str) -> Vm {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let src = format!("globalThis.result = {src};");
    let main = vm
        .compile(src.as_bytes(), &CompileOptions::script("regexp_fuel.js"))
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(main, this, &[]).unwrap();
    vm
}

/// Runs to completion with `fuel` per slice; with `restore`, continues in a VM restored
/// from the heap image at every stop. Returns the displayed result and the stops.
fn run_sliced(src: &str, fuel: u64, restore: bool) -> (String, u64) {
    let mut vm = start(src);
    let mut stops = 0;
    loop {
        match vm.run(fuel) {
            Status::Done(_) => {
                let key = vm.heap_mut().atom_str("result").unwrap();
                let (_, p) = vm.heap().find_property(vm.global(), key).unwrap();
                return (vm.display(p.value().unwrap()), stops);
            }
            Status::Threw(e) => return (format!("threw {}", vm.display(e)), stops),
            Status::OutOfFuel => {
                stops += 1;
                if restore {
                    let o = vm.options();
                    vm = Vm::from_bytes(vm.bytes().to_vec(), o).unwrap();
                }
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn catastrophic_pattern_runs_out_of_fuel() {
    // 2^30 backtracking steps: far beyond the budget, but no hang.
    let mut vm = start(r#"/(a*)*b/.test("a".repeat(30))"#);
    assert_eq!(vm.run(5_000_000), Status::OutOfFuel);
    // Still suspended inside the match: more fuel keeps it going.
    assert_eq!(vm.run(1_000_000), Status::OutOfFuel);
}

#[test]
fn suspended_match_resumes_from_the_image() {
    for src in [
        r#"/(a*)*b/.test("a".repeat(14))"#,
        r#"/(a|ab)*c$/.exec("ab".repeat(12) + "c").index"#,
        r#"JSON.stringify("xaaay aay".replace(/(a+)(?=y)/g, "<$1>"))"#,
        r#"JSON.stringify(/(?<x>b+)\1/d.exec("abbbbbbc"))"#,
    ] {
        let (plain, _) = run_sliced(src, u64::MAX, false);
        let (sliced, stops) = run_sliced(src, 97, false);
        // Suspending through heap images changes nothing, not even the fuel spent.
        let (restored, restored_stops) = run_sliced(src, 97, true);
        assert_eq!(plain, sliced, "{src}");
        assert_eq!(plain, restored, "{src}");
        assert_eq!(stops, restored_stops, "{src}");
    }
    // The match is nearly all of the work, so nearly all of these stops are inside it.
    let (r, stops) = run_sliced(r#"/(a*)*b/.test("a".repeat(14))"#, 97, true);
    assert_eq!(r, "false");
    assert!(stops > 1000, "{stops} stops");
}
