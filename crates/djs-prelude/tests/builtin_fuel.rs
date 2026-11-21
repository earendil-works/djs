//! Builtins charge fuel for the work they do (0289, GitHub issue #3): an operation over
//! an input of `n` elements, code units or keys costs fuel that grows with `n`. A builtin
//! that charges a flat amount lets one call of a one-line script run for seconds per
//! slice of fuel. The table is broad on purpose: a new builtin that does its work
//! without charging for it (outside the heap's primitives, `heap/work.rs`) fails here.
//!
//! Each case's setup builds its input (flat strings: a rope's first flattening would
//! be the only work a flat builtin is charged for); only the call of the body is
//! measured. The results are checked, so that a declining or failing fast path cannot
//! pass as a cheap one.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const SMALL: u32 = 60;
const LARGE: u32 = 6_000;
const BUDGET: u64 = 100_000_000;

/// The least fuel per element of growth: well below the rates of the primitives (one
/// unit per 16 code units, per 4 elements, per 32 bytes allocated), so that the test
/// pins no rate, only that there is one.
const PER_FUEL: u32 = 32;

/// `(name, setup, body)`: the setup's `n` is the input size, the body an expression
/// whose value (a string, number or boolean) is compared against the same expression
/// computed in a fresh VM after the measured call.
const CASES: &[(&str, &str, &str)] = &[
    // ---- the reporter's (GitHub issue #3) ----
    ("split('')", STR, "s.split('').length"),
    ("join('-')", CHARS, "a.join('-').length"),
    ("string indexOf miss", STR, "s.indexOf('!')"),
    ("array indexOf miss", CHARS, "a.indexOf('!')"),
    ("Object.keys", OBJ, "Object.keys(o).length"),
    (
        "JSON.parse",
        "const j = '[' + '0,'.repeat(n - 1) + '0]';",
        "JSON.parse(j).length",
    ),
    ("JSON.stringify", NUMS, "JSON.stringify(na).length"),
    ("regexp replace", STR, "s.replace(/[abc]/g, 'xx').length"),
    // ---- strings ----
    ("split(sep)", STR, "s.split('b').length"),
    ("split(two units)", STR, "s.split('ca').length"),
    ("lastIndexOf miss", STR, "s.lastIndexOf('!')"),
    ("includes miss", STR, "s.includes('!')"),
    ("indexOf two-byte", STR16, "w.indexOf('!')"),
    ("indexOf long needle", STR, "s.indexOf('abcabcabcabd')"),
    ("search", STR, "s.search('!')"),
    ("toUpperCase", STR, "s.toUpperCase().length"),
    ("toLowerCase unchanged", STR, "s.toLowerCase().length"),
    ("toUpperCase two-byte", STR16, "w.toUpperCase().length"),
    ("padStart", STR, "s.padStart(2 * n).length"),
    ("padEnd", "const d = String(n);", "d.padEnd(n).length"),
    (
        "trim",
        "const sp = ' '.repeat(n) + 'x' + ' '.repeat(n); sp.indexOf('!');",
        "sp.trim().length",
    ),
    ("concat flattened", STR, "(s + s).indexOf('!')"),
    ("replaceAll string", STR, "s.replaceAll('a', 'b').length"),
    ("replace string miss", STR, "s.replace('!', 'b').length"),
    ("normalize", STR, "s.normalize().length"),
    ("localeCompare", STR2, "s.localeCompare(t)"),
    ("=== equal", STR2, "s === t"),
    ("!== equal", STR2, "s !== t"),
    ("< equal prefix", STR2, "s < t + 'x'"),
    ("String(arr)", NUMS, "String(na).length"),
    ("toString of an array", NUMS, "na.toString().length"),
    ("parseInt", DIGITS, "parseInt(ds) > 0"),
    ("Number", DIGITS, "Number(ds) > 0"),
    ("unary plus", DIGITS, "+ds > 0"),
    ("btoa", STR, "btoa(s).length"),
    (
        "atob",
        "const b64 = btoa('abc'.repeat(n / 3)); b64.indexOf('!');",
        "atob(b64).length",
    ),
    ("encodeURIComponent", STR, "encodeURIComponent(s).length"),
    (
        "sort strings",
        "const ws = Array.from({length: n}, (_, i) => 'w' + (i * 7919 % n));",
        "ws.toSorted()[0]",
    ),
    // ---- arrays ----
    ("join()", CHARS, "a.join().length"),
    ("join() numbers", NUMS, "na.join().length"),
    (
        "join in a loop",
        CHARS,
        "{ let t = 0; for (let i = 0; i < 10; i++) t += a.join('-').length; return t; }",
    ),
    ("includes miss", CHARS, "a.includes('!')"),
    ("indexOf numbers", NUMS, "na.indexOf(-1)"),
    ("lastIndexOf", CHARS, "a.lastIndexOf('!')"),
    ("slice", CHARS, "a.slice().length"),
    ("slice.call", CHARS, "Array.prototype.slice.call(a).length"),
    ("splice", CHARS, "a.splice(0, 0, 1).length"),
    ("shift", CHARS, "a.shift()"),
    ("unshift", CHARS, "a.unshift(1)"),
    ("length = 0", CHARS, "{ a.length = 0; return a.length; }"),
    ("spread", CHARS, "[...a].length"),
    ("concat", CHARS, "a.concat(a).length"),
    ("Array.from", CHARS, "Array.from(a).length"),
    ("fill", CHARS, "a.fill(0).length"),
    ("reverse", CHARS, "a.reverse().length"),
    ("toSorted", CHARS, "a.toSorted().length"),
    ("flat", CHARS, "a.flat().length"),
    // ---- objects ----
    ("Object.values", OBJ, "Object.values(o).length"),
    ("Object.entries", OBJ, "Object.entries(o).length"),
    (
        "getOwnPropertyNames",
        OBJ,
        "Object.getOwnPropertyNames(o).length",
    ),
    ("Reflect.ownKeys", OBJ, "Reflect.ownKeys(o).length"),
    (
        "Object.assign",
        OBJ,
        "Object.keys(Object.assign({}, o)).length",
    ),
    ("object spread", OBJ, "({...o}).k0"),
    ("Object.freeze", OBJ, "Object.isFrozen(Object.freeze(o))"),
    (
        "Object.isFrozen",
        "const o = {}; for (let i = 0; i < n; i++) o['k' + i] = i; Object.freeze(o);",
        "Object.isFrozen(o)",
    ),
    (
        "for in",
        OBJ,
        "{ let c = 0; for (const k in o) c++; return c; }",
    ),
    (
        "fromEntries",
        OBJ,
        "Object.keys(Object.fromEntries(Object.entries(o))).length",
    ),
    // ---- collections ----
    ("new Set(numbers)", NUMS, "new Set(na).size"),
    ("new Set(repeats)", CHARS, "new Set(a).size"),
    ("new Map(entries)", OBJ, "new Map(Object.entries(o)).size"),
    (
        "Map.get long key",
        "const k = 'x'.repeat(n); k.indexOf('!'); const m = new Map([[k, 1]]);",
        "m.get(k.slice(1) + 'x')",
    ),
    (
        "long key",
        "const k = 'x'.repeat(n); k.indexOf('!'); const ko = {[k]: 1};",
        "ko[k.slice(1) + 'x']",
    ),
    // ---- regexps ----
    ("regexp test miss", STR, "/!/.test(s)"),
    ("matchAll", STR, "[...s.matchAll(/c/g)].length"),
];

