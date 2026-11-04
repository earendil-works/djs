//! The format 2 loader (0171): hand-written dumps in tests/dump/v2/ and refusals.
//!
//! A fixture is a program `NAME.src.js` and a dump of it `NAME.dump.js`, written by hand
//! at idle point K (the test names K; the program's header comment says what the dump
//! holds there, since dumps have no comments, D2641). The program prints through
//! the sync host function `print` and waits through the async host function `later(v,
//! rejected)`, which the driver settles at idle points, oldest call first, with `v` (a
//! rejection if `rejected`). The program's output up to idle point K followed by the
//! loaded dump's output must equal the uninterrupted run's.

extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

use super::dump_load::{LoadOptions, Loaded};
use super::*;

const LATER: u32 = 1;
const PRINT: u32 = 2;
/// A plain dump's `$djs.seed`.
const SEED: u32 = 3;

fn base() -> &'static [u8] {
    static BASE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let mut v = Vm::new(VmOptions::default()).unwrap();
        v.prepare_base_image().unwrap();
        v.bytes().to_vec()
    })
}

/// A fresh VM with `later` and `print`.
fn new_vm() -> Vm {
    new_vm_with(false)
}

/// ... collecting at every safepoint with `stress`.
fn new_vm_with(stress: bool) -> Vm {
    let mut opts = VmOptions::default();
    opts.exec.gc_stress = stress;
    let mut vm = Vm::from_base(base(), opts).unwrap();
    let later = vm.new_async_host_function(LATER, "later").unwrap();
    vm.define_global("later", later).unwrap();
    let print = vm.new_host_function(PRINT, "print").unwrap();
    vm.define_global("print", print).unwrap();
    vm
}

fn host_fn(name: &str) -> Option<u32> {
    match name {
        "later" => Some(LATER),
        "print" => Some(PRINT),
        _ => None,
    }
}

fn host_fn_name(id: u32) -> Option<String> {
    match id {
        LATER => Some("later".into()),
        PRINT => Some("print".into()),
        _ => None,
    }
}

/// Runs the VM to its end, settling `later` calls at idle points (oldest first).
/// `marks[k]` is the number of lines printed when idle point k+1 was reached.
/// With `slice`, `run` gets that much fuel at a time.
fn drive(vm: &mut Vm, lines: &mut Vec<String>, marks: &mut Vec<usize>, slice: Option<u64>) {
    let mut rounds = 0;
    loop {
        rounds += 1;
        assert!(rounds < 1_000_000, "runaway");
        match vm.run(slice.unwrap_or(1_000_000_000)) {
            Status::OutOfFuel if slice.is_some() => {}
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, PRINT, "unexpected host call");
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    break;
                };
                marks.push(lines.len());
                settle(vm, id);
            }
            Status::Threw(e) => {
                let m = vm.display(e);
                lines.push(alloc::format!("threw {m}"));
                break;
            }
            s => panic!("{s:?}"),
        }
    }
    for r in vm.take_unhandled_rejections() {
        let m = vm.display(r);
        lines.push(alloc::format!("unhandled {m}"));
    }
}

/// Settles the `later` call `id` with its arguments.
fn settle(vm: &mut Vm, id: u32) {
    let info = vm.host_call(id).unwrap();
    assert_eq!(info.fn_id, LATER);
    let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
    let rejected = info.args.get(1).copied() == Some(Value::TRUE);
    vm.resolve_host_call(id, if rejected { Err(v) } else { Ok(v) })
        .unwrap();
}

/// The uninterrupted run of a program: its lines and idle marks.
fn run_program(src: &str) -> (Vec<String>, Vec<usize>) {
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap_or_else(|e| panic!("{} at {}:{}", e.message, e.line, e.col));
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    drive(&mut vm, &mut lines, &mut marks, None);
    (lines, marks)
}

fn load(vm: &mut Vm, text: &str) -> Result<Loaded, String> {
    let opts = LoadOptions {
        host_fn: &host_fn,
        ..LoadOptions::default()
    };
    vm.load_dump(text, &opts).map_err(|e| e.message())
}

/// Runs a Script in an idle VM: its completion value.
fn run_value(vm: &mut Vm, src: &str) -> String {
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("check.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    match vm.run(1_000_000) {
        Status::Done(v) => vm.display(v),
        // (Idle: the dump's calls are open.)
        Status::Idle => match vm.outcome() {
            Some(Ok(v)) => vm.display(v),
            o => panic!("{o:?}"),
        },
        s => panic!("{s:?}"),
    }
}

/// Loads `dump` and runs it to the end: the lines it prints. `stress`: collecting at every
/// safepoint, in slices of 97 fuel.
fn continue_dump(text: &str, stress: bool) -> (Loaded, Vec<String>) {
    let mut vm = new_vm_with(stress);
    let loaded = load(&mut vm, text).unwrap_or_else(|e| panic!("{e}"));
    // Idle, waiting on the dump's calls, which the host knows already.
    assert!(vm.new_host_calls().is_empty());
    let mut open = vm.pending_host_calls();
    open.sort_unstable();
    let ids: Vec<u32> = loaded.calls.iter().map(|c| c.id).collect();
    assert_eq!(open, ids);
    assert_eq!(run_value(&mut vm, "typeof $dump"), "undefined");
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    drive(&mut vm, &mut lines, &mut marks, stress.then_some(97));
    for (_, h) in &loaded.handles {
        vm.heap_mut().drop_handle(*h);
    }
    (loaded, lines)
}

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dump/v2")
}

/// Checks fixture `name`, a dump of its program at idle point `k`; returns what the load
/// reported.
fn fixture(name: &str, k: usize) -> Loaded {
    let dir = fixture_dir();
    let src = std::fs::read_to_string(dir.join(alloc::format!("{name}.src.js"))).unwrap();
    let dump = std::fs::read_to_string(dir.join(alloc::format!("{name}.dump.js"))).unwrap();
    assert!(dump.starts_with("\"djs-dump 2\";\n"), "{name}: header");
    let (want, marks) = run_program(&src);
    assert!(
        k >= 1 && k <= marks.len(),
        "{name}: {} idle points",
        marks.len()
    );
    let (loaded, after) = continue_dump(&dump, false);
    let mut got: Vec<String> = want[..marks[k - 1]].to_vec();
    got.extend(after);
    assert_eq!(got, want, "{name}");
    let (_, stressed) = continue_dump(&dump, true);
    assert_eq!(stressed, got[marks[k - 1]..], "{name} under stress");
    if std::env::var_os("DJS_DUMP_SHOW").is_some() {
        std::println!(
            "{name}: idle point {k} of {}, {} lines after it:",
            marks.len(),
            want.len() - marks[k - 1]
        );
        for l in &want {
            std::println!("  {l}");
        }
    }
    loaded
}

#[test]
fn plan_example() {
    let l = fixture("plan", 5);
    assert_eq!(
        l.calls,
        [super::dump_load::OpenCall {
            id: 5,
            fn_id: LATER,
            name: "later".into(),
            state: None,
        }]
    );
    assert_eq!(l.handles.len(), 1);
}

#[test]
fn every_operand_construct() {
    let l = fixture("operands", 1);
    assert_eq!(l.calls.len(), 17);
}

#[test]
fn generator_at_a_yield() {
    fixture("generator", 2);
}

#[test]
fn async_generators() {
    fixture("asyncgen", 2);
}

#[test]
fn combinators_and_rejections() {
    let l = fixture("combinators", 6);
    let ids: Vec<u32> = l.calls.iter().map(|c| c.id).collect();
    assert_eq!(ids, [6, 7, 8, 9, 10, 11, 12, 13]);
}

#[test]
fn nested_loops() {
    fixture("loops", 3);
}

/// steps.dump.js was written by the exporter at idle point 1 and is kept as a golden
/// fixture.
#[test]
fn an_exported_dump_stays_loadable() {
    fixture("steps", 1);
}

#[test]
fn cells_in_their_tdz() {
    fixture("cells", 1);
}

