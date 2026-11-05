//! Doc-tests and disassembly: a page's `js` blocks are compiled and run by this build of
//! djs, so the output and bytecode in the guide are always what djs does.
//!
//! Examples are codemode async function bodies (D008), as in pi and in the live examples
//! (`Sandbox.run`): top-level `await` and `return` work, and `console.log` prints. The
//! VM starts from the base snapshot, with the clock at 0. The bodies have no parameters:
//! `console` is a global, as a host binds it (D2972).

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{CompileOptions, compile};
use djs_core::heap::Heap;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

const CONSOLE_LOG: u32 = 1;

/// Runs `src`; returns what it printed, then `Uncaught ...` if it threw.
pub fn run(src: &str) -> String {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).expect("out of memory");
    vm.set_now(0.0);
    let console = vm.new_object().expect("out of memory");
    let log = vm
        .new_host_function(CONSOLE_LOG, "log")
        .expect("out of memory");
    vm.define(console, "log", log).expect("out of memory");
    vm.define_global("console", Value::object(console))
        .expect("out of memory");
    let main = match vm.compile(
        src.as_bytes(),
        &CompileOptions::async_body("example.js", &[]),
    ) {
        Ok(f) => f,
        Err(e) => return format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &[]).expect("the VM is idle");
    let mut out = String::new();
    // Suspend every SLICE units of fuel and resume in a new VM from the heap image, like
    // the gate's stress mode: an example also proves that it survives suspension.
    // Examples are small; MAX_FUEL only stops a runaway loop.
    const SLICE: u64 = 50;
    const MAX_FUEL: u64 = 1_000_000;
    let mut spent = 0;
    loop {
        match vm.run(SLICE) {
            Status::Done(_) | Status::Idle => break,
            Status::OutOfFuel if spent >= MAX_FUEL => return out + "(out of fuel)\n",
            Status::OutOfFuel => {
                spent += SLICE;
                let o = vm.options();
                vm = Vm::from_bytes(vm.image().to_vec(), o).expect("a valid heap image");
            }
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

/// Host function ids of [`dump`]'s VMs.
const DEBUGGER: u32 = 2;

/// The continuation dump `src` writes at its `nth` pause (from 1): a `debugger;` in an
/// async function awaits the host's `debugger` there (D3050), and the run is dumped while
/// it waits, as the live examples' Dump tab does with the Sandbox: frozen builtins,
/// `console` a namespace of host functions, the body by reference (`$djs.source`), the
/// run's promise as the dump's value. The text, or why there is none.
pub fn dump(src: &str, nth: usize) -> String {
    // As the editor has it: without the block's final newline, which the script's hash in
    // `$djs.source` would count.
    let src = src.strip_suffix('\n').unwrap_or(src);
    use djs_core::vm::dump::{BaseNames, DumpOptions};
    let mut vm = djs_prelude::new_vm(VmOptions {
        frozen_builtins: true,
        ..VmOptions::default()
    })
    .expect("out of memory");
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
        ..CompileOptions::async_body("example.js", &[])
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
                let run = match vm.outcome() {
                    Some(Ok(v)) => v,
                    _ => return "(the run has no result promise)\n".into(),
                };
                let name = |id: u32| match id {
                    CONSOLE_LOG => Some("console.log".to_string()),
                    DEBUGGER => Some("debugger".to_string()),
                    _ => None,
                };
                let roots = [("console", Value::object(console))];
                let handles = [("run", run)];
                let opts = DumpOptions {
                    base: djs_prelude::BASE,
                    host: &["console", "debugger"],
                    host_roots: &roots,
                    host_fn_name: &name,
                    handles: &handles,
                    call_states: &[],
                    source_refs: true,
                    base_names: Some(BaseNames {
                        builtins: djs_prelude::BASE_NAMES,
                        globals: djs_prelude::BASE_GLOBALS,
                    }),
                    check: true,
                };
                return match vm.dump(&opts, None) {
                    Ok(text) => text,
                    Err(e) => format!("(refused: {})\n", e.message()),
                };
            }
            Status::Done(_) => return "(the run ended without pausing)\n".into(),
            Status::Threw(e) => return format!("Uncaught {}\n", vm.display(e)),
            Status::HostOutOfMemory => return "out of host memory\n".into(),
        }
    }
}

/// The bytecode of `src` and of every function in it.
pub fn disasm(src: &str) -> String {
    let mut heap = Heap::new().expect("out of memory");
    match compile(
        &mut heap,
        src.as_bytes(),
        &CompileOptions::async_body("example.js", &[]),
    ) {
        Ok(t) => disasm_template(&heap, t),
        Err(e) => format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_print_await_and_throw() {
        assert_eq!(run("console.log(1, 'a', [2])"), "1 a [2]\n");
        assert_eq!(
            run("const x = await Promise.resolve(3); console.log(x)"),
            "3\n"
        );
        assert_eq!(
            run("console.log(1); null.x"),
            "1\nUncaught TypeError: cannot read property 'x' of null\n"
        );
        assert!(
            run("let s = 0; for (let i = 0; i < 1000; i++) s += i; console.log(s)")
                .ends_with("499500\n")
        );
        assert!(run("let").starts_with("SyntaxError"));
        assert!(disasm("return 1").contains("return"));
    }

    #[test]
    fn examples_dump_at_debugger() {
        let src = "let n = 0;\nfor (const x of [1, 2]) {\n  n += x;\n  debugger;\n}\nreturn n;\n";
        let d = dump(src, 1);
        assert!(d.contains("$djs.resume("), "{d}");
        assert!(d.contains("\"debugger\""), "{d}");
        assert!(dump(src, 2).contains("$djs.resume("));
        assert_eq!(dump(src, 3), "(the run ended without pausing)\n");
    }
}
