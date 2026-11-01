//! `djs run`: run a script, serving `console.log` as a sync host call.

use std::io::Write;

use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

/// Host function ids registered by `djs run`.
pub const CONSOLE_LOG: u32 = 1;

/// Options of `djs run`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    /// Collect at every safepoint.
    pub gc_stress: bool,
    /// Run with this much fuel at a time; on `OutOfFuel`, rebuild the VM from its heap
    /// image and continue (the suspension stress mode).
    pub suspend_every: Option<u64>,
}

impl RunOptions {
    pub fn vm_options(&self) -> VmOptions {
        VmOptions {
            exec: ExecOptions {
                gc_stress: self.gc_stress,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// A VM with the globals `djs run` provides (`console.log`).
pub fn new_vm(opts: &RunOptions) -> Vm {
    let mut vm = Vm::new(opts.vm_options()).expect("out of memory creating the VM");
    let console = vm.new_object().expect("oom");
    let log = vm.new_host_function(CONSOLE_LOG, "log").expect("oom");
    vm.define(console, "log", log).expect("oom");
    vm.define_global("console", Value::object(console))
        .expect("oom");
    vm
}

/// Calls `main` and runs it to completion. Output goes to `out`; an uncaught exception
/// is printed to `err`. Returns the process exit code.
pub fn run_main(
    mut vm: Vm,
    main: Value,
    opts: &RunOptions,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    // Script mode: the top-level `this` is the global object (D004).
    let this = Value::object(vm.global());
    vm.call(main, this, &[]).expect("the VM is idle");
    let fuel = opts.suspend_every.unwrap_or(u64::MAX);
    loop {
        match vm.run(fuel) {
            Status::Done(_) | Status::Idle => return 0,
            Status::Threw(e) => {
                let msg = vm.display(e);
                let _ = writeln!(err, "Uncaught {msg}");
                return 1;
            }
            Status::OutOfFuel => {
                // Everything is in the heap: continue in a fresh VM built from the image.
                let o = vm.options();
                vm = Vm::from_bytes(vm.bytes().to_vec(), o).expect("valid heap image");
            }
            Status::HostCall { call_id, fn_id } => {
                let info = vm.host_call(call_id).expect("parked call");
                let result = match fn_id {
                    CONSOLE_LOG => {
                        let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                        let _ = writeln!(out, "{}", parts.join(" "));
                        Ok(Value::UNDEFINED)
                    }
                    _ => Err(Value::UNDEFINED),
                };
                vm.resolve_host_call(call_id, result)
                    .expect("parked on this call");
            }
        }
    }
}