/// Loads a format 2 dump made of a few functions and `tail`; the reason it is refused.
fn refusal(tail: &str) -> String {
    let text = alloc::format!(
        "\"djs-dump 2\";\n\
         async function f(a) {{ const b = a + 1; return o.m(b, await later(b)); }}\n\
         function* gen() {{ yield 1; }}\n\
         async function fin() {{ try {{ return 1; }} finally {{ await later(2); }} }}\n\
         async function cls() {{ class C {{ [await later(2)]() {{}} }} }}\n\
         async function keys(o) {{ for (const k in o) await later(k); }}\n\
         const o = {{ m: (x, y) => x + y }};\n\
         $dump.p = $dump.pending(1, 'later', [2]);\n\
         {tail}"
    );
    let mut vm = new_vm();
    match load(&mut vm, &text) {
        Ok(_) => panic!("loaded: {tail}"),
        Err(e) => e,
    }
}

/// The slots of `f` at its await.
const F_SLOTS: &str = "a: 1, b: 2, 'call.callee@46': o.m, 'call.this@46': o, 'call.arg@50': 2";

/// A description of `f` at its await, with `extra` keys (overriding).
fn describe(extra: &str) -> String {
    alloc::format!(
        "$dump.resume(f, {{ at: 'await#1', awaiting: $dump.p, args: [1], \
         slots: {{ {F_SLOTS} }}, {extra} }});"
    )
}

/// `describe` with the slots `slots`.
fn describe_slots(slots: &str) -> String {
    describe(&alloc::format!("slots: {{ {slots} }}"))
}

#[test]
fn resume_refuses_what_the_map_does_not_have() {
    // The description as such loads.
    let mut vm = new_vm();
    let ok = alloc::format!(
        "\"djs-dump 2\";\nasync function f(a) {{ const b = a + 1; return o.m(b, await later(b)); }}\n\
         const o = {{ m: (x, y) => x + y }};\n$dump.p = $dump.pending(1, 'later', [2]);\n\
         $dump.handle('r', {});",
        describe("").trim_end_matches(';')
    );
    load(&mut vm, &ok).unwrap_or_else(|e| panic!("{e}"));

    let cases: &[(&str, &str)] = &[
        (&describe("at: 'await#2'"), "f has no site await#2"),
        (&describe("at: 'yield#1'"), "f has no site yield#1"),
        (
            &describe_slots("a: 1, 'call.callee@46': o.m, 'call.this@46': o, 'call.arg@50': 2"),
            "await#1 has a slot b: give it",
        ),
        (
            &describe_slots(&alloc::format!("{F_SLOTS}, c: 3")),
            "await#1 has no slot c",
        ),
        // A binding's key has its scope only when two of the name are in scope.
        (
            &describe_slots(
                "a: 1, 'b@0': 2, 'call.callee@46': o.m, 'call.this@46': o, 'call.arg@50': 2",
            ),
            "await#1 has a slot b: give it",
        ),
        // By name: any order loads.
        (
            &describe_slots(
                "'call.arg@50': 2, 'call.this@46': o, b: 2, 'call.callee@46': o.m, a: 1",
            ),
            "",
        ),
        (
            &describe_slots("a: 1, b: 2, 'call.callee@46': o.m, 'call.this@46': o"),
            "await#1 has a slot call.arg@50: give it",
        ),
        // A temporary of another construct or offset (another build's slot set).
        (
            &describe_slots(
                "a: 1, b: 2, 'call.callee@46': o.m, 'call.this@46': o, 'call.arg@51': 2",
            ),
            "await#1 has a slot call.arg@50: give it",
        ),
        (
            &describe_slots(&alloc::format!("{F_SLOTS}, 'binary.left@1': 1")),
            "await#1 has no slot binary.left@1",
        ),
        (&describe("operands: []"), "unknown key operands"),
        (&describe("bindings: {}"), "unknown key bindings"),
        (&describe("loops: []"), "unknown key loops"),
        (
            &describe("awaiting: 1"),
            "await#1 awaits a promise: give it",
        ),
        (
            &describe("awaiting: undefined"),
            "await#1 awaits a promise: give it",
        ),
        (&describe("wait: 1"), "unknown key wait"),
        (&describe("args: [1, , 2]"), "an array with holes"),
        (
            &describe_slots(
                "a: (() => { let z = 1; return $dump.cell(() => z); })(), b: 2, \
                 'call.callee@46': o.m, 'call.this@46': o, 'call.arg@50': 2",
            ),
            "a is not captured",
        ),
        (
            &describe("queue: [{ type: 'next', promise: new Promise(() => {}) }]"),
            "only async generators have a request queue",
        ),
        (
            "$dump.resume(gen, { at: 'yield#1', awaiting: $dump.p });",
            "yield#1 is a yield",
        ),
        (
            "$dump.resume(o.m, { at: 'await#1' });",
            "not an async function or a generator",
        ),
        (
            "$dump.resume(cls, { at: 'await#1', awaiting: $dump.p });",
            "cannot be resumed: class",
        ),
        (
            "$dump.resume(fin, { at: 'await#1', awaiting: $dump.p, slots: { 'completion.kind@23': 2, 'completion.value@23': 1, 'finally.return@41': 7 } });",
            "await#1 has no slot finally.return@41",
        ),
        (
            "$dump.resume(fin, { at: 'await#1', awaiting: $dump.p, slots: { 'completion.kind@23': 2 } });",
            "await#1 has a slot completion.value@23: give it",
        ),
        (
            "$dump.resume(keys, { at: 'await#1', awaiting: $dump.p, args: [{}], slots: { o: {}, k: 'a', 'for-in.iterator@25': 1 } });",
            "a for-in loop's iterator is made by $dump.forIn",
        ),
        (
            "$dump.resume((async function () {}).constructor('await 1'), { at: 'await#1', awaiting: $dump.p });",
            "not made by the dump's text",
        ),
        (
            "(() => { let x, y; $dump.cell(() => [x, y]); })();",
            "capture exactly one variable",
        ),
        (
            "$dump.iteratorHelper([1, 0]);",
            "not the slots of an iterator helper",
        ),
        (
            "$dump.iteratorHelper([12, 0, 0, 0, 0, 0, 0, 0]);",
            "not the slots of an iterator helper",
        ),
        (
            "$dump.regexpIterator(/a/g, 1, 0);",
            "not (an object, a string, flags 0-7)",
        ),
        (
            "$dump.regexpIterator(/a/g, 'a', 9);",
            "not (an object, a string, flags 0-7)",
        ),
        ("$dump.forIn(1, []);", "not an object or undefined"),
        ("$dump.forIn(undefined, [1]);", "a key that is not a string"),
        ("$dump.using({});", "the resources are not an array"),
        (
            "$dump.using([1, 2, 7]);",
            "not [value, method, kind] triples",
        ),
        ("$dump.using([], { next: 1 });", "a bad next"),
        ("$dump.using([], { hasError: 1 });", "a bad hasError"),
        ("$dump.using([], { other: 1 });", "unknown key other"),
        ("$dump.using([1, 2, 0], { next: 0, hasError: false });", ""),
        ("$dump.pending(1, 'later', []);", "call 1 is open already"),
        (
            "$dump.pending(2, 'tools.read', []);",
            "unknown host function tools.read",
        ),
        ("$dump.pending(-1, 'later', []);", "not a call id"),
        (
            "$dump.handle('x', 1); $dump.handle('x', 2);",
            "two handles named x",
        ),
    ];
    for (tail, want) in cases {
        if want.is_empty() {
            let mut vm = new_vm();
            let text = alloc::format!(
                "\"djs-dump 2\";\nasync function f(a) {{ const b = a + 1; return o.m(b, await later(b)); }}\n\
                 const o = {{ m: (x, y) => x + y }};\n$dump.p = $dump.pending(1, 'later', [2]);\n{tail}"
            );
            load(&mut vm, &text).unwrap_or_else(|e| panic!("{tail}: {e}"));
            continue;
        }
        let got = refusal(tail);
        assert!(got.contains(want), "{tail}\n  got: {got}\n  want: {want}");
    }
}

