//! `djs run [--body] [--ts] [--experimental-decorators] [--emit-decorator-metadata]
//! [--gc-stress] [--suspend-every N] [--backend B] [--tools answers.json] <file.js>`: run
//! a script, serving `console.log` and `print` as sync host calls, `later(v)` and
//! `tools.x()` as async host calls (0174, D2602): `later` is answered with its argument,
//! tools from `--tools answers.json`, one call at every idle point.
//!
//! `djs resume [same options] <dump.js>` loads a dump into a fresh VM and runs it on the
//! same way (D2603).
//!
//! Exit codes: 0 ok, 1 uncaught exception (printed as `Uncaught Name: message`) or a tool
//! call the answers file has no answer for, 2 usage or compile error.

use std::io::Write;

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

use crate::tools::{MAKE_TOOL, Tools};

const USAGE: &str = "usage: djs run [--body] [--ts] [--experimental-decorators]\n\
                [--emit-decorator-metadata] [--gc-stress] [--suspend-every N] [--backend B]\n\
                [--tools answers.json] <file.js>\n\
  --body             run as a codemode async body with parameters (tools, console)\n\
  --ts               parse as TypeScript (the default for .ts files)\n\
  --experimental-decorators\n\
                     TypeScript legacy decorators (experimentalDecorators)\n\
  --emit-decorator-metadata\n\
                     legacy decorators with emitDecoratorMetadata\n\
  --gc-stress        collect garbage at every safepoint\n\
  --suspend-every N  run N fuel at a time; at every stop, continue in a fresh VM\n\
                     restored from the heap image\n\
  --backend B        run the @native builtins as `native` code (default) or `bytecode`\n\
  --tools FILE       answer `tools.x()` calls from FILE, one at a time whenever the\n\
                     run is idle (docs/cli.md)\n\
  --dump-after K     write a format 2 dump at idle point K and stop (needs 0172)\n\
  --dump-every-idle  dump and reload at every idle point (needs 0172)";

const RESUME_USAGE: &str = "usage: djs resume [--tools answers.json] [--gc-stress] [--suspend-every N]\n\
                [--backend B] <dump.js>\n\
  loads a dump (`// djs-dump 1` or `2`) into a fresh VM and runs it to its end, answering\n\
  open calls as `djs run` does";

/// What format 2 dumps of a running session still need.
const NEEDS_EXPORTER: &str = "djs: dumping a waiting run needs the format 2 exporter (issue 0172), \
which this build does not have yet";

pub fn main(args: &[String]) -> i32 {
    command(args, false)
}

/// `djs resume`.
pub fn resume_main(args: &[String]) -> i32 {
    command(args, true)
}

