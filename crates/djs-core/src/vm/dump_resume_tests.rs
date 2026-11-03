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
            name: "later".into()
        }]
    );
    assert_eq!(l.handles.len(), 1);
}

#[test]
fn every_operand_role() {
    let l = fixture("roles", 1);
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

#[test]
fn role_names_are_the_maps() {
    use crate::compiler::{Loop, Role};
    for r in [
        Role::CallCallee,
        Role::CallArg(3),
        Role::ArrayElement(0),
        Role::TemplatePart(12),
        Role::LoopIterator(Loop::Await),
        Role::LoopNext(Loop::Of),
        Role::SuperBase,
    ] {
        assert_eq!(super::dump_resume::role_name(r), alloc::format!("{r}"));
    }
}

/// Loads a format 2 dump made of a few functions and `tail`; the reason it is refused.
fn refusal(tail: &str) -> String {
    let text = alloc::format!(
        "\"djs-dump 2\";\n\
         async function f(a) {{ const b = a + 1; return o.m(b, await later(b)); }}\n\
         function* gen() {{ yield 1; }}\n\
         async function fin() {{ try {{ return 1; }} finally {{ await later(2); }} }}\n\
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

/// A description of `f` at its await, with `extra` keys (overriding).
fn describe(extra: &str) -> String {
    alloc::format!(
        "$dump.resume(f, {{ at: 'await#1', awaiting: $dump.p, args: [1], \
         bindings: {{ a: 1, b: 2 }}, \
         operands: [['call.callee', o.m], ['call.this', o], ['call.arg[0]', 2]], {extra} }});"
    )
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
        (&describe("bindings: { a: 1 }"), "no value for binding b"),
        (
            &describe("bindings: { a: 1, b: 2, c: 3 }"),
            "await#1 has no binding c",
        ),
        (&describe("bindings: { a: 1, 'b@0': 2 }"), ""),
        (
            &describe("bindings: { a: 1, 'b@7': 2 }"),
            "no value for binding b",
        ),
        (
            &describe("operands: [['call.callee', o.m], ['call.arg[0]', 2], ['call.this', o]]"),
            "operand 1 of await#1 is call.this, not call.arg[0]",
        ),
        (
            &describe("operands: [['call.callee', o.m], ['call.this', o]]"),
            "await#1 needs call.arg[0]",
        ),
        (
            &describe(
                "operands: [['call.callee', o.m], ['call.this', o], ['call.arg[0]', 2], ['binary.left', 1]]",
            ),
            "fewer operands or loops than given",
        ),
        (
            &describe("loops: [{ kind: 'for-of', iterator: [].values(), next: [].values().next }]"),
            "fewer operands or loops than given",
        ),
        (
            &describe("operands: [['call.callee', o.m, 1]]"),
            "a [role, value] pair",
        ),
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
            &describe(
                "bindings: { a: (() => { let z = 1; return $dump.cell(() => z); })(), b: 2 }",
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
            "$dump.resume(fin, { at: 'await#1', awaiting: $dump.p });",
            "cannot be resumed: finally",
        ),
        (
            "$dump.resume(keys, { at: 'await#1', awaiting: $dump.p, args: [{}], bindings: { o: {}, k: 'a' }, loops: [{ kind: 'for-in', iterator: 1 }] });",
            "for-in loops cannot be resumed yet",
        ),
        (
            "$dump.resume((async function () {}).constructor('await 1'), { at: 'await#1', awaiting: $dump.p });",
            "not made by the dump's text",
        ),
        (
            "(() => { let x, y; $dump.cell(() => [x, y]); })();",
            "capture exactly one variable",
        ),
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
        "roles",
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