/// Runs `src` to idle point `k`, dumps it, loads the dump into a fresh VM and runs that
/// to the end: the lines before the dump and after, or why the dump was refused.
fn dump_at(src: &str, k: usize) -> Result<(Vec<String>, String), String> {
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    let mut seen = 0;
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, .. } => {
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    panic!("ended before idle point {k}");
                };
                seen += 1;
                if seen == k {
                    break;
                }
                settle(&mut vm, id);
            }
            s => panic!("{s:?}"),
        }
    }
    // `later` and `print` are host globals, as for `djs resume` (0174); `console` and
    // `tools` too, as for a codemode host: plan.src.js's own `const tools` shadows it.
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["console", "tools", "print", "later"],
        host_fn_name: &host_fn_name,
        ..Default::default()
    };
    let text = vm.dump(&opts, None).map_err(|e| e.message())?;
    if std::env::var("DJS_DUMP_SHOW").is_ok_and(|v| v == "all") {
        std::println!("{text}");
    }
    let (_, after) = continue_dump(&text, false);
    lines.extend(after);
    Ok((lines, text))
}

#[test]
fn exporter_round_trips_at_every_idle_point() {
    let mut failed = Vec::new();
    for name in [
        "plan",
        "operands",
        "generator",
        "asyncgen",
        "combinators",
        "loops",
        "cells",
        "steps",
        "reenact",
        "cycles",
    ] {
        let src =
            std::fs::read_to_string(fixture_dir().join(alloc::format!("{name}.src.js"))).unwrap();
        let (want, marks) = run_program(&src);
        for k in 1..=marks.len() {
            match dump_at(&src, k) {
                Ok((got, text)) => {
                    if std::env::var("DJS_DUMP_SHOW")
                        .is_ok_and(|v| v == alloc::format!("{name}@{k}"))
                    {
                        std::println!("{text}");
                    }
                    if got != want {
                        failed.push(alloc::format!(
                            "{name} @{k}: {got:?}\n  want {want:?}\n{text}"
                        ));
                    }
                }
                Err(e) => failed.push(alloc::format!("{name} @{k}: {e}")),
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// A loaded dump is its own fixed point: dumping the VM right after the load (the loaded
/// state, the next idle point) writes the same text (docs/suspension-design-review.md
/// section 7: hosts can skip re-dumping an unchanged run).
#[test]
fn dumping_a_loaded_dump_writes_the_same_text() {
    // (`dump_at`'s.)
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["console", "tools", "print", "later"],
        host_fn_name: &host_fn_name,
        ..Default::default()
    };
    let mut failed = Vec::new();
    for name in [
        "plan",
        "operands",
        "generator",
        "asyncgen",
        "combinators",
        "loops",
        "cells",
        "steps",
        "reenact",
        "cycles",
    ] {
        let src =
            std::fs::read_to_string(fixture_dir().join(alloc::format!("{name}.src.js"))).unwrap();
        let (_, marks) = run_program(&src);
        for k in 1..=marks.len() {
            let text = match dump_at(&src, k) {
                Ok((_, text)) => text,
                Err(e) => {
                    failed.push(alloc::format!("{name} @{k}: {e}"));
                    continue;
                }
            };
            let mut vm = new_vm();
            if let Err(e) = load(&mut vm, &text) {
                failed.push(alloc::format!("{name} @{k}: {e}"));
                continue;
            }
            match vm.dump(&opts, None) {
                Ok(again) if again == text => {}
                Ok(again) => failed.push(alloc::format!("{name} @{k}:\n{text}\nagain:\n{again}")),
                Err(e) => failed.push(alloc::format!("{name} @{k}: {}", e.message())),
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Prints the suspension maps of a file, for writing fixtures by hand:
/// `DJS_SMAP=tests/dump/v2/x.src.js cargo test -p djs-core --lib show_maps -- --ignored --nocapture`.
#[test]
#[ignore]
fn show_maps() {
    let path = std::env::var("DJS_SMAP").expect("DJS_SMAP=file");
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap();
    let mut vm = new_vm();
    let maps = crate::compiler::suspension_maps(
        vm.heap_mut(),
        src.as_bytes(),
        &crate::compiler::CompileOptions::script("src.js"),
    )
    .unwrap();
    for m in maps {
        std::print!("{}", m.text(src.as_bytes()));
    }
}

#[test]
fn dumps_need_an_idle_vm() {
    let opts = super::dump::DumpOptions {
        base: base(),
        host_fn_name: &host_fn_name,
        ..Default::default()
    };
    let reason = |vm: &Vm| match vm.dump(&opts, None) {
        Ok(t) => panic!("dumped:\n{t}"),
        Err(e) => e.message(),
    };
    // Parked on a sync host call.
    let mut vm = new_vm();
    let f = vm
        .compile(
            b"print(1);",
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(1_000_000), Status::HostCall { .. }));
    assert!(reason(&vm).contains("is not idle"), "{}", reason(&vm));
    // Out of fuel in the middle of a call.
    let mut vm = new_vm();
    let f = vm
        .compile(
            b"for (let i = 0; i < 1e6; i++);",
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(100), Status::OutOfFuel));
    assert!(reason(&vm).contains("is not idle"), "{}", reason(&vm));
    // A host result delivered and not applied yet.
    let mut vm = new_vm();
    let f = vm
        .compile(
            b"globalThis.p = later(1);",
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(1_000_000), Status::Idle | Status::Done(_)));
    let id = vm.pending_host_calls()[0];
    vm.resolve_host_call(id, Ok(Value::int(1))).unwrap();
    assert!(reason(&vm).contains("not applied yet"), "{}", reason(&vm));
    // A call of a host function the host does not name.
    let mut vm = new_vm();
    let anon = vm.new_async_host_function(9, "anon").unwrap();
    vm.define_global("anon", anon).unwrap();
    let f = vm
        .compile(b"anon();", &crate::compiler::CompileOptions::script("t.js"))
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    vm.run(1_000_000);
    let r = reason(&vm);
    assert!(
        r.contains("<call 1>: a call of a host function the host does not name"),
        "{r}"
    );
    assert!(
        r.contains("anon: a host function the host does not name"),
        "{r}"
    );
}

/// example's scenario (0179): dumped during the first sleep, the report names both
/// activations and where they stopped; the text has no comments about them.
#[test]
fn the_report_names_where_activations_stopped() {
    let src = std::fs::read_to_string(fixture_dir().join("steps.src.js")).unwrap();
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("steps.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(
        vm.run(1_000_000_000),
        Status::Idle | Status::Done(_)
    ));
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["later", "print"],
        host_fn_name: &host_fn_name,
        ..Default::default()
    };
    let d = vm.dump_report(&opts, None).unwrap();
    let got: Vec<(String, String, u32, u32, String)> = d
        .activations
        .iter()
        .map(|s| {
            (
                s.function.clone(),
                s.site.clone(),
                s.line,
                s.col,
                String::from(s.source_line.trim()),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "step".into(),
                "await#1".into(),
                6,
                3,
                "await sleep(ms);".into()
            ),
            (
                "main".into(),
                "await#1".into(),
                13,
                18,
                "results.push(await step(label, 10));".into()
            ),
        ]
    );
    assert_eq!(
        (d.open_calls, d.activations[0].file.as_str()),
        (1, "steps.js")
    );
    assert!(
        d.text
            .contains(&cat!(d.activations[1].loc, " = $dump.resume(main"))
    );
    assert!(!d.text.contains("^\n"), "{}", d.text);
}

/// A codemode run (pi's Sandbox, `djs run --body`): the body, compiled as an async body
/// with the parameters `tools` and `console`, which the host also defines as globals.
const BODY: &str = "console.log(\"start\");\nlet n = 0;\nfor (const f of await tools.list()) {\n  \
                    const body = await tools.read(f);\n  n += body.length;\n  console.log(f, body);\n}\n\
                    const total = () => n;\nawait later(0);\nconsole.log(\"total\", total());\nreturn n;\n";

/// A VM with the codemode host's `tools` and `console` (made by host code).
fn body_host_vm() -> Vm {
    body_host_vm_with(false)
}

/// ... collecting at every safepoint with `stress`.
fn body_host_vm_with(stress: bool) -> Vm {
    let mut vm = new_vm_with(stress);
    run_value(
        &mut vm,
        "globalThis.tools = { list: () => later([\"a\", \"b\"]), read: (f) => later(f + f) };\n\
         globalThis.console = { log: print }; 0",
    );
    vm
}

/// The body run up to idle point `k` (all of it with `usize::MAX`): its lines, the
/// run's promise and the dump there.
fn body_at(k: usize) -> (Vec<String>, Option<super::dump::Dumped>) {
    let (lines, d) = body_dump(BODY, k);
    (
        lines,
        d.map(|d| d.unwrap_or_else(|e| panic!("idle point {k}: {e}"))),
    )
}

/// [`body_at`] for body `src`: the dump, or why it was refused.
#[allow(clippy::type_complexity)]
fn body_dump(src: &str, k: usize) -> (Vec<String>, Option<Result<super::dump::Dumped, String>>) {
    let mut vm = body_host_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::async_body("body.js", &["tools", "console"]),
        )
        .unwrap();
    let g = vm.global();
    let args = ["tools", "console"].map(|n| {
        let k = vm.heap_mut().atom_str(n).unwrap();
        vm.heap().get_own_property(g, k).unwrap().value().unwrap()
    });
    vm.call(f, Value::object(g), &args).unwrap();
    let (mut lines, mut seen, mut run) = (Vec::new(), 0, None);
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, .. } => {
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                // The run's promise (`Idle` after the call returned, while it waits).
                if run.is_none() {
                    // Held by a handle: nothing else does once the run is over.
                    let v = state::result(&vm.heap);
                    run = Some(vm.heap_mut().new_handle(v).unwrap());
                }
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    let r = vm.promise_state(vm.heap().get_handle(run.unwrap()));
                    let r = r.flatten().map(|r| r.map(|v| vm.display(v)));
                    lines.push(alloc::format!("run {r:?}"));
                    return (lines, None);
                };
                seen += 1;
                if seen == k {
                    break;
                }
                settle(&mut vm, id);
            }
            Status::Threw(e) => {
                lines.push(alloc::format!("threw {}", vm.display(e)));
                return (lines, None);
            }
            s => panic!("{s:?}"),
        }
    }
    let handles = [("run", vm.heap().get_handle(run.unwrap()))];
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["console", "tools", "print", "later"],
        host_fn_name: &host_fn_name,
        handles: &handles,
        call_states: &[],
        plain: false,
    };
    let d = vm.dump_report(&opts, None).map_err(|e| e.message());
    (lines, Some(d))
}

