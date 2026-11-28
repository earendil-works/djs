//! Doc-tests and disassembly: a page's `js` blocks are compiled and run by this build of
//! djs, so the output and bytecode in the guide are always what djs does.
//!
//! Examples are codemode async function bodies (D008), as in pi and in the live examples
//! (`Sandbox.run`): top-level `await` and `return` work, and `console.log` prints. The
//! VM starts from the base image, with the clock at 0. The bodies have no parameters:
//! `console` is a global, as a host binds it (D2972).

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{CompileOptions, compile};
use djs_core::heap::Heap;
use djs_core::value::Value;
use djs_core::vm::{Status, VmOptions};

const CONSOLE_LOG: u32 = 1;

/// The compile options of an example: a codemode body, TypeScript for a `ts` block.
fn body_options(ts: bool) -> CompileOptions<'static> {
    CompileOptions {
        typescript: ts,
        ..CompileOptions::async_body("example.js", &[])
    }
}

/// Runs `src` (TypeScript if `ts`); returns what it printed, then `Uncaught ...` if it threw.
pub fn run(src: &str, ts: bool) -> String {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).expect("out of memory");
    vm.set_now(0.0);
    let console = vm.new_object().expect("out of memory");
    let log = vm
        .new_host_function(CONSOLE_LOG, "log")
        .expect("out of memory");
    vm.define(console, "log", log).expect("out of memory");
    vm.define_global("console", Value::object(console))
        .expect("out of memory");
    let main = match vm.compile(src.as_bytes(), &body_options(ts)) {
        Ok(f) => f,
        Err(e) => return format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &[]).expect("the VM is idle");
    let mut out = String::new();
    // Stop every SLICE units of fuel and go on: an example also shows that its output does
    // not depend on where a run stops. Examples are small; MAX_FUEL only stops a runaway
    // loop.
    const SLICE: u64 = 50;
    const MAX_FUEL: u64 = 1_000_000;
    let mut spent = 0;
    loop {
        match vm.run(SLICE) {
            Status::Done(_) | Status::Idle => break,
            Status::OutOfFuel if spent >= MAX_FUEL => return out + "(out of fuel)\n",
            Status::OutOfFuel => spent += SLICE,
            Status::Threw(e) => {
                out.push_str(&format!("Uncaught {}\n", vm.display(e)));
                return out;
            }
            Status::HostOutOfMemory => return out + "out of host memory\n",
            Status::HostCall { call_id, fn_id } => {
                let info = vm.host_call(call_id).expect("a parked call");
                if fn_id == CONSOLE_LOG {
                    let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                    out.push_str(&parts.join(" "));
                    out.push('\n');
                }
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED))
                    .expect("parked on this call");
            }
        }
    }
    // The body's promise rejected: the script threw.
    for r in vm.take_unhandled_rejections() {
        out.push_str(&format!("Uncaught {}\n", vm.display(r)));
    }
    out
}

/// Host function ids of [`snapshot`]'s VMs.
const DEBUGGER: u32 = 2;

