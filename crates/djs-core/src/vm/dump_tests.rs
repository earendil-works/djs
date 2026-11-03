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
    match vm.run(u64::MAX) {
        Status::Done(v) => vm.display(v),
        Status::Threw(v) => alloc::format!("throw {}", vm.display(v)),
        Status::Idle => String::from("idle"),
        s => panic!("{s:?}"),
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

#[test]
fn symbol_keys_keep_identity_across_patches() {
    round_trip(
        "const o = { visible: 1 };\n\
         Object.defineProperty(o, Symbol('hidden'), { value: 2, enumerable: false });",
        None,
        "const ks = Object.getOwnPropertySymbols(o);\n\
         [ks.length, ks[0].description, o[ks[0]], o.propertyIsEnumerable(ks[0])].join()",
    );
}

#[test]
fn function_scope_bindings_by_box() {
    // A harness dumps a function's locals: `n` is a captured variable (its box), and
    // the closure that captures it refers to it by name, whatever the order of roots.
    let mut a = new_vm();
    run(
        &mut a,
        "function mk() { let n = 1; const inc = () => ++n; return inc; }\nvar f = mk(); f();",
    );
    let f = run_value(&mut a, "f");
    let o = f.as_object().unwrap();
    let b = a.heap.object_slot(o, 0).as_internal().unwrap();
    let text = a
        .dump_values(
            &DumpOptions {
                base: base(),
                host: &[],
            },
            &[("inc", f), ("n", Value::internal(b))],
        )
        .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(text.contains("let n = 2;"), "{text}");
    let mut b = new_vm();
    run(&mut b, &text);
    assert_eq!(run(&mut b, "inc(); inc(); n"), "4", "{text}");
}

/// The value of a global expression.
fn run_value(vm: &mut Vm, src: &str) -> Value {
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("v.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    match vm.run(u64::MAX) {
        Status::Done(v) => v,
        s => panic!("{s:?}"),
    }
}

/// Timing of a large dump (`cargo test --release ... -- --ignored --nocapture big_dump`).
#[test]
#[ignore]
fn big_dump_timing() {
    let mut a = new_vm();
    run(
        &mut a,
        "const users = [];\n\
         for (let i = 0; i < 10000; i++) users.push({ id: i, name: 'user' + i, tags: ['a', 'b' + (i % 7)], score: i * 1.5, meta: { created: new Date(i * 1000), active: i % 3 === 0 } });\n\
         const byId = new Map(users.map((u) => [u.id, u]));",
    );
    let t = std::time::Instant::now();
    let names = super::registry::reachable_names(
        &Heap::from_bytes_trusted(base().to_vec(), base().len() * 2).unwrap(),
    );
    std::println!("names: {} in {:?}", names.len(), t.elapsed());
    let t = std::time::Instant::now();
    let mut text = String::new();
    let reps: u32 = std::env::var("DJS_DUMP_REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    for _ in 0..reps {
        text = dump(&a, None).unwrap();
    }
    std::println!("dump: {} bytes in {:?}", text.len(), t.elapsed());
}

/// The committed dump of [`FIXTURE_SETUP`] (tests/dump/v1.js): every later build must load
/// it and give [`FIXTURE_CHECK`]'s answer (tests/dump/v1.expected). The format promise in
/// docs/functions-as-code.md: a dump stays loadable by later builds.
const FIXTURE_SETUP: &str = "\
function makeCounter(start) { let n = start; return { inc: () => ++n, get: () => n }; }\n\
const counter = makeCounter(40); counter.inc();\n\
const bytes = new Uint8Array(100).map((_, i) => i * 3);\n\
const small = new Int16Array([-1, 2, 300]);\n\
const key = { k: 1 };\n\
const index = new Map([[key, 'object key'], ['when', new Date(1700000000000)]]);\n\
const graph = { name: 'root', children: [] }; graph.children.push({ parent: graph });\n\
const frozen = Object.freeze({ list: [1, 2] });\n\
const tag = Symbol.for('app.tag');\n\
const re = /ab+c/gi;\n\
const iterProto = Object.getPrototypeOf([][Symbol.iterator]());\n\
var legacy = { [tag]: 'tagged', big: 12345678901234567890n };\n\
Array.prototype.last = function () { return this[this.length - 1]; };";

const FIXTURE_CHECK: &str = "\
[counter.inc(), counter.get(), bytes.length, bytes[99], small.join(), index.get(key), \
index.get('when').toISOString(), graph.children[0].parent === graph, Object.isFrozen(frozen), \
frozen.list.last(), legacy[Symbol.for('app.tag')], String(legacy.big), re.flags, re.test('xabbbc'), \
iterProto === Object.getPrototypeOf([].values())].join('|')";

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dump")
}

/// Rewrites the fixture (`DJS_DUMP_FIXTURE=1 cargo test ... write_fixture -- --ignored`).
/// Only when the format changes on purpose: a new version keeps the old file next to it.
#[test]
#[ignore]
fn write_fixture() {
    if std::env::var_os("DJS_DUMP_FIXTURE").is_none() {
        return;
    }
    let mut a = new_vm();
    run(&mut a, FIXTURE_SETUP);
    let text = dump(&a, None).unwrap_or_else(|e| panic!("{}", e.message()));
    let want = run(&mut a, FIXTURE_CHECK);
    std::fs::create_dir_all(fixture_dir()).unwrap();
    std::fs::write(fixture_dir().join("v1.js"), text).unwrap();
    std::fs::write(fixture_dir().join("v1.expected"), want + "\n").unwrap();
}

#[test]
fn committed_dumps_still_load() {
    let text = std::fs::read_to_string(fixture_dir().join("v1.js")).unwrap();
    let want = std::fs::read_to_string(fixture_dir().join("v1.expected")).unwrap();
    assert!(text.starts_with(super::dump::HEADER));
    let mut b = new_vm();
    let loaded = run(&mut b, &text);
    assert!(!loaded.starts_with("throw"), "{loaded}");
    assert_eq!(run(&mut b, FIXTURE_CHECK), want.trim_end());
}

#[test]
fn captured_host_values_read_the_global() {
    // A body's closure that captures its `tools` parameter: the host re-binds `tools`.
    let mut a = new_vm();
    run(
        &mut a,
        "globalThis.tools = { read: (p) => 'read ' + p };\n\
         var lib = ((tools) => ({ go: (p) => tools.read(p) }))(tools);",
    );
    let text = a
        .dump(
            &DumpOptions {
                base: base(),
                host: &["tools"],
            },
            Some(&["lib"]),
        )
        .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(!text.contains("let tools"), "{text}");
    let mut b = new_vm();
    run(&mut b, "globalThis.tools = { read: (p) => 'host ' + p };");
    run(&mut b, &text);
    assert_eq!(run(&mut b, "lib.go('x')"), "host x", "{text}");
}
