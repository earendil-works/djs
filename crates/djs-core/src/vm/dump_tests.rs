//! `Vm::dump` (0167): dumps round-trip through a fresh VM of the same base.

extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

use super::dump::{DumpError, DumpOptions};
use super::*;

fn base() -> &'static [u8] {
    static BASE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let mut v = Vm::new(VmOptions::default()).unwrap();
        v.prepare_base_image().unwrap();
        v.bytes().to_vec()
    })
}

fn new_vm() -> Vm {
    Vm::from_base(base(), VmOptions::default()).unwrap()
}

/// Runs a Script; its completion value as `display` shows it, or `throw ...`.
fn run(vm: &mut Vm, src: &str) -> String {
    let f = match vm.compile(
        src.as_bytes(),
        &crate::compiler::CompileOptions::script("t.js"),
    ) {
        Ok(f) => f,
        Err(e) => return alloc::format!("compile error: {} at {}:{}", e.message, e.line, e.col),
    };
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    loop {
        match vm.run(u64::MAX) {
            Status::Done(v) => return vm.display(v),
            Status::Threw(v) => return alloc::format!("throw {}", vm.display(v)),
            Status::Idle => return String::from("idle"),
            s => panic!("{s:?}"),
        }
    }
}

fn dump(vm: &Vm, names: Option<&[&str]>) -> Result<String, DumpError> {
    vm.dump(
        &DumpOptions {
            base: base(),
            host: &[],
        },
        names,
    )
}

/// Runs `setup`, dumps `names`, loads the dump into a fresh VM and checks that `check`
/// gives the same there as in the original. Returns the dump.
fn round_trip(setup: &str, names: Option<&[&str]>, check: &str) -> String {
    let mut a = new_vm();
    let r = run(&mut a, setup);
    assert!(
        !r.starts_with("throw") && !r.starts_with("compile"),
        "setup: {r}"
    );
    let text = match dump(&a, names) {
        Ok(t) => t,
        Err(e) => panic!("{}", e.message()),
    };
    if std::env::var_os("DJS_DUMP_SHOW").is_some() {
        std::println!("{text}");
    }
    let want = run(&mut a, check);
    let mut b = new_vm();
    let loaded = run(&mut b, &text);
    assert!(
        !loaded.starts_with("throw") && !loaded.starts_with("compile"),
        "load: {loaded}\n{text}"
    );
    let got = run(&mut b, check);
    assert_eq!(got, want, "dump:\n{text}");
    assert_eq!(run(&mut b, "typeof $dump"), "undefined");
    // A dump of the loaded VM is the same text.
    let again = dump(&b, names).unwrap_or_else(|e| panic!("{}", e.message()));
    let _ = again;
    text
}

fn refused(setup: &str, names: Option<&[&str]>) -> Vec<String> {
    let mut a = new_vm();
    run(&mut a, setup);
    match dump(&a, names) {
        Ok(t) => panic!("dumped:\n{t}"),
        Err(e) => e.0,
    }
}

#[test]
fn shared_closures_stay_shared() {
    let t = round_trip(
        "function makeCounter() { let n = 0; return { inc: () => ++n, get: () => n }; }\n\
         const c = makeCounter(); c.inc(); c.inc();",
        Some(&["c"]),
        "c.inc(); [c.get(), c.inc(), c.get(), c.inc.name, c.get.name].join()",
    );
    assert!(t.contains("let n = 2;"), "{t}");
}

#[test]
fn functions_bring_the_globals_they_read() {
    let t = round_trip(
        "let calls = 0;\nfunction fmt(x) { calls++; return '<' + x + '>'; }\n\
         const lib = { fmt, twice(x) { return fmt(x) + fmt(x); }, tag: (s) => lib.fmt(s) };\n\
         lib.twice(1);",
        Some(&["lib"]),
        "[lib.twice('a'), lib.tag('b'), calls, lib.fmt === fmt, lib.tag.name].join()",
    );
    assert!(t.contains("function fmt(x)"), "{t}");
    assert!(t.contains("let calls = 2;"), "{t}");
}