/// The snapshot `src` is at its `nth` pause (from 1), described (`snapshot::describe`: the
/// run as a tree from its open calls): a `debugger;` in an async function awaits the host's
/// `debugger` there (D3050), and the run is snapshotted while it waits, as the live
/// examples do with the Sandbox (D3720): `console` and `debugger` the host's values, the
/// body by its hash. The text, or why there is none.
pub fn snapshot(src: &str, nth: usize, ts: bool) -> String {
    use djs_core::vm::snapshot::{BaseInfo, HostNames, SnapshotOptions, describe};
    // As the editor has it: without the block's final newline.
    let src = src.strip_suffix('\n').unwrap_or(src);
    let mut vm = djs_prelude::new_vm(VmOptions::default()).expect("out of memory");
    vm.set_now(0.0);
    let console = vm.new_object().expect("out of memory");
    let log = vm
        .new_host_function(CONSOLE_LOG, "log")
        .expect("out of memory");
    vm.define(console, "log", log).expect("out of memory");
    vm.define_global("console", Value::object(console))
        .expect("out of memory");
    let pause = vm
        .new_async_host_function(DEBUGGER, "debugger")
        .expect("out of memory");
    vm.define_global("debugger", pause).expect("out of memory");
    let opts = CompileOptions {
        debugger_awaits: true,
        ..body_options(ts)
    };
    let main = match vm.compile(src.as_bytes(), &opts) {
        Ok(f) => f,
        Err(e) => return format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    };
    // (`this` undefined, as the Sandbox calls a body.)
    vm.call(main, Value::UNDEFINED, &[])
        .expect("the VM is idle");
    let mut pauses = 0;
    let mut spent = 0;
    loop {
        match vm.run(1_000_000) {
            Status::OutOfFuel if spent < 20 => spent += 1,
            Status::OutOfFuel => return "(out of fuel)\n".into(),
            Status::HostCall { call_id, .. } => {
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED))
                    .expect("parked on this call");
            }
            Status::Idle => {
                let open = vm.pending_host_calls();
                let Some(&id) = open.first() else {
                    return "(the run waits for nothing)\n".into();
                };
                pauses += 1;
                if pauses < nth {
                    vm.resolve_host_call(id, Ok(Value::UNDEFINED))
                        .expect("an open call");
                    continue;
                }
                let base =
                    BaseInfo::new(djs_prelude::BASE, VmOptions::default()).expect("the base loads");
                let host = HostNames {
                    functions: vec![
                        (CONSOLE_LOG, "console.log".to_string()),
                        (DEBUGGER, "debugger".to_string()),
                    ],
                    values: vec![("console".to_string(), Value::object(console))],
                    ..Default::default()
                };
                // With the script's text, as the Sandbox writes snapshots.
                let opts = SnapshotOptions {
                    sources: true,
                    host: Some(host),
                };
                return match vm.snapshot_with(&base, &opts) {
                    Ok(bytes) => describe(&bytes).unwrap_or_else(|e| e.0.join("; ")),
                    Err(e) => format!("(refused: {})\n", e.0.join("; ")),
                };
            }
            Status::Done(_) => return "(the run ended without pausing)\n".into(),
            Status::Threw(e) => return format!("Uncaught {}\n", vm.display(e)),
            Status::HostOutOfMemory => return "out of host memory\n".into(),
        }
    }
}

/// The bytecode of `src` and of every function in it.
pub fn disasm(src: &str, ts: bool) -> String {
    let mut heap = Heap::new().expect("out of memory");
    match compile(&mut heap, src.as_bytes(), &body_options(ts)) {
        Ok(t) => disasm_template(&heap, t),
        Err(e) => format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_print_await_and_throw() {
        assert_eq!(run("console.log(1, 'a', [2])", false), "1 a [2]\n");
        assert_eq!(
            run("const x = await Promise.resolve(3); console.log(x)", false),
            "3\n"
        );
        assert_eq!(
            run("console.log(1); null.x", false),
            "1\nUncaught TypeError: cannot read property 'x' of null\n"
        );
        assert!(
            run(
                "let s = 0; for (let i = 0; i < 1000; i++) s += i; console.log(s)",
                false
            )
            .ends_with("499500\n")
        );
        assert!(run("let", false).starts_with("SyntaxError"));
        // A `ts` block is TypeScript.
        assert_eq!(
            run(
                "enum E { A, B } const n: number = E.B; console.log(n)",
                true
            ),
            "1\n"
        );
        assert!(run("const n: number = 1", false).starts_with("SyntaxError"));
        assert!(disasm("return 1", false).contains("return"));
    }

    #[test]
    fn examples_snapshot_at_debugger() {
        let src = "let n = 0;\nfor (const x of [1, 2]) {\n  n += x;\n  debugger;\n}\nreturn n;\n";
        let d = snapshot(src, 1, false);
        assert!(d.contains("open call 1: debugger()"), "{d}");
        assert!(d.contains("the body waits at 4:3"), "{d}");
        assert!(d.contains("n = 1"), "{d}");
        assert!(snapshot(src, 2, false).contains("n = 3"));
        assert_eq!(snapshot(src, 3, false), "(the run ended without pausing)\n");
    }
}
