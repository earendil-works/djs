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
use djs_core::heap::Handle;
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
  --dump-after K OUT write the run as a format 2 dump to OUT at idle point K (from 1)\n\
                     and stop; `djs resume OUT` continues it\n\
  --dump-every-idle  at every idle point, dump the run and continue in a fresh VM that\n\
                     loaded the dump";

const RESUME_USAGE: &str = "usage: djs resume [--explain] [--tools answers.json] [--gc-stress]\n\
                [--suspend-every N] [--backend B] <dump.js>\n\
  loads a dump (format 2, `\"djs-dump 2\";`, or version 1) into a fresh VM and runs it\n\
  to its end, answering open calls as `djs run` does\n\
  --explain          only load it, print its open calls and suspended activations (where\n\
                     each stopped in the dump, with the source line), and exit";

/// Dumps at idle points (D2604). An idle point is a stop with only host calls open,
/// before the run answers one, so each answer starts a new one.
#[derive(Clone, Debug, Default)]
pub struct Dumping {
    /// `--dump-after K OUT`.
    pub after: Option<(u64, String)>,
    /// `--dump-every-idle`.
    pub every_idle: bool,
}

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
    let mut explain = false;
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
            "--explain" if resume => explain = true,
            "--dump-after" => match (it.next().and_then(|k| k.parse().ok()), it.next()) {
                (Some(k), Some(out)) if k > 0 => opts.dumping.after = Some((k, out.clone())),
                _ => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--dump-every-idle" => opts.dumping.every_idle = true,
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
        if explain {
            return explain_dump(
                &text,
                &path,
                &opts,
                &mut tools,
                &mut stdout.lock(),
                &mut stderr.lock(),
            );
        }
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
    let mut loaded = match crate::dumps::load(&mut vm, tools, text) {
        Ok(l) => l,
        Err(e) => {
            let _ = writeln!(err, "djs: cannot load {filename}: {e}");
            return 2;
        }
    };
    // The run's promise stays the handle `run` for later dumps; the others are dropped
    // (the promises stay alive through what waits on them).
    let run = crate::dumps::take_handle(&mut loaded.handles, "run");
    for (_, h) in loaded.handles {
        vm.heap_mut().drop_handle(h);
    }
    drive_from(vm, run, opts, tools, out, err)
}

/// `djs resume --explain` (D2641): what a dump holds in flight, for a reader, since the
/// dump itself has no comments. Loads it into a fresh VM (a dump is code: this runs it,
/// as `resume` does), then lists the open calls with the activations waiting on them and
/// the other suspended activations, each with where it stopped in the dump
/// (`dump.js:LINE:COL`), the line and a caret, and its bindings. Answers nothing.
pub fn explain_dump(
    text: &str,
    filename: &str,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    use crate::repl::activations;
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
    // Dead activations would be listed too; the handles keep the host's values alive.
    let _ = vm.collect();
    let format = djs_core::vm::dump_load::dump_format(text).unwrap_or("?");
    let ts = if text.starts_with("\"djs-dump 2 typescript\";") {
        ", TypeScript"
    } else {
        ""
    };
    let handles: Vec<(Value, String)> = loaded
        .handles
        .iter()
        .map(|(n, h)| (vm.heap().get_handle(*h), n.clone()))
        .collect();
    let all = activations::all(vm.heap());
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut s = format!(
        "{filename}: format {format}{ts}; {}, {}",
        plural(loaded.calls.len(), "open call", "open calls"),
        plural(all.len(), "suspended activation", "suspended activations"),
    );
    if !handles.is_empty() {
        let names: Vec<&str> = handles.iter().map(|(_, n)| n.as_str()).collect();
        s.push_str(&format!("; handles: {}", names.join(", ")));
    }
    s.push('\n');
    let label = |a: &activations::Activation| {
        handles
            .iter()
            .find(|(p, _)| *p == a.promise)
            .map(|(_, n)| format!(" ({n})"))
            .unwrap_or_default()
    };
    let mut shown: Vec<String> = Vec::new();
    for c in &loaded.calls {
        s.push_str(&format!("#{} {}\n", c.id, tools.describe(&mut vm, c.id)));
        let heap = vm.heap();
        let waiting = activations::call_promise(heap, c.id)
            .map(|p| activations::waiting_on(heap, p))
            .unwrap_or_default();
        for a in waiting
            .iter()
            .filter_map(|&g| activations::activation(heap, g))
        {
            let d = a.describe(&label(&a));
            s.push_str(&d);
            shown.push(d);
        }
    }
    let rest: Vec<String> = all
        .iter()
        .map(|a| a.describe(&label(a)))
        .filter(|d| !shown.contains(d))
        .collect();
    if !rest.is_empty() {
        s.push_str("not waiting on a call:\n");
        rest.iter().for_each(|d| s.push_str(d));
    }
    for (_, h) in loaded.handles {
        vm.heap_mut().drop_handle(h);
    }
    let _ = out.write_all(s.as_bytes());
    0
}