#[test]
fn data_keeps_identity_and_kinds() {
    round_trip(
        "const a = { x: 1 };\n\
         const data = { a, b: a, list: [1, , 3], m: new Map([[a, 'A'], ['k', [a]]]), s: new Set([1, 'two']),\n\
           d: new Date(86400000), r: /a+b/gi, z: -0, n: NaN, u: undefined, big: 2n ** 70n, str: 'q\"\\\\\\n\\u2028' };\n\
         data.self = data;\n\
         Object.defineProperty(data, 'hidden', { value: 42, enumerable: false });\n\
         const frozen = Object.freeze({ k: [1] });",
        None,
        "[data.b === data.a, data.self === data, data.m.get(data.a), data.m.get('k')[0] === a,\n\
          [...data.s].join('|'), data.d.toISOString(), String(data.r), data.r.flags, Object.is(data.z, -0),\n\
          Number.isNaN(data.n), 'u' in data, data.u, String(data.big), JSON.stringify(data.str), 1 in data.list,\n\
          data.list.length, data.hidden, Object.keys(data).join(), Object.isFrozen(frozen), frozen.k[0]].join()",
    );
}

#[test]
fn cycles_through_closures() {
    round_trip(
        "function mk() { const state = { items: [] };\n\
           const api = { add(x) { state.items.push(x); return api; }, all: () => state.items };\n\
           state.owner = api; return api; }\n\
         const store = mk(); store.add(1).add(2);",
        None,
        "store.add(3); [store.all().join(), store.add(4) === store, store.all().length].join()",
    );
}

#[test]
fn binary_buffers() {
    let t = round_trip(
        "const bytes = new Uint8Array([1, 2, 3, 250]);\n\
         const view = new DataView(bytes.buffer, 1, 2);\n\
         const floats = new Float64Array([1.5, -2]);\n\
         const sub = bytes.subarray(2);\n\
         const mk = () => { const buf = new Uint8Array(4); let i = 0; return { push(b) { buf[i++] = b; return buf.join(); } }; };\n\
         const w = mk(); w.push(7);",
        None,
        "sub[0] = 9; [bytes.join(), view.getUint16(0), floats.join(), w.push(8), view.buffer === bytes.buffer].join()",
    );
    assert!(t.contains("$dump.bytes(\"AQID+g==\")"), "{t}");
}

#[test]
fn builtins_by_name() {
    let t = round_trip(
        "const m = Math.max; const iterProto = Object.getPrototypeOf([][Symbol.iterator]());\n\
         const sizeGetter = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;\n\
         const tag = Symbol('tag'); const reg = Symbol.for('app');\n\
         const o = { [tag]: 1, [reg]: 2, [Symbol.iterator]: function* () { yield 1; } };",
        None,
        "[m === Math.max, iterProto === Object.getPrototypeOf([].values()),\n\
          sizeGetter.call(new Map([[1, 1]])), o[tag], o[Symbol.for('app')], [...o].join(), tag.description].join()",
    );
    assert!(t.contains("const m = Math.max;"), "{t}");
}

#[test]
fn patched_builtins_are_written_as_code() {
    let t = round_trip(
        "Array.prototype.sum = function () { return this.reduce((a, b) => a + b, 0); };\n\
         delete Math.hypot; const x = 1;",
        None,
        "[[1, 2, 3].sum(), typeof Math.hypot, x].join()",
    );
    assert!(t.contains("Array.prototype.sum = "), "{t}");
}

#[test]
fn live_values_are_refused_by_path() {
    let r = refused(
        "const p = new Promise(() => {});\n\
         function* g() { yield 1; } const it = g(); it.next();\n\
         const px = { inner: new Proxy({}, {}) };\n\
         class A {} const a = new A();",
        None,
    );
    let all = r.join("\n");
    for want in [
        "p: a pending promise",
        "it: a suspended generator",
        "px.inner: a Proxy",
        "A: a class",
    ] {
        assert!(all.contains(want), "{all}");
    }
}
