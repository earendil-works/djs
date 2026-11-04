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
    run_sliced_every(src, fuel, if restore { 1 } else { 0 })
}

/// [`run_sliced`], restoring from the image at every `restore_every`-th stop (never for 0).
fn run_sliced_every(src: &str, fuel: u64, restore_every: u64) -> (String, u64) {
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
                if restore_every != 0 && stops % restore_every == 1 % restore_every {
                    let o = vm.options();
                    vm = Vm::from_bytes(vm.image().to_vec(), o).unwrap();
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

const RESUME_CASES: [&str; 7] = [
    r#"/(a*)*b/.test("a".repeat(14))"#,
    r#"/(a|ab)*c$/.exec("ab".repeat(12) + "c").index"#,
    r#"JSON.stringify("xaaay aay".replace(/(a+)(?=y)/g, "<$1>"))"#,
    r#"JSON.stringify(/(?<x>b+)\1/d.exec("abbbbbbc"))"#,
    // The split and replace leaves (D1503) suspend between and inside matches.
    r#"JSON.stringify("a1b22c333d".repeat(20).split(/(\d)\d*/))"#,
    r#""ab".repeat(60).replace(/(a)(b)/g, "$2$1-$&")"#,
    r#"JSON.stringify("x".repeat(40).split(/(?:)/))"#,
];

/// Every case suspended every 97 fuel, and restored from the image at every
/// `restore_every`-th stop: the same result and the same number of stops (fuel).
fn resumes_from_the_image(restore_every: u64) {
    for src in RESUME_CASES {
        let (plain, _) = run_sliced(src, u64::MAX, false);
        let (sliced, stops) = run_sliced(src, 97, false);
        // Suspending through heap images changes nothing, not even the fuel spent.
        let (restored, restored_stops) = run_sliced_every(src, 97, restore_every);
        assert_eq!(plain, sliced, "{src}");
        assert_eq!(plain, restored, "{src}");
        assert_eq!(stops, restored_stops, "{src}");
    }
    // The match is nearly all of the work, so nearly all of these stops are inside it.
    let (r, stops) = run_sliced(RESUME_CASES[0], 97, false);
    assert_eq!(r, "false");
    assert!(stops > 1000, "{stops} stops");
}

/// Restores at every 13th stop (the first one included): each image round trip costs a
/// copy of the whole heap, and every stop was 6 s per gate.
#[test]
fn suspended_match_resumes_from_the_image() {
    resumes_from_the_image(13);
}

/// Every stop restored (`cargo test -p djs-core --test it regexp_fuel:: -- --ignored`).
#[test]
#[ignore]
fn suspended_match_resumes_from_the_image_at_every_stop() {
    resumes_from_the_image(1);
}

#[test]
fn base64_check_on_megabytes() {
    // pi's image(): `/^[A-Za-z0-9+/]+={0,2}$/` on 5 MB of base64 (D1504).
    let src = r#"(() => { const s = "QUJD".repeat(1310720); const re = /^[A-Za-z0-9+/]+={0,2}$/; return [re.test(s + "=="), re.test(s + "==!"), re.test("")].join() })()"#;
    let (r, _) = run_sliced(src, u64::MAX, false);
    assert_eq!(r, "true,false,false");
    // Suspended mid-match and restored from images, in large slices.
    let (r, stops) = run_sliced(src, 20_000_000, true);
    assert_eq!(r, "true,false,false");
    assert!(stops > 0);
}