/// Host function ids registered by `djs run`.
pub const CONSOLE_LOG: u32 = 1;

/// Options of `djs run`.
#[derive(Clone, Debug, Default)]
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
    /// Dumps at idle points.
    pub dumping: Dumping,
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
    vm: Vm,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    drive_from(vm, None, opts, tools, out, err)
}

/// [`drive`] with the handle of the run's promise of a resumed run (else the entry
/// call's result is the run's).
fn drive_from(
    mut vm: Vm,
    mut run: Option<Handle>,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let fuel = opts.suspend_every.unwrap_or(u64::MAX);
    let mut idle_points = 0;
    loop {
        match vm.run(fuel) {
            Status::Idle if !vm.pending_host_calls().is_empty() => {
                idle_points += 1;
                if let Some((k, path)) = &opts.dumping.after
                    && *k == idle_points
                {
                    let _ = out.flush();
                    return match dump_run(&vm, run, tools) {
                        Ok(d) => match std::fs::write(path, &d.text) {
                            Ok(()) => 0,
                            Err(e) => {
                                let _ = writeln!(err, "djs: cannot write {path}: {e}");
                                2
                            }
                        },
                        Err(e) => {
                            let _ = writeln!(err, "djs: at idle point {k}: {e}");
                            1
                        }
                    };
                }
                if opts.dumping.every_idle {
                    let reloaded = dump_run(&vm, run, tools).and_then(|d| {
                        let mut fresh = new_vm(opts);
                        let mut loaded = crate::dumps::load(&mut fresh, tools, &d.text)?;
                        let run = crate::dumps::take_handle(&mut loaded.handles, "run");
                        for (_, h) in loaded.handles {
                            fresh.heap_mut().drop_handle(h);
                        }
                        Ok((fresh, run))
                    });
                    match reloaded {
                        Ok((fresh, r)) => (vm, run) = (fresh, r),
                        Err(e) => {
                            let _ = writeln!(err, "djs: at idle point {idle_points}: {e}");
                            return 1;
                        }
                    }
                }
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
                if let Some((k, _)) = &opts.dumping.after {
                    let _ = writeln!(
                        err,
                        "djs: --dump-after {k}: the run ended after {idle_points} idle points"
                    );
                }
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

/// Dumps the run at an idle point; its result promise (an async body's) is the handle
/// `run`.
fn dump_run(
    vm: &Vm,
    run: Option<Handle>,
    tools: &Tools,
) -> Result<djs_core::vm::dump::Dumped, String> {
    let run = match (run, vm.outcome()) {
        (Some(h), _) => Some(vm.heap().get_handle(h)),
        (None, Some(Ok(v))) if vm.promise_state(v).is_some() => Some(v),
        _ => None,
    };
    let handles: Vec<(&str, Value)> = run.map(|v| ("run", v)).into_iter().collect();
    crate::dumps::dump(vm, tools, &handles)
}

/// The system clock in ms since the epoch.
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