/// Cases of the full build's builtins (typed arrays, BigInt: D2411).
#[cfg(feature = "full")]
const FULL_CASES: &[(&str, &str, &str)] = &[
    ("new Uint8Array(n)", "", "new Uint8Array(n).length"),
    ("Uint8Array from array", NUMS, "new Uint8Array(na).length"),
    (
        "Uint8Array slice",
        "const u = new Uint8Array(n);",
        "u.slice().length",
    ),
    ("BigInt of a string", DIGITS, "BigInt(ds) > 0n"),
    ("TextEncoder", STR, "new TextEncoder().encode(s).length"),
];

const STR: &str = "const s = 'abc'.repeat(n / 3); s.indexOf('!');";
const STR2: &str = "const s = 'abc'.repeat(n / 3); const t = 'abc'.repeat(n / 3); \
                    s.indexOf('!'); t.indexOf('!');";
const STR16: &str = "const w = 'ab\\u0101'.repeat(n / 3); w.indexOf('!');";
const DIGITS: &str = "const ds = '1'.repeat(n); ds.indexOf('!');";
const CHARS: &str = "const a = 'abc'.repeat(n / 3).split('');";
const NUMS: &str = "const na = Array.from({length: n}, (_, i) => i);";
const OBJ: &str = "const o = {}; for (let i = 0; i < n; i++) o['k' + i] = i;";

