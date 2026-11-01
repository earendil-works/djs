//! `djs run [--body] [--ts] [--gc-stress] [--suspend-every N] [--backend B] <file.js>`: run a script,
//! serving `console.log` as a sync host call.
//!
//! Exit codes: 0 ok, 1 uncaught exception (printed as `Uncaught Name: message`), 2 usage
//! or compile error.

use std::io::Write;

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const USAGE: &str = "usage: djs run [--body] [--ts] [--gc-stress] [--suspend-every N] [--backend B] <file.js>\n\
  --body             run as a codemode async body with parameters (tools, console)\n\
  --ts               parse as TypeScript (the default for .ts files)\n\
  --gc-stress        collect garbage at every safepoint\n\
  --suspend-every N  run N fuel at a time; at every stop, continue in a fresh VM\n\
                     restored from the heap image\n\
  --backend B        run the @native builtins as `native` code (default) or `bytecode`";

pub fn main(args: &[String]) -> i32 {
    let mut opts = RunOptions::default();
    let mut path = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--body" => opts.body = true,
            "--ts" => opts.typescript = true,
            "--gc-stress" => opts.gc_stress = true,
            "--backend" => match it.next().map(|s| s.as_str()) {
                Some("native") => opts.bytecode_builtins = false,
                Some("bytecode") => opts.bytecode_builtins = true,
                _ => {
                    eprintln!("{USAGE}");
                    return 2;
                }
            },
            "--suspend-every" => match it.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => opts.suspend_every = Some(n),
                _ => {
                    eprintln!("{USAGE}");
                    return 2;
                }
            },
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            _ if path.is_none() && !a.starts_with('-') => path = Some(a.clone()),
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return 2;
    };
    if path.ends_with(".ts") || path.ends_with(".mts") {
        opts.typescript = true;
    }
    let src = match std::fs::read(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("djs: cannot read {path}: {e}");
            return 2;
        }
    };
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    run_source(&src, &path, &opts, &mut stdout.lock(), &mut stderr.lock())
}

/// Compiles and runs `src`. See the module docs for the exit codes.
pub fn run_source(
    src: &[u8],
    filename: &str,
    opts: &RunOptions,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut vm = new_vm(opts);
    let params = ["tools", "console"];
    let copts = if opts.body {
        CompileOptions::async_body(filename, &params)
    } else {
        CompileOptions::script(filename)
    };
    let copts = CompileOptions {
        typescript: opts.typescript,
        ..copts
    };
    let main = match vm.compile(src, &copts) {
        Ok(f) => f,
        Err(e) => {
            match e.kind {
                CompileErrorKind::Syntax => {
                    let _ = writeln!(
                        err,
                        "SyntaxError: {} at {filename}:{}:{}",
                        e.message, e.line, e.col
                    );
                }
                CompileErrorKind::OutOfMemory => {
                    let _ = writeln!(err, "djs: out of memory while compiling");
                }
            }
            return 2;
        }
    };
    let args: Vec<Value> = if opts.body {
        // `tools` is not provided by `djs run`; `console` is the global console object.
        let key = vm.heap_mut().atom_str("console").expect("oom");
        let console = vm
            .heap()
            .find_property(vm.global(), key)
            .and_then(|(_, p)| p.value())
            .unwrap_or(Value::UNDEFINED);
        vec![Value::UNDEFINED, console]
    } else {
        Vec::new()
    };
    run_main(vm, main, &args, opts, out, err)
}

/// Host function ids registered by `djs run`.
pub const CONSOLE_LOG: u32 = 1;

/// Options of `djs run`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    /// Compile as a codemode async body (D008) instead of a Script.
    pub body: bool,
    /// Collect at every safepoint.
    pub gc_stress: bool,
    /// Run with this much fuel at a time; on `OutOfFuel`, rebuild the VM from its heap
    /// image and continue (the suspension stress mode).
    pub suspend_every: Option<u64>,
    /// Parse as TypeScript.
    pub typescript: bool,
    /// Run the `@native` builtins as bytecode (`--backend bytecode`, 0063).
    pub bytecode_builtins: bool,
}

impl RunOptions {
    pub fn vm_options(&self) -> VmOptions {
        VmOptions {
            exec: ExecOptions {
                gc_stress: self.gc_stress,
                native_builtins: !self.bytecode_builtins,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// A VM with the globals `djs run` provides (`console.log`).
pub fn new_vm(opts: &RunOptions) -> Vm {
    let mut vm = djs_prelude::new_vm(opts.vm_options()).expect("out of memory creating the VM");
    // The VM reads no clock itself (D1383): give it the wall clock, in UTC.
    vm.set_now(now_ms());
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
    args: &[Value],
    opts: &RunOptions,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    // The top level is called with `this` = the global object in both modes (D1140).
    let this = Value::object(vm.global());
    vm.call(main, this, args).expect("the VM is idle");
    let fuel = opts.suspend_every.unwrap_or(u64::MAX);
    loop {
        match vm.run(fuel) {
            Status::Done(_) | Status::Idle => {
                // Like qjs: unhandled rejections are reported once the jobs ran out.
                let rejections = vm.take_unhandled_rejections();
                for &r in &rejections {
                    let msg = vm.display(r);
                    let _ = writeln!(err, "Possibly unhandled promise rejection: {msg}");
                    if let Some(stack) = vm.error_stack(r) {
                        let _ = write!(err, "{stack}");
                    }
                }
                return i32::from(!rejections.is_empty());
            }
            Status::Threw(e) => {
                let msg = vm.display(e);
                let _ = writeln!(err, "Uncaught {msg}");
                if let Some(stack) = vm.error_stack(e) {
                    let _ = write!(err, "{stack}");
                }
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

/// The system clock in ms since the epoch.
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