/// The run's own activation is written as `async function (tools, console) {...}` and
/// resumed at every idle point; the report points into the body.
#[test]
fn a_codemode_body_run_round_trips() {
    let (want, _) = body_at(usize::MAX);
    assert_eq!(want.last().unwrap(), "run Some(Ok(\"4\"))", "{want:?}");
    for k in 1..=4 {
        let (mut lines, d) = body_at(k);
        let d = d.unwrap();
        if std::env::var("DJS_DUMP_SHOW").is_ok_and(|v| v == alloc::format!("body@{k}")) {
            std::println!("{}", d.text);
        }
        assert!(
            d.text
                .contains("$dump.resume(async function (tools, console) {\nconsole.log("),
            "{}",
            d.text
        );
        let mut vm = body_host_vm();
        let loaded = load(&mut vm, &d.text).unwrap_or_else(|e| panic!("@{k}: {e}\n{}", d.text));
        let (mut after, mut marks) = (Vec::new(), Vec::new());
        drive(&mut vm, &mut after, &mut marks, None);
        lines.extend(after);
        let run = vm.heap().get_handle(loaded.handles[0].1);
        let r = vm
            .promise_state(run)
            .flatten()
            .map(|r| r.map(|v| vm.display(v)));
        lines.push(alloc::format!("run {r:?}"));
        assert_eq!(lines, want, "@{k}\n{}", d.text);
        let at: Vec<(String, u32, u32, String)> = d
            .activations
            .iter()
            .map(|s| {
                (
                    s.site.clone(),
                    s.line,
                    s.col,
                    String::from(s.source_line.trim()),
                )
            })
            .collect();
        let want_at = match k {
            1 => ("await#1", 3, 17, "for (const f of await tools.list()) {"),
            2 | 3 => ("await#2", 4, 16, "const body = await tools.read(f);"),
            _ => ("await#3", 9, 1, "await later(0);"),
        };
        assert_eq!(
            at,
            [(want_at.0.into(), want_at.1, want_at.2, want_at.3.into())],
            "@{k}"
        );
        assert_eq!(d.activations[0].file, "body.js");
    }
}

/// A body whose code would mean something else as a function is refused: `arguments`
/// is a global in a body and the call's arguments in a function.
#[test]
fn a_codemode_body_that_differs_as_a_function_is_refused() {
    let (_, d) = body_dump("await later(1); console.log(typeof arguments);", 1);
    let e = d.unwrap().unwrap_err();
    assert!(
        e.contains("a codemode body whose frame at await#1 differs from the async function"),
        "{e}"
    );
    // `this` is the global object in both.
    let src = "const t = this; await later(1); console.log(t === globalThis); return 5";
    let (want, _) = body_dump(src, usize::MAX);
    let (mut lines, d) = body_dump(src, 1);
    let d = d.unwrap().unwrap();
    let mut vm = body_host_vm();
    load(&mut vm, &d.text).unwrap();
    drive(&mut vm, &mut lines, &mut Vec::new(), None);
    assert_eq!(lines, want[..want.len() - 1]);
    assert_eq!(want, ["true", "run Some(Ok(\"5\"))"]);
}

// ------------------------------------------------------------------ golden dumps (0178)
//
// tests/dump/v2/golden/: small programs (`NAME.src.js` Scripts, `NAME.body.js` codemode
// bodies) and the dumps the exporter wrote of them, `NAME.K.dump.js` at idle point K.
// Every later build must load them and finish the run as the program does (D2560); a
// golden dump changes only with a decision (D2642). The host is `body_host_vm`'s: `later`,
// `print`, `tools`, `console`. Write the dumps of a new program (never over existing
// ones) with `DJS_GOLDEN_WRITE=1 cargo test -p djs-core --lib write_golden_dumps --
// --ignored`.

const GOLDEN_HOST: &[&str] = &["console", "tools", "print", "later"];

fn golden_dir() -> std::path::PathBuf {
    fixture_dir().join("golden")
}

/// The golden programs: name, whether it is a codemode body, source.
fn golden_programs() -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(golden_dir()).unwrap() {
        let file = e.unwrap().file_name().into_string().unwrap();
        let (name, body) = match (file.strip_suffix(".src.js"), file.strip_suffix(".body.js")) {
            (Some(n), _) => (n, false),
            (_, Some(n)) => (n, true),
            _ => continue,
        };
        let src = std::fs::read_to_string(golden_dir().join(&file)).unwrap();
        out.push((String::from(name), body, src));
    }
    out.sort();
    out
}

/// The golden dumps: program name, idle point, file name.
fn golden_dumps() -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(golden_dir()).unwrap() {
        let file = e.unwrap().file_name().into_string().unwrap();
        let Some(stem) = file.strip_suffix(".dump.js") else {
            continue;
        };
        let (name, k) = stem.rsplit_once('.').unwrap();
        out.push((String::from(name), k.parse().unwrap(), file));
    }
    out.sort();
    out
}

/// Starts golden program `src` in a host VM: a Script, or a body called with the host's
/// `tools` and `console`.
fn golden_start(src: &str, body: bool) -> Vm {
    let mut vm = body_host_vm();
    let g = vm.global();
    if body {
        let f = vm
            .compile(
                src.as_bytes(),
                &crate::compiler::CompileOptions::async_body("body.js", &["tools", "console"]),
            )
            .unwrap();
        let args = ["tools", "console"].map(|n| {
            let k = vm.heap_mut().atom_str(n).unwrap();
            vm.heap().get_own_property(g, k).unwrap().value().unwrap()
        });
        vm.call(f, Value::object(g), &args).unwrap();
    } else {
        let f = vm
            .compile(
                src.as_bytes(),
                &crate::compiler::CompileOptions::script("src.js"),
            )
            .unwrap_or_else(|e| panic!("{} at {}:{}", e.message, e.line, e.col));
        vm.call(f, Value::object(g), &[]).unwrap();
    }
    vm
}

