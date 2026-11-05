//! Plain dump size (0207): the sleep-dump example's dump, operands read again (D2905).

use super::*;

/// The sleep-dump example (its `SCRIPT`, sleep-dump.ts in examples/sleep-dump), dumped
/// as the Sandbox dumps it at its first idle point: a codemode body with the parameter
/// `ms`, `console.log` and the async `tools.sleep` as host functions, the open sleep's
/// state its deadline. With `DJS_SLEEP_OUT` the dump is written there, for the size score
/// (tests/scores/dump-size.tsv `sleep_*`, D2901): the dump the owner reads first.
#[test]
fn the_sleep_example_dumps() {
    const SCRIPT: &str = "\nasync function step(label, ms) {\n  console.log(label, \"sleeps\", ms, \"ms\");\n  const r = await tools.sleep(ms);\n  return label + \": \" + r;\n}\nconst t0 = Date.now();\nconst results = [];\nfor (const label of [\"first\", \"second\"]) {\n  results.push(await step(label, ms));\n}\nconsole.log(\"done after\", Date.now() - t0 >= 2 * ms ? \"at least\" : \"less than\", 2 * ms, \"ms:\", results.join(\", \"));\nreturn results;\n";
    const LOG: u32 = 1;
    const SLEEP: u32 = 2;
    let mut vm = Vm::from_base(
        base(),
        VmOptions {
            frozen_builtins: true,
            ..VmOptions::default()
        },
    )
    .unwrap();
    vm.set_now(1_793_822_820_106.0);
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let tools = vm.new_object().unwrap();
    let sleep = vm.new_async_host_function(SLEEP, "sleep").unwrap();
    vm.define(tools, "sleep", sleep).unwrap();
    vm.define_global("tools", Value::object(tools)).unwrap();
    let f = vm
        .compile(
            SCRIPT.as_bytes(),
            &crate::compiler::CompileOptions {
                goal: crate::compiler::Goal::AsyncBody,
                params: &["ms"],
                ..crate::compiler::CompileOptions::script("sleep.js")
            },
        )
        .unwrap();
    vm.call(f, Value::UNDEFINED, &[Value::int(5000)]).unwrap();
    let mut lines = Vec::new();
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, fn_id } if fn_id == LOG => {
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle => break,
            s => panic!("{s:?}"),
        }
    }
    assert_eq!(lines, ["first sleeps 5000 ms"]);
    let run = vm
        .outcome()
        .and_then(|r| r.ok())
        .unwrap_or(Value::UNDEFINED);
    let name = |id: u32| match id {
        LOG => Some("console.log".into()),
        SLEEP => Some("tools.sleep".into()),
        _ => None,
    };
    let call = *vm.pending_host_calls().first().expect("the open sleep");
    let opts = super::dump::DumpOptions {
        base: base(),
        host: &["console", "tools"],
        host_roots: &[],
        host_fn_name: &name,
        handles: &[("run", run)],
        call_states: &[(call, "{\"due\":1793822825106}")],
        source_refs: true,
    };
    let text = vm.dump(&opts, None).map_err(|e| e.message()).unwrap();
    assert!(
        text.contains("$djs.resume(2, \"tools.sleep\", [5000], {\"due\":1793822825106})"),
        "{text}"
    );
    // The body by reference (D2882), not its text again.
    let hash = super::dump::script_hash(SCRIPT, "ms", {
        let opts = crate::compiler::CompileOptions {
            goal: crate::compiler::Goal::AsyncBody,
            params: &["ms"],
            ..crate::compiler::CompileOptions::script("sleep.js")
        };
        opts.bits()
    });
    assert!(
        text.contains(&alloc::format!(
            "$djs.source(\"sleep.js\", \"{hash}\", \"ms\")"
        )),
        "{text}"
    );
    assert_eq!(text.matches("results.join").count(), 1, "{text}");
    if let Ok(dir) = std::env::var("DJS_SLEEP_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("sleep.dump.js"), &text).unwrap();
    }
    // A host that has the script loads it, continues and dumps again by reference.
    const SOURCE: u32 = 9;
    const RESUME: u32 = 10;
    const SEED_HOOK: u32 = 11;
    const HOST_HOOK: u32 = 12;
    let mut vm = Vm::from_base(
        base(),
        VmOptions {
            frozen_builtins: true,
            ..VmOptions::default()
        },
    )
    .unwrap();
    vm.set_now(1_793_822_820_106.0);
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let tools = vm.new_object().unwrap();
    let sleep = vm.new_async_host_function(SLEEP, "sleep").unwrap();
    vm.define(tools, "sleep", sleep).unwrap();
    vm.define_global("tools", Value::object(tools)).unwrap();
    let rt = vm.dump_runtime().unwrap().as_object().unwrap();
    let source = vm.new_host_function(SOURCE, "source").unwrap();
    vm.define(rt, "source", source).unwrap();
    let resume = vm.new_host_function(RESUME, "resume").unwrap();
    vm.define(rt, "resume", resume).unwrap();
    let seed = vm.new_host_function(SEED_HOOK, "seed").unwrap();
    vm.define(rt, "seed", seed).unwrap();
    let host = vm.new_host_function(HOST_HOOK, "host").unwrap();
    vm.define(rt, "host", host).unwrap();
    vm.define_global("$djs", Value::object(rt)).unwrap();
    let f = vm
        .compile(
            text.as_bytes(),
            &crate::compiler::CompileOptions::script("dump.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut lines = Vec::new();
    let mut slept = 0;
    let mut dumped = None;
    loop {
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, fn_id } if fn_id == SOURCE => {
                let args = vm.host_call(call_id).unwrap().args;
                let h = vm.display(args[1]);
                let opts = crate::compiler::CompileOptions {
                    goal: crate::compiler::Goal::AsyncBody,
                    params: &["ms"],
                    ..crate::compiler::CompileOptions::script("sleep.js")
                };
                let r = vm.source_function(SCRIPT.as_bytes(), &opts, &h);
                vm.resolve_host_call(call_id, Ok(r.unwrap())).unwrap();
            }
            Status::HostCall { call_id, fn_id } if fn_id == RESUME => {
                vm.open_host_call(call_id, SLEEP).unwrap();
            }
            Status::HostCall { call_id, fn_id } if fn_id == HOST_HOOK => {
                let r = super::dump_tests::serve_host_hook(&mut vm, call_id);
                vm.resolve_host_call(call_id, r).unwrap();
            }
            Status::HostCall { call_id, fn_id } if fn_id == SEED_HOOK => {
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::HostCall { call_id, fn_id } if fn_id == LOG => {
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle => {
                let Some(&id) = vm.pending_host_calls().first() else {
                    break;
                };
                slept += 1;
                if slept == 2 && dumped.is_none() {
                    let opts = super::dump::DumpOptions {
                        base: base(),
                        host: &["console", "tools"],
                        host_fn_name: &name,
                        source_refs: true,
                        ..Default::default()
                    };
                    dumped = Some(vm.dump(&opts, None).map_err(|e| e.message()).unwrap());
                }
                vm.resolve_host_call(id, Ok(Value::int(1))).unwrap();
            }
            Status::Done(_) => break,
            s => panic!("{s:?}"),
        }
    }
    assert_eq!(
        lines,
        [
            "second sleeps 5000 ms",
            "done after less than 10000 ms: first: 1, second: 1"
        ]
    );
    let again = dumped.expect("a second dump");
    assert!(
        again.contains(&alloc::format!(
            "$djs.source(\"sleep.js\", \"{hash}\", \"ms\")"
        )),
        "{again}"
    );
    assert_eq!(again.matches("results.join").count(), 1, "{again}");
}

/// Operands evaluated before the site are read again where that gives the saved value
/// (D2836, dump_reread.rs): a builtin method of a session object, a binding, a global; and
/// kept where it would not: a receiver reassigned after its read, a shadowed method, a
/// user function on a user object, a getter.
#[test]
fn plain_dumps_read_operands_again() {
    let src = r#"
const log = [];
async function builtin() { const a = []; a.push(await later(1)); return a.join(); }
async function binding() { const f = (x, y) => x + y; const k = 2; return f(k, await later(3)); }
async function global() { return Math.max(1, await later(5)); }
async function reassigned() { let a = [1]; const b = a; a.push(await (a = [9], later(2))); return b.join() + "/" + a.join(); }
async function shadowed() { const a = []; a.push = (x) => log.push("own " + x); a.push(await later(4)); return log.join(); }
async function userMethod() { const o = { m(x) { return "m" + x; } }; return o.m(await later(6)); }
async function getter() { const o = { get m() { log.push("get"); return (x) => x + log.length; } }; return o.m(await later(7)); }
async function main() {
  for (const f of [builtin, binding, global, reassigned, shadowed, userMethod, getter]) print(f.name, await f());
}
main();
"#;
    let (failed, refused, ok) = plain_check_program("reread", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    let (failed, _, refused) = plain_chain_program("reread", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ok >= 7, "{ok}");
    let at = |k: usize| plain_dump_at(src, k);
    assert!(at(1).contains("a.push(await "), "{}", at(1));
    assert!(at(2).contains("f(k, await "), "{}", at(2));
    assert!(at(3).contains("Math.max(1, await "), "{}", at(3));
    for k in 4..=7 {
        assert!(at(k).contains("$djs_R("), "{k}: {}", at(k));
    }
}

/// The rest of the current iteration before the loop, then the loop over the saved
/// record (D2908): an array's iterator, a for-in's keys, a loop variable the rest reads,
/// two awaits in one iteration (a re-dump inside the rest and inside the loop); kept as
/// the placeholder scaffold where the body jumps (`break`, `continue`) or the iterator has
/// a `return` (a generator).
#[test]
fn plain_dumps_continue_loops_after_the_rest() {
    let src = r#"
async function arr() { const out = []; for (const x of [1, 2, 3]) { out.push(await later(x)); out.push(x * 10); } return out.join(); }
async function twice() { let s = ""; for (const x of ["a", "b"]) { s += await later(x); s += await later(x + "!"); } return s; }
async function keys() { const o = { p: 1, q: 2 }; let s = ""; for (const k in o) s += k + (await later(o[k])); return s; }
async function jumps() { let s = 0; for (const x of [1, 2, 3]) { if (x === 2) continue; s += await later(x); if (x === 3) break; } return s; }
function* gen() { try { yield 1; yield 2; } finally { print("closed"); } }
async function generator() { let s = 0; for (const x of gen()) s += await later(x); return s; }
async function main() {
  for (const f of [arr, twice, keys, jumps, generator]) print(f.name, await f());
}
main();
"#;
    let (failed, refused, ok) = plain_check_program("rest", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    let (failed, _, refused) = plain_chain_program("rest", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ok >= 12, "{ok}");
    let first = plain_dump_at(src, 1);
    assert!(first.contains("of $djs_I($djs_i"), "{first}");
}

/// The body's bindings are the continuation's parameters, their values its call's
/// arguments (D2909); a constant the text may assign stays a constant (the assignment
/// throws), as does one the text destructures into; `arguments` stays K's own binding.
#[test]
fn plain_dumps_take_the_frame_as_arguments() {
    let src = r#"
async function f(a, b) {
  let n = a + 1;
  const c = [b];
  const k = 3;
  const d = 4;
  await later(1);
  n += k;
  c.push(arguments.length);
  try { d = 5; } catch (e) { print(e.name); }
  try { [k] = [7]; } catch (e) { print(e.name); }
  print(n, c.join(), k, d);
}
f(1, 2);
"#;
    let (failed, refused, ok) = plain_check_program("params", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    let (failed, _, refused) = plain_chain_program("params", src);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ok >= 1, "{ok}");
    let d = plain_dump_at(src, 1);
    // (`a` and `b` are dead after the site, D2911.)
    assert!(d.contains("async function (n, c) {"), "{d}");
    assert!(
        d.contains("const k = 3;") && d.contains("const d = 4;"),
        "{d}"
    );
}

/// A continuation's record holds what makes its layout again (D2913): its origin, its
/// site, the decisions taken on values (`r`) and its holes (`h`), no layout. A dump of the
/// loaded dump writes the original again; one whose continuation's text is not the one its
/// origin makes (an edit in the copied text) is refused.
#[test]
fn plain_records_make_the_layout_again() {
    let src = r#"
async function f(xs) {
  const a = [await later(1)];
  for (const x of xs) a.push(await later(x));
  return a.join();
}
f([2, 3]).then(print);
"#;
    let d = plain_dump_at(src, 2);
    assert!(!d.contains("segs:") && !d.contains("scaf:"), "{d}");
    assert!(d.contains("{ f: f, at: \"await#2\""), "{d}");
    let again = |text: &str| -> Result<String, String> {
        let mut vm = plain_load(text)?;
        let mut lines = Vec::new();
        let id = plain_step(&mut vm, &mut lines).expect("an idle point");
        settle(&mut vm, id);
        plain_step(&mut vm, &mut lines).expect("an idle point");
        plain_dump(&vm)
    };
    let next = again(&d).unwrap();
    assert_eq!(next.matches("return a.join();").count(), 2, "{next}");
    let edited = d.replacen("return a.join();\n}, {", "return  a.join();\n}, {", 1);
    assert_ne!(edited, d);
    let e = again(&edited).unwrap_err();
    assert!(
        e.contains("its text is not the one its origin makes"),
        "{e}"
    );
}
