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
        host_fn_name: &name,
        handles: &[("run", run)],
        call_states: &[(call, "{\"due\":1793822825106}")],
    };
    let text = vm.dump(&opts, None).map_err(|e| e.message()).unwrap();
    assert!(
        text.contains("$djs.resume(2, \"tools.sleep\", [5000], {\"due\":1793822825106})"),
        "{text}"
    );
    if let Ok(dir) = std::env::var("DJS_SLEEP_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("sleep.dump.js"), &text).unwrap();
    }
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
    assert!(at(1).contains("a.push((await "), "{}", at(1));
    assert!(at(2).contains("f(k, (await "), "{}", at(2));
    assert!(at(3).contains("Math.max(1, (await "), "{}", at(3));
    for k in 4..=7 {
        assert!(at(k).contains("$djs_R("), "{k}: {}", at(k));
    }
}