/// Runs to idle point `k` (true) or to the end, as `drive` does. `run` is the run's
/// promise; with `entry`, the entry call's result at the first stop (a body's promise).
/// At the end, the promise's state is a line of its own.
fn golden_drive(
    vm: &mut Vm,
    k: usize,
    entry: bool,
    run: &mut Option<crate::heap::Handle>,
    lines: &mut Vec<String>,
    marks: &mut Vec<usize>,
    slice: Option<u64>,
) -> bool {
    loop {
        match vm.run(slice.unwrap_or(1_000_000_000)) {
            Status::OutOfFuel if slice.is_some() => {}
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, PRINT, "unexpected host call");
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                if entry && run.is_none() {
                    let v = state::result(&vm.heap);
                    *run = Some(vm.heap_mut().new_handle(v).unwrap());
                }
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    break;
                };
                marks.push(lines.len());
                if marks.len() == k {
                    return true;
                }
                settle(vm, id);
            }
            Status::Threw(e) => {
                let m = vm.display(e);
                lines.push(alloc::format!("threw {m}"));
                break;
            }
            s => panic!("{s:?}"),
        }
    }
    for r in vm.take_unhandled_rejections() {
        let m = vm.display(r);
        lines.push(alloc::format!("unhandled {m}"));
    }
    let v = run.map_or(Value::UNDEFINED, |h| vm.heap().get_handle(h));
    if let Some(r) = vm.promise_state(v) {
        let r = r.map(|r| r.map(|v| vm.display(v)));
        lines.push(alloc::format!("run {r:?}"));
    }
    false
}

/// A golden program's uninterrupted run: its lines and idle marks.
fn golden_run(src: &str, body: bool) -> (Vec<String>, Vec<usize>) {
    let mut vm = golden_start(src, body);
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    golden_drive(
        &mut vm,
        usize::MAX,
        body,
        &mut None,
        &mut lines,
        &mut marks,
        None,
    );
    (lines, marks)
}

/// Loads golden dump `text` and finishes the run: the lines it prints.
fn golden_continue(text: &str, stress: bool) -> Result<Vec<String>, String> {
    let mut vm = body_host_vm_with(stress);
    let loaded = load(&mut vm, text)?;
    let mut run = loaded
        .handles
        .iter()
        .find(|(n, _)| n == "run")
        .map(|(_, h)| *h);
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    golden_drive(
        &mut vm,
        usize::MAX,
        false,
        &mut run,
        &mut lines,
        &mut marks,
        stress.then_some(97),
    );
    Ok(lines)
}