fn done(vm: &mut Vm, status: Status, what: &str) -> Value {
    match status {
        Status::Done(value) => value,
        Status::Threw(error) => panic!("{what}: unexpected exception: {}", vm.display(error)),
        other => panic!("{what}: expected completion within {BUDGET} fuel, got {other:?}"),
    }
}

/// The fuel of one call of `body` after `setup`, and its result as displayed. With
/// `hot`, the tier-0 loop runs it (as always outside tests); without, the run loop's
/// steps and `@fast` calls (the reference the loop must equal).
fn measure(name: &str, setup: &str, body: &str, n: u32, hot: bool) -> (u64, String) {
    let opts = VmOptions {
        exec: ExecOptions {
            hot_loop: hot,
            ..ExecOptions::default()
        },
        ..VmOptions::default()
    };
    let mut vm = djs_prelude::new_vm(opts).unwrap();
    let body = if body.starts_with('{') {
        body.to_string()
    } else {
        format!("return {body};")
    };
    // The script's completion value is a closure over the input it built.
    let source =
        format!("(function() {{ const n = {n}; {setup} return function() {{ {body} }}; }})()");
    let main = vm
        .compile(
            source.as_bytes(),
            &CompileOptions::script("builtin_fuel.js"),
        )
        .unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    let status = vm.run(BUDGET);
    let probe = done(&mut vm, status, name);
    vm.call(probe, Value::UNDEFINED, &[]).unwrap();
    let (status, used) = vm.run_metered(BUDGET);
    let result = done(&mut vm, status, name);
    (used, vm.display(result))
}

fn check(cases: &[(&str, &str, &str)]) {
    let mut flat = Vec::new();
    for &(name, setup, body) in cases {
        let (small, _) = measure(name, setup, body, SMALL, true);
        let (large, result) = measure(name, setup, body, LARGE, true);
        // Without the tier-0 loop: the same result and fuel (the loop is invisible).
        assert_eq!(
            measure(name, setup, body, LARGE, false),
            (large, result.clone()),
            "{name}"
        );
        eprintln!("{name:<28} {SMALL} => {small:>7}   {LARGE} => {large:>7}   ({result})");
        if large < small + u64::from((LARGE - SMALL) / PER_FUEL) {
            flat.push(format!("{name}: {small} => {large}"));
        }
    }
    assert!(
        flat.is_empty(),
        "builtins that do not charge for growing work:\n{}",
        flat.join("\n")
    );
}

#[test]
fn builtin_fuel_scales_with_input() {
    check(CASES);
}

#[cfg(feature = "full")]
#[test]
fn full_builtin_fuel_scales_with_input() {
    check(FULL_CASES);
}

/// The reporter's worst case (`new Set` of a 30,000-element array: a million units of
/// fuel took 128 s): one call costs at least a quarter unit per element, so a slice of a
/// million units makes at most 134 such calls.
#[test]
fn new_set_pays_per_element() {
    let (fuel, size) = measure("new Set", NUMS, "new Set(na).size", 30_000, true);
    assert_eq!(size, "30000");
    assert!(
        fuel >= 30_000 / 4,
        "new Set of 30,000 elements: {fuel} fuel"
    );
}

