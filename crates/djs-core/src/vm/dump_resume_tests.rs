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