/// Every golden dump loads and finishes as its program does, also under GC stress and
/// in slices; every program has a dump.
#[test]
fn golden_dumps_load_and_finish() {
    let programs = golden_programs();
    let dumps = golden_dumps();
    let mut failed = Vec::new();
    for (name, _, _) in &programs {
        if !dumps.iter().any(|d| d.0 == *name) {
            failed.push(alloc::format!("{name}: no dump"));
        }
    }
    for (name, k, file) in &dumps {
        let Some((_, body, src)) = programs.iter().find(|p| p.0 == *name) else {
            failed.push(alloc::format!("{file}: no program {name}"));
            continue;
        };
        let text = std::fs::read_to_string(golden_dir().join(file)).unwrap();
        let (want, marks) = golden_run(src, *body);
        if *k == 1 && std::env::var("DJS_DUMP_SHOW").is_ok_and(|v| v == "golden") {
            std::println!("{name}: {} idle points, {want:?}", marks.len());
        }
        if !text.starts_with("\"djs-dump 2\";\n") {
            failed.push(alloc::format!("{file}: not a format 2 dump"));
            continue;
        }
        if *k == 0 || *k > marks.len() {
            failed.push(alloc::format!(
                "{file}: {name} has {} idle points",
                marks.len()
            ));
            continue;
        }
        for stress in [false, true] {
            match golden_continue(&text, stress) {
                Ok(after) => {
                    let mut got = want[..marks[k - 1]].to_vec();
                    got.extend(after);
                    if got != want {
                        failed.push(alloc::format!(
                            "{file} (stress {stress}): {got:?}\n  want {want:?}"
                        ));
                    }
                }
                Err(e) => failed.push(alloc::format!("{file}: {e}")),
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Writes the dumps of the golden programs that have none, at every idle point.
#[test]
#[ignore]
fn write_golden_dumps() {
    assert!(
        std::env::var_os("DJS_GOLDEN_WRITE").is_some(),
        "DJS_GOLDEN_WRITE=1 writes golden dumps"
    );
    let dumps = golden_dumps();
    for (name, body, src) in golden_programs() {
        if dumps.iter().any(|d| d.0 == name) {
            continue;
        }
        let (_, marks) = golden_run(&src, body);
        for k in 1..=marks.len() {
            let mut vm = golden_start(&src, body);
            let mut run = None;
            assert!(golden_drive(
                &mut vm,
                k,
                body,
                &mut run,
                &mut Vec::new(),
                &mut Vec::new(),
                None
            ));
            let run = run.map_or(Value::UNDEFINED, |h| vm.heap().get_handle(h));
            let handles = [("run", run)];
            let opts = super::dump::DumpOptions {
                base: base(),
                host: GOLDEN_HOST,
                host_fn_name: &host_fn_name,
                handles: if body { &handles } else { &[] },
                call_states: &[],
                plain: false,
            };
            let text = vm
                .dump(&opts, None)
                .unwrap_or_else(|e| panic!("{name} @{k}: {}", e.message()));
            let file = golden_dir().join(alloc::format!("{name}.{k}.dump.js"));
            std::fs::write(&file, text).unwrap();
            std::println!("wrote {}", file.display());
        }
    }
}

/// The common pi shape: a closure over a body's parameter makes it a boxed local, which
/// must sit where the async function the dump writes keeps it (D2620; 0174's repro).
#[test]
fn a_codemode_body_whose_parameter_a_closure_captures() {
    let p = ["tools", "console"];
    let body = "const a = await later(1);\nfunction f() { tools; }\n";
    let wrapped = alloc::format!("(async function (tools, console) {{\n{body}\n}})");
    let mut vm = new_vm();
    let site = |vm: &mut Vm, src: &str, o: &crate::compiler::CompileOptions<'_>| {
        let maps = crate::compiler::suspension_maps(vm.heap_mut(), src.as_bytes(), o).unwrap();
        let s = maps.last().unwrap().sites[0].clone();
        (s.args, s.locals, s.boxed)
    };
    let a = site(
        &mut vm,
        body,
        &crate::compiler::CompileOptions::async_body("b.js", &p),
    );
    let b = site(
        &mut vm,
        &wrapped,
        &crate::compiler::CompileOptions::script("d.js"),
    );
    assert_eq!(a, b);
    let src = "const t = tools;\nconst a = await later(1);\nconst f = (k) => tools === t && k;\n\
               console.log(f(a), await tools.read(\"x\"));\nreturn a;\n";
    let (want, _) = body_dump(src, usize::MAX);
    assert_eq!(want, ["1 xx", "run Some(Ok(\"1\"))"]);
    for k in 1..=2 {
        let (mut lines, d) = body_dump(src, k);
        let d = d.unwrap().unwrap_or_else(|e| panic!("@{k}: {e}"));
        let mut vm = body_host_vm();
        let loaded = load(&mut vm, &d.text).unwrap();
        drive(&mut vm, &mut lines, &mut Vec::new(), None);
        let run = vm.heap().get_handle(loaded.handles[0].1);
        let r = vm
            .promise_state(run)
            .flatten()
            .map(|r| r.map(|v| vm.display(v)));
        lines.push(alloc::format!("run {r:?}"));
        assert_eq!(lines, want, "@{k}");
    }
}

/// `const r = await ...` while the await is pending: a slot the compiler keeps in its
/// TDZ (code reads it before the declaration) is `$dump.TDZ`, a captured one a cell in
/// its TDZ; a slot it does not (no read can come first) holds undefined, which nothing
/// can observe. 0179's question; the TDZ slot was refused as "an internal value".
#[test]
fn bindings_in_their_tdz_at_an_await() {
    for (src, want) in [
        (
            "async function main() { try { const r = (await later(1)) + r; print(r); } catch (e) { print(e.name); } }\nmain();",
            "slots: { r: $dump.TDZ }",
        ),
        (
            "let get; later(2).then(() => print(get()));\nasync function main() { get = () => { try { return r; } catch (e) { return e.name; } }; const r = await later(1); print(r); }\nmain();",
            "r: $dump.s",
        ),
        (
            "async function main() { for (let i = 0; i < 2; i++) { if (i) { try { print(r); } catch (e) { print(e.name); } } const r = await later(i); } }\nmain();",
            "slots: { i: 0, r: $dump.TDZ }",
        ),
        (
            "async function main() { const r = await later(1); print(r); }\nmain();",
            "slots: { r: undefined }",
        ),
    ] {
        let (got, text) = dump_at(src, 1).unwrap_or_else(|e| panic!("{src}: {e}"));
        assert_eq!(got, run_program(src).0, "{src}");
        assert!(got[0] == "ReferenceError" || got[0] == "1", "{got:?}");
        assert!(text.contains(want), "{src}\n{text}");
    }
}

/// A class with instance fields or private members declared in a
/// waiting function: the frame's hidden bindings of the class are dead after its
/// statement and written as undefined; the class comes from its text (0175's repro).
#[test]
fn classes_with_fields_in_a_waiting_frame() {
    for src in [
        "class C { x = 1; } const c = new C(); await later(1); return c.x;",
        "class C { #n = 0; inc() { return ++this.#n; } } const c = new C(); await later(1); return c.inc();",
        "class C { static s = 1; kk = 2; #m() { return 3; } get m() { return this.#m(); } }\n\
         const c = new C(); await later(1); return c.kk + c.m + C.s;",
        "class P { #n = 0; x = 1; async add() { this.#n += String(await later(1)).length; return this.#n + this.x; } }\n\
         const p = new P(); return [await p.add(), await p.add()].join();",
        "const out = []; for (const v of [1, 2]) { class C { y = v; } out.push(new C().y + (await later(v))); } return out.join();",
    ] {
        let (want, _) = body_dump(src, usize::MAX);
        assert!(!want.last().unwrap().contains("Err"), "{src}: {want:?}");
        for k in 1..=marks_of(src) {
            let (mut lines, d) = body_dump(src, k);
            let d = d.unwrap().unwrap_or_else(|e| panic!("{src} @{k}: {e}"));
            let mut vm = body_host_vm();
            let loaded = load(&mut vm, &d.text).unwrap_or_else(|e| panic!("{src} @{k}: {e}"));
            drive(&mut vm, &mut lines, &mut Vec::new(), None);
            let run = vm.heap().get_handle(loaded.handles[0].1);
            let r = vm
                .promise_state(run)
                .flatten()
                .map(|r| r.map(|v| vm.display(v)));
            lines.push(alloc::format!("run {r:?}"));
            assert_eq!(lines, want, "{src} @{k}\n{}", d.text);
        }
    }
}

/// A private name in a frame (the key operand of `this.#n += await v`) is its class's
/// (`$dump.private(C, "#n")`), not a symbol of the same description.
#[test]
fn a_private_name_operand_is_the_class_s() {
    let src = "class P { #n = 1; async add() { this.#n += await later(2); return this.#n; } }\n\
               const p = new P(); p.add().then(print);";
    let (got, text) = dump_at(src, 1).unwrap();
    assert_eq!(got, ["3"]);
    assert!(text.contains("= $dump.private(P, \"#n\");"), "{text}");
}

/// The number of idle points of body `src`.
fn marks_of(src: &str) -> usize {
    (1..).find(|&k| body_dump(src, k).1.is_none()).unwrap() - 1
}

/// A class's or object literal method's computed key is evaluated again where the dump
/// writes its text: refused unless its value cannot differ (D2623; 0182).
#[test]
fn computed_keys_evaluated_again_are_refused() {
    for (src, why) in [
        (
            "const k = \"kk\"; class C { [k] = 2; } const c = new C(); later(1).then(() => print(c.kk, new C().kk));",
            "C: a computed key [k] that the dump would evaluate again",
        ),
        (
            "function mk() { const k = \"kk\"; return class C { [k]() { return 2; } }; } const C = mk(); later(1).then(() => print(new C().kk()));",
            "C: a computed key [k] that the dump would evaluate again",
        ),
        (
            "function mk() { const k = \"kk\"; return { [k]() { return 2; } }; } const o = mk(); later(1).then(() => print(o.kk()));",
            "o.kk: a computed key [k] that the dump would evaluate again",
        ),
    ] {
        let e = dump_at(src, 1).unwrap_err();
        assert!(e.contains(why), "{src}: {e}");
    }
    // Literal keys and well-known symbols; a computed key inside a method body is the
    // method's own.
    let src = "class C { [Symbol.iterator]() { return [1][Symbol.iterator](); } [\"a b\"]() { return 3; } [0] = 1;\n\
               m() { const o = { [x]: 1 }; return o; } }\n\
               const x = 'q'; const c = new C(); later(1).then(() => print([...c].join(), c['a b'](), JSON.stringify(c.m()), c[0]));";
    assert_eq!(dump_at(src, 1).unwrap().0, ["1 3 {\"q\":1} 1"]);
}

/// The run of `src` dumped at every idle point and continued in a fresh VM that loaded
/// the dump, dump after dump (as `--dump-every-idle`): its lines.
fn chained(src: &str) -> Vec<String> {
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    for _ in 0..1000 {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, .. } => {
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                if vm.pending_host_calls().is_empty() {
                    return lines;
                }
                let opts = super::dump::DumpOptions {
                    base: base(),
                    host: &["console", "tools", "print", "later"],
                    host_fn_name: &host_fn_name,
                    ..Default::default()
                };
                let text = vm
                    .dump(&opts, None)
                    .unwrap_or_else(|e| panic!("{}", e.message()));
                vm = new_vm();
                let loaded = load(&mut vm, &text).unwrap_or_else(|e| panic!("{e}\n{text}"));
                for (_, h) in &loaded.handles {
                    vm.heap_mut().drop_handle(*h);
                }
                let id = *vm.pending_host_calls().iter().min().unwrap();
                settle(&mut vm, id);
            }
            s => panic!("{s:?}"),
        }
    }
    panic!("runaway: {lines:?}");
}

/// `Math.random`'s state goes with the dump: a resumed run continues the sequence
/// (0176's repro, D2624); iterators stepped by `$dump.advance` dump again as they were
/// (two awaits per iteration: dumped again before the loop steps them).
#[test]
fn dumps_carry_the_random_state_and_iterators_again() {
    for src in [
        "async function main() { const a = Math.random(); await later(1); const b = Math.random(); await later(2); print(a, b, Math.random()); }\nmain();",
        "async function main() { const s = new Set([1, 2, 3]).values(); s.next(); for (const [k, v] of [[\"a\", 1], [\"b\", 2], [\"c\", 3]]) print(k, await later(v), await later(v + 1), s.next().value); for (const c of \"h\\u00e9\\u{1F600}x\") print(await later(c)); }\nmain();",
    ] {
        let (want, _) = run_program(src);
        assert_eq!(chained(src), want, "{src}");
    }
}

/// A for-in loop over a Proxy walks a Record its traps fill: refused.
#[test]
fn a_for_in_over_a_proxy_is_refused() {
    let src = "async function main() { for (const k in new Proxy({ a: 1 }, {})) print(k, await later(k)); }\nmain();";
    let e = dump_at(src, 1).unwrap_err();
    assert!(e.contains("a for-in loop over a Proxy"), "{e}");
}

/// The new golden programs' every idle point, dumped once and dumped in a chain.
#[test]
fn delegation_round_trips() {
    for name in [
        "yield-star",
        "for-await-close",
        "finally",
        "using",
        "destructuring",
        "for-in",
        "iterator-helpers",
    ] {
        let src =
            std::fs::read_to_string(golden_dir().join(alloc::format!("{name}.src.js"))).unwrap();
        let (want, marks) = run_program(&src);
        assert!(!marks.is_empty());
        for k in 1..=marks.len() {
            let (got, text) = dump_at(&src, k).unwrap_or_else(|e| panic!("{name} @{k}: {e}"));
            assert_eq!(got, want, "{name} @{k}\n{text}");
        }
        assert_eq!(chained(&src), want, "{name} chained");
    }
}

/// An array with named properties is not a literal alone: it gets a location for its
/// patches even where a frame holds it once.
#[test]
fn a_frame_holds_an_array_with_named_properties() {
    let src =
        "async function main() { const a = [1]; a.x = 2; print(await later(1), a.x); }\nmain();";
    let (got, _) = dump_at(src, 1).unwrap();
    assert_eq!(got, ["1 2"]);
}

/// Runs the VM to its next idle point (printing): `Some(oldest open call)`, or `None` at
/// the end (with "threw ..." for an uncaught error).
fn plain_step(vm: &mut Vm, lines: &mut Vec<String>) -> Option<u32> {
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, fn_id } if fn_id == SEED => {
                let args = vm.host_call(call_id).unwrap().args;
                let half = |v: Value| v.as_number().unwrap_or(0.0) as i32 as u32 as u64;
                vm.set_math_random_seed((half(args[1]) << 32) | half(args[0]));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, PRINT, "unexpected host call");
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => return vm.pending_host_calls().iter().min().copied(),
            Status::Threw(e) => {
                let m = vm.display(e);
                lines.push(alloc::format!("threw {m}"));
                return None;
            }
            s => panic!("{s:?}"),
        }
    }
}