/// The wall-clock time of one slice of a million units of fuel spent on each operation
/// in a loop, against the tight loop's (diagnostic: `cargo test --release -p djs-prelude
/// --test builtin_fuel -- --ignored --nocapture`; the 0289 log has the numbers).
#[test]
#[ignore = "diagnostic wall-clock table; run in release with --ignored --nocapture"]
fn slice_wall_clock() {
    for (name, src) in [
        ("tight loop", "let i = 0; for (;;) i++;"),
        (
            "join 30k",
            "const a = 'abc'.repeat(10000).split(''); for (;;) a.join('-');",
        ),
        (
            "indexOf 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.indexOf('!');",
        ),
        (
            "Object.keys 30k",
            "const o = {}; for (let i = 0; i < 30000; i++) o['k'+i] = i; for (;;) Object.keys(o);",
        ),
        (
            "new Set 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) new Set(a);",
        ),
        ("repeat 1M", "for (;;) 'abc'.repeat(333333);"),
        (
            "split 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.split('');",
        ),
        (
            "split sep 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.split('b');",
        ),
        (
            "new Set str 30k",
            "const a = 'abc'.repeat(10000).split(''); for (;;) new Set(a);",
        ),
        (
            "includes 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) a.includes(-1);",
        ),
        (
            "=== 1M",
            "const s = 'abc'.repeat(333333), t = 'abc'.repeat(333333); s.indexOf('!'); t.indexOf('!'); for (;;) s === t;",
        ),
        (
            "< 1M",
            "const s = 'abc'.repeat(333333), t = 'abc'.repeat(333333); s.indexOf('!'); t.indexOf('!'); for (;;) s < t;",
        ),
        (
            "localeCompare 1M",
            "const s = 'abc'.repeat(333333), t = 'abc'.repeat(333333); for (;;) s.localeCompare(t);",
        ),
        (
            "toLowerCase 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.toLowerCase();",
        ),
        (
            "toUpperCase 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.toUpperCase();",
        ),
        (
            "normalize 1M",
            "const s = 'abc'.repeat(333333); for (;;) s.normalize();",
        ),
        (
            "parseInt 1M",
            "const s = '1'.repeat(1000000); for (;;) parseInt(s);",
        ),
        (
            "Number 1M",
            "const s = '1'.repeat(1000000); for (;;) Number(s);",
        ),
        (
            "shift 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) { a.shift(); a.push(1); }",
        ),
        (
            "unshift 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) { a.unshift(1); a.pop(); }",
        ),
        (
            "slice 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) a.slice();",
        ),
        (
            "spread 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) [...a];",
        ),
        (
            "obj spread 30k",
            "const o = {}; for (let i = 0; i < 30000; i++) o['k'+i] = i; for (;;) ({...o});",
        ),
        (
            "freeze 30k",
            "const o = {}; for (let i = 0; i < 30000; i++) o['k'+i] = i; for (;;) Object.isFrozen(Object.freeze({...o}));",
        ),
        (
            "length=0 30k",
            "const b = Array.from({length:30000}, (_, i) => i); for (;;) { const a = b.slice(); a.length = 0; }",
        ),
        (
            "sort str 3k",
            "const a = Array.from({length:3000}, (_, i) => 'x'.repeat(300) + (i * 7919 % 3000)); for (;;) a.toSorted();",
        ),
        (
            "Map.get long key",
            "const k = 'x'.repeat(1000000); const m = new Map([[k, 1]]); for (;;) m.get(k.slice(1));",
        ),
        (
            "long key",
            "const k = 'x'.repeat(1000000); const o = {}; for (;;) o[k.slice(1)] = 1;",
        ),
        ("Uint8Array 1M", "for (;;) new Uint8Array(1000000);"),
        (
            "join num 30k",
            "const a = Array.from({length:30000}, (_, i) => i); for (;;) a.join();",
        ),
    ] {
        let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
        let f = vm
            .compile(src.as_bytes(), &CompileOptions::script("s.js"))
            .unwrap();
        vm.call(f, Value::UNDEFINED, &[]).unwrap();
        // (The setup, and the first flattening of a repeat.)
        let _ = vm.run(300_000);
        let t = std::time::Instant::now();
        let st = vm.run(1_000_000);
        eprintln!("{name:<20} {:>12.3?} ({st:?})", t.elapsed());
    }
}