fn command(args: &[String], resume: bool) -> i32 {
    let usage = if resume { RESUME_USAGE } else { USAGE };
    let mut opts = RunOptions::default();
    let mut path = None;
    let mut tools = Tools::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--body" => opts.body = true,
            "--ts" => opts.typescript = true,
            "--experimental-decorators" => opts.experimental_decorators = true,
            "--emit-decorator-metadata" => {
                opts.experimental_decorators = true;
                opts.emit_decorator_metadata = true;
            }
            "--gc-stress" => opts.gc_stress = true,
            "--dump-after" | "--dump-every-idle" if !resume => {
                eprintln!("{NEEDS_EXPORTER}");
                return 2;
            }
            "--tools" => match it.next().map(|p| crate::tools::read_answers(p)) {
                Some(Ok(a)) => tools = Tools::with_answers(Some(a)),
                Some(Err(e)) => {
                    eprintln!("djs: {e}");
                    return 2;
                }
                None => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--backend" => match it.next().map(|s| s.as_str()) {
                Some("native") => opts.bytecode_builtins = false,
                Some("bytecode") => opts.bytecode_builtins = true,
                _ => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--suspend-every" => match it.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => opts.suspend_every = Some(n),
                _ => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "-h" | "--help" => {
                println!("{usage}");
                return 0;
            }
            _ if path.is_none() && !a.starts_with('-') => path = Some(a.clone()),
            _ => {
                eprintln!("{usage}");
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{usage}");
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
    if resume {
        let text = String::from_utf8_lossy(&src);
        return resume_dump(
            &text,
            &path,
            &opts,
            &mut tools,
            &mut stdout.lock(),
            &mut stderr.lock(),
        );
    }
    run_source(
        &src,
        &path,
        &opts,
        &mut tools,
        &mut stdout.lock(),
        &mut stderr.lock(),
    )
}

/// Compiles and runs `src`. See the module docs for the exit codes.
pub fn run_source(
    src: &[u8],
    filename: &str,
    opts: &RunOptions,
    tools: &mut Tools,
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
        experimental_decorators: opts.experimental_decorators,
        emit_decorator_metadata: opts.emit_decorator_metadata,
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
                CompileErrorKind::HostOutOfMemory => {
                    let _ = writeln!(err, "djs: out of host memory while compiling");
                }
            }
            return 2;
        }
    };
    let args: Vec<Value> = if opts.body {
        // `tools` (with `--tools`) and `console` are the globals.
        let mut global = |name: &str| {
            let key = vm.heap_mut().atom_str(name).expect("oom");
            vm.heap()
                .find_property(vm.global(), key)
                .and_then(|(_, p)| p.value())
                .unwrap_or(Value::UNDEFINED)
        };
        vec![global("tools"), global("console")]
    } else {
        Vec::new()
    };
    run_main(vm, main, &args, opts, tools, out, err)
}

/// Loads a dump into a fresh VM and runs it to its end (D2603); exit codes as `run`.
pub fn resume_dump(
    text: &str,
    filename: &str,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut vm = new_vm(opts);
    let host_fn = |name: &str| tools.id_for_call_name(name);
    let lopts = djs_core::vm::dump_load::LoadOptions {
        host_fn: &host_fn,
        ..Default::default()
    };
    let loaded = match vm.load_dump(text, &lopts) {
        Ok(l) => l,
        Err(e) => {
            let _ = writeln!(err, "djs: cannot load {filename}: {}", e.message());
            return 2;
        }
    };
    // The host's handles: the promises stay alive through what waits on them.
    for (_, h) in loaded.handles {
        vm.heap_mut().drop_handle(h);
    }
    drive(vm, opts, tools, out, err)
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
    /// TypeScript legacy decorators, and their metadata.
    pub experimental_decorators: bool,
    pub emit_decorator_metadata: bool,
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
    crate::tools::install(&mut vm);
    vm
}

/// Calls `main` and runs it to completion. Output goes to `out`; an uncaught exception
/// is printed to `err`. Returns the process exit code.
pub fn run_main(
    mut vm: Vm,
    main: Value,
    args: &[Value],
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    // The top level is called with `this` = the global object in both modes (D1140).
    let this = Value::object(vm.global());
    vm.call(main, this, args).expect("the VM is idle");
    drive(vm, opts, tools, out, err)
}

/// Runs the VM to its end, answering open calls at idle points.
fn drive(
    mut vm: Vm,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let fuel = opts.suspend_every.unwrap_or(u64::MAX);
    loop {
        match vm.run(fuel) {
            Status::Idle if !vm.pending_host_calls().is_empty() => {
                // An idle point: answer the oldest call there is an answer for.
                let open = vm.pending_host_calls();
                let answered = open
                    .iter()
                    .find_map(|&id| Some((id, tools.answer(&mut vm, id)?)));
                match answered {
                    Some((id, r)) => vm.resolve_host_call(id, r).expect("an open call"),
                    None => {
                        for id in open {
                            let d = tools.describe(&mut vm, id);
                            let _ = writeln!(err, "djs: no answer for #{id} {d}");
                        }
                        return 1;
                    }
                }
            }
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
            Status::HostOutOfMemory => {
                let _ = writeln!(err, "djs: out of host memory");
                return 1;
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
                vm = Vm::from_bytes(vm.image().to_vec(), o).expect("valid heap image");
            }
            Status::HostCall {
                call_id,
                fn_id: MAKE_TOOL,
            } => tools.make(&mut vm, call_id),
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