/// A plain dump (D2830) of the idle `vm`, or why it was refused.
fn plain_dump(vm: &Vm) -> Result<String, String> {
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["console", "tools", "print", "later"],
        host_fn_name: &host_fn_name,
        plain: true,
        ..Default::default()
    };
    vm.dump(&opts, None).map_err(|e| e.message())
}

/// The plain dump of program `src` run to idle point `k`.
fn plain_dump_at(src: &str, k: usize) -> String {
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    for _ in 1..k {
        let id = plain_step(&mut vm, &mut lines).expect("an idle point");
        settle(&mut vm, id);
    }
    plain_step(&mut vm, &mut lines).expect("an idle point");
    plain_dump(&vm).unwrap()
}

/// A fresh VM that ran plain dump `text` as a Script, with the host's hooks: `$djs.seed`
/// a no-op (the fixtures draw no random numbers), `$djs.resume` making the call again.
fn plain_load(text: &str) -> Result<Vm, String> {
    let mut vm = new_vm();
    let seed = vm.new_host_function(SEED, "seed").unwrap();
    vm.define_global("$djs_seed", seed).unwrap();
    run_value(
        &mut vm,
        "globalThis.$djs = { seed: $djs_seed, resume: (id, name, args) => later(...args) }; delete globalThis.$djs_seed; 0",
    );
    let f = vm
        .compile(
            text.as_bytes(),
            &crate::compiler::CompileOptions::script("dump.js"),
        )
        .map_err(|e| alloc::format!("{} at {}:{}", e.message, e.line, e.col))?;
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).map_err(|e| alloc::format!("{e:?}"))?;
    loop {
        match vm.run(1_000_000_000) {
            Status::Idle | Status::Done(_) => break,
            Status::HostCall { call_id, fn_id } if fn_id == SEED => {
                let args = vm.host_call(call_id).unwrap().args;
                let half = |v: Value| v.as_number().unwrap_or(0.0) as i32 as u32 as u64;
                vm.set_math_random_seed((half(args[1]) << 32) | half(args[0]));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Threw(e) => return Err(alloc::format!("the load threw {}", vm.display(e))),
            s => return Err(alloc::format!("the load stopped: {s:?}")),
        }
    }
    if run_value(&mut vm, "typeof $dump + typeof $djs") != "undefinedundefined" {
        return Err("the dump left $dump or $djs".into());
    }
    Ok(vm)
}

/// Every fixture program, plain-dumped at every idle point of a chain: the run dumped at
/// its first idle point, the loaded dump at its first, and so on. The lines must be the
/// uninterrupted run's; a refusal ends the chain (counted: the constructs 0203 adds).
/// The programs plain dumps are checked on: the v2 fixtures and the golden programs.
fn plain_programs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for name in [
        "plan",
        "operands",
        "generator",
        "asyncgen",
        "combinators",
        "loops",
        "cells",
        "steps",
        "reenact",
        "cycles",
    ] {
        let src =
            std::fs::read_to_string(fixture_dir().join(alloc::format!("{name}.src.js"))).unwrap();
        out.push((String::from(name), src));
    }
    for (name, body, src) in golden_programs() {
        if !body {
            out.push((alloc::format!("golden/{name}"), src));
        }
    }
    out
}

/// The refusals plain dumps of the programs may give, `program@k: reason` (a ratchet:
/// `DJS_UPDATE_PLAIN_REFUSALS=1` rewrites it).
fn plain_refusals_file() -> std::path::PathBuf {
    fixture_dir().join("plain-refusals.txt")
}

/// Every program dumped (plainly) at each of its idle points, the dump run in a fresh VM
/// to the end: the lines before and after must be the uninterrupted run's. Refusals are
/// checked against tests/dump/v2/plain-refusals.txt.
/// Program `src` dumped plainly at each of its idle points, each dump run in a fresh VM to
/// the end: the failures (`name@k: ...`), the refusals and how many continued identically.
fn plain_check_program(name: &str, src: &str) -> (Vec<String>, Vec<String>, usize) {
    let (mut failed, mut refused, mut ok) = (Vec::new(), Vec::new(), 0);
    let (want, marks) = run_program(src);
    for k in 1..=marks.len() {
        let mut vm = new_vm();
        let f = vm
            .compile(
                src.as_bytes(),
                &crate::compiler::CompileOptions::script("src.js"),
            )
            .unwrap();
        let this = Value::object(vm.global());
        vm.call(f, this, &[]).unwrap();
        let mut lines = Vec::new();
        for _ in 1..k {
            let id = plain_step(&mut vm, &mut lines).expect("an idle point");
            settle(&mut vm, id);
        }
        plain_step(&mut vm, &mut lines).expect("an idle point");
        let text = match plain_dump(&vm) {
            Ok(t) => t,
            Err(e) => {
                let why: Vec<&str> = e.lines().skip(1).map(str::trim).collect();
                refused.push(alloc::format!("{name}@{k}: {}", why.join(" | ")));
                continue;
            }
        };
        if std::env::var("DJS_PLAIN_SHOW").is_ok_and(|v| v == alloc::format!("{name}@{k}")) {
            std::println!("{text}");
        }
        let mut vm = match plain_load(&text) {
            Ok(vm) => vm,
            Err(e) => {
                failed.push(alloc::format!("{name}@{k}: {e}\n{text}"));
                continue;
            }
        };
        while let Some(id) = plain_step(&mut vm, &mut lines) {
            settle(&mut vm, id);
        }
        if lines != want {
            failed.push(alloc::format!(
                "{name}@{k}: {lines:?}\n  want {want:?}\n{text}"
            ));
        } else {
            ok += 1;
        }
    }
    (failed, refused, ok)
}

