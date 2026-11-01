//! Doc-tests and disassembly: a page's `js` blocks are compiled and run by this build of
//! djs, so the output and bytecode in the guide are always what djs does.
//!
//! Examples are codemode async function bodies (D008), as in pi and in the live examples
//! (`Sandbox.run`): top-level `await` and `return` work, and `console.log` prints. The
//! VM starts from the base snapshot, with the clock at 0.

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{CompileOptions, compile};
use djs_core::heap::Heap;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

const CONSOLE_LOG: u32 = 1;
const PARAMS: [&str; 2] = ["tools", "console"];

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
        &CompileOptions::async_body("example.js", &PARAMS),
    ) {
        Ok(f) => f,
        Err(e) => return format!("SyntaxError: {} at {}:{}\n", e.message, e.line, e.col),
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &[Value::UNDEFINED, Value::object(console)])
        .expect("the VM is idle");
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
                vm = Vm::from_bytes(vm.bytes().to_vec(), o).expect("a valid heap image");
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

/// The bytecode of `src` and of every function in it.
pub fn disasm(src: &str) -> String {
    let mut heap = Heap::new().expect("out of memory");
    match compile(
        &mut heap,
        src.as_bytes(),
        &CompileOptions::async_body("example.js", &PARAMS),
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
}