#[test]
fn plain_dumps_at_every_idle_point() {
    let (mut failed, mut refused, mut ok) = (Vec::new(), Vec::new(), 0);
    for (name, src) in plain_programs() {
        let (f, r, n) = plain_check_program(&name, &src);
        failed.extend(f);
        refused.extend(r);
        ok += n;
    }
    let path = plain_refusals_file();
    if std::env::var("DJS_UPDATE_PLAIN_REFUSALS").is_ok() {
        let mut t = String::from(
            "# Idle points of tests/dump/v2 whose plain dump is refused (plain_dumps_at_every_idle_point).\n",
        );
        for r in &refused {
            t.push_str(r);
            t.push('\n');
        }
        std::fs::write(&path, t).unwrap();
    }
    let listed = std::fs::read_to_string(&path).unwrap_or_default();
    let listed: Vec<&str> = listed.lines().filter(|l| !l.starts_with('#')).collect();
    for r in &refused {
        if !listed.contains(&r.as_str()) {
            failed.push(alloc::format!("refused, not in plain-refusals.txt: {r}"));
        }
    }
    for l in &listed {
        if !refused.iter().any(|r| r == l) {
            std::println!("stale in plain-refusals.txt: {l}");
        }
    }
    std::println!("plain dumps: {ok} identical, {} refused", refused.len());
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Program `src` dumped plainly at its first idle point, the dump run to its first, dumped
/// again, and so on: the failures, the links and the refusal that ended the chain.
fn plain_chain_program(name: &str, src: &str) -> (Vec<String>, usize, Vec<String>) {
    let (mut failed, mut links, mut refused) = (Vec::new(), 0, Vec::new());
    let (want, _) = run_program(src);
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    let mut dumping = true;
    let show = std::env::var("DJS_PLAIN_SHOW").is_ok_and(|v| v == name);
    let mut sizes = Vec::new();
    while let Some(id) = plain_step(&mut vm, &mut lines) {
        if dumping {
            match plain_dump(&vm) {
                Ok(text) => match plain_load(&text) {
                    _ if show && {
                        std::println!("--- {name} link {}\n{text}", links + 1);
                        false
                    } =>
                    {
                        unreachable!()
                    }
                    Ok(v) => {
                        links += 1;
                        sizes.push(text.len());
                        vm = v;
                        let id = vm.pending_host_calls().iter().min().copied();
                        settle(&mut vm, id.expect("the dump's open call"));
                        continue;
                    }
                    Err(e) => {
                        failed.push(alloc::format!("{name} link {}: {e}\n{text}", links + 1));
                        break;
                    }
                },
                Err(e) => {
                    let why: Vec<&str> = e.lines().skip(1).map(str::trim).collect();
                    refused.push(alloc::format!("{name}: {}", why.join(" | ")));
                    dumping = false;
                }
            }
        }
        settle(&mut vm, id);
    }
    if lines != want {
        failed.push(alloc::format!("{name}: {lines:?}\n  want {want:?}"));
    }
    // Origin tracking keeps a chain's dumps the size of their state (D2870): a
    // continuation of a continuation is written from the original's text, so link k
    // is about as long as a dump of the uninterrupted run at idle point k.
    for (k, &len) in sizes.iter().enumerate() {
        let fresh = plain_dump_at(src, k + 1).len();
        if len.abs_diff(fresh) > 512 {
            failed.push(alloc::format!(
                "{name}: link {} is {len} bytes, a dump of the run there {fresh}",
                k + 1
            ));
        }
    }
    (failed, links, refused)
}

#[test]
fn plain_dump_chains_continue_like_the_run() {
    let (mut failed, mut links, mut refused) = (Vec::new(), 0, Vec::new());
    for (name, src) in plain_programs() {
        let (f, n, r) = plain_chain_program(&name, &src);
        failed.extend(f);
        links += n;
        refused.extend(r);
    }
    // Where chains stop: a ratchet like the single dumps' (the same variable updates it).
    let path = fixture_dir().join("plain-chain-refusals.txt");
    if std::env::var("DJS_UPDATE_PLAIN_REFUSALS").is_ok() {
        let mut t = String::from(
            "# Programs of tests/dump/v2 whose plain-dump chain is refused at some link (plain_dump_chains_continue_like_the_run).\n",
        );
        for r in &refused {
            t.push_str(r);
            t.push('\n');
        }
        std::fs::write(&path, t).unwrap();
    }
    let listed = std::fs::read_to_string(&path).unwrap_or_default();
    let listed: Vec<&str> = listed.lines().filter(|l| !l.starts_with('#')).collect();
    for r in &refused {
        if !listed.contains(&r.as_str()) {
            failed.push(alloc::format!(
                "refused, not in plain-chain-refusals.txt: {r}"
            ));
        }
    }
    for l in &listed {
        if !refused.iter().any(|r| r == l) {
            std::println!("stale in plain-chain-refusals.txt: {l}");
        }
    }
    std::println!(
        "plain dumps: {links} links, {} chains refused",
        refused.len()
    );
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(links >= 10, "{links} links");
}

/// A plain continuation's `super` is read again (K is a method of an object whose
/// prototype is the home object's): a base read before the dump that the home object no
/// longer has is refused, not changed.
#[test]
fn a_super_base_changed_since_it_was_read_is_refused() {
    let src = "class A { m(x) { return 'a' + x; } }\nclass B { m(x) { return 'b' + x; } }\n\
               class C extends A { async m() { return super[await later('m')](1); } }\n\
               new C().m().then(print);\nObject.setPrototypeOf(C.prototype, B.prototype);";
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    plain_step(&mut vm, &mut lines).expect("an idle point");
    let e = plain_dump(&vm).unwrap_err();
    assert!(
        e.contains("a super base or this the method no longer has"),
        "{e}"
    );
}

/// `for await` loops closed while `return()` awaits: left by `continue` and `break` to an
/// outer label and by `return` (the completion's kind in the frame), and by a `break`.
#[test]
fn plain_dumps_of_for_await_closes() {
    let src = r#"
async function* source(tag) {
  try { yield tag + "1"; yield tag + "2"; } finally { print(tag, "closing", await later(tag)); }
}
async function pick(mode) {
  outer: for (let i = 0; i < 2; i++) {
    inner: for await (const v of source(mode + i)) {
      print("got", v);
      if (mode === "c") continue outer;
      if (mode === "b") break outer;
      if (mode === "r") return "returned " + v;
      break inner;
    }
    print("after", i);
  }
  return mode;
}
async function main() {
  for (const m of ["c", "b", "r", "x"]) print(await pick(m));
}
main();
"#;
    let (failed, refused, ok) = plain_check_program("closes", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    let (failed, _, refused) = plain_chain_program("closes", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ok >= 6, "{ok}");
}

/// Destructuring stopped at every kind of place: holes and rest elements after the site,
/// an object nested in an array and an array in an object, computed keys and a rest
/// property after the site, getters read in order, reference targets of assignment
/// patterns, a source that is a string, an iterator closed after the pattern.
#[test]
fn plain_dumps_of_patterns() {
    let src = r#"
const log = [];
const src = {
  get a() { log.push("get a"); return undefined; },
  get b() { log.push("get b"); return [undefined, 2, 3, 4]; },
  c: 3, d: 4, [Symbol.for("s")]: 5,
};
function* gen() {
  try { yield 1; yield undefined; yield 3; yield 4; } finally { log.push("closed"); }
}
async function f1() {
  const { a = await later("A"), b: [x = await later("X"), , ...ys], ["c" + ""]: c, ...rest } = src;
  return [a, x, ys.join(), c, Object.keys(rest).join(), log.join()].join(" ");
}
async function f2() {
  const [p, q = await later("Q"), { r = await later("R") } = {}] = gen();
  return [p, q, r, log.join()].join(" ");
}
async function f3() {
  const t = {};
  let m;
  [t.a = await later("ta"), t[await later("k")] = await later("tk"), ...m] = "xyz".split("");
  ({ length: t.len = await later("len"), 0: t.first } = "hello");
  return JSON.stringify(t) + " " + m.join();
}
async function f4() {
  const [{ u = await later("U"), ...more } = { v: 1, w: 2 }] = [];
  return u + " " + JSON.stringify(more);
}
async function main() {
  print(await f1());
  print(await f2());
  print(await f3());
  print(await f4());
}
main();
"#;
    let (failed, refused, ok) = plain_check_program("patterns", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    let (failed, _, refused) = plain_chain_program("patterns", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ok >= 6, "{ok}");
}
