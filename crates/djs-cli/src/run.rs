//! `djs run [--body] [--ts] [--experimental-decorators] [--emit-decorator-metadata]
//! [--gc-stress] [--suspend-every N] [--backend B] [--tools answers.json] <file.js>`: run
//! a script, serving `console.log` and `print` as sync host calls, `later(v)` and
//! `tools.x()` as async host calls (0174, D2602): `later` is answered with its argument,
//! tools from `--tools answers.json`, one call at every idle point. A codemode body's
//! parameters are the CLI's default bindings `tools` and `console`, and the functions
//! `--bind NAME=IDENTITY` adds (D2972).
//!
//! `djs resume [same options] <dump.js>` runs a dump (a plain Script, D2830) in a fresh
//! VM and goes on the same way (D2603).
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
                [--tools answers.json] [--bind NAME=IDENTITY] <file.js>\n\
  --body             run as a codemode async body with parameters (tools, console)\n\
  --bind NAME=ID     with --body: one more parameter NAME, the async host function\n\
                     ID (`--bind myTool=@sdk/our-stuff#myTool`), answered from\n\
                     --tools under ID like a tool (repeatable)\n\
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
  --dump-after K OUT write the run as a dump (`\"djs-dump\";`, a Script, D2830) to OUT\n\
                     at idle point K (from 1) and stop; `djs resume OUT` continues it\n\
  --dump-every-idle  at every idle point, dump the run and continue in a fresh VM that\n\
                     ran the dump\n\
                     (with either, the VM has frozen builtins, which dumps need, D2836)";

const RESUME_USAGE: &str = "usage: djs resume [--explain] [--tools answers.json] [--gc-stress]\n\
                [--suspend-every N] [--backend B] <dump.js>\n\
  runs a dump (`\"djs-dump\";`, a Script) in a fresh VM with frozen builtins, to its\n\
  end, answering open calls as `djs run` does (--dump-after and --dump-every-idle as\n\
  for run)\n\
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
            "--bind" if !resume => match it.next().and_then(|b| b.split_once('=')) {
                Some((name, id)) if !name.is_empty() && !id.is_empty() => {
                    opts.binds.push((name.to_string(), id.to_string()))
                }
                _ => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
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
    opts.resume = resume;
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
    let mut params: Vec<&str> = BODY_PARAMS.to_vec();
    params.extend(opts.binds.iter().map(|(n, _)| n.as_str()));
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
        // `tools` and `console` are the globals; a bound function is made in the tools'
        // table under its identity, so that a dump names it so (D2971).
        let mut args: Vec<Value> = BODY_PARAMS
            .iter()
            .map(|n| vm.global_path(n).unwrap_or(Value::UNDEFINED))
            .collect();
        for (_, id) in &opts.binds {
            if tools.id_for_call_name(id).is_none() {
                tools.id_for(id);
            }
            args.push(tools.host_value(&mut vm, id).unwrap_or(Value::UNDEFINED));
        }
        args
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
    // A dump is a Script: run it, with the hooks served while it loads.
    if let Err(e) = crate::dumps::begin_plain(&mut vm, text, filename) {
        let _ = writeln!(err, "djs: cannot load {filename}: {e}");
        return 2;
    }
    let load = Some(crate::dumps::PlainLoad::default());
    drive_from(vm, None, load, opts, tools, out, err)
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
    let Some(ts) = crate::dumps::plain_format(text) else {
        let _ = writeln!(
            err,
            "djs: cannot load {filename}: not a djs dump (no \"djs-dump\"; header)"
        );
        return 2;
    };
    // The dump runs to its first stop; nothing it prints is shown.
    let loaded = crate::dumps::run_plain(&mut vm, tools, text, filename, &mut |_, _| {});
    let loaded = match loaded {
        Ok(l) => l,
        Err(e) => {
            let _ = writeln!(err, "djs: cannot load {filename}: {e}");
            return 2;
        }
    };
    // Dead activations would be listed too; the handles keep the host's values alive.
    let _ = vm.collect();
    // The open calls (id, the dump's name, state) and the values the dump names for the host.
    let calls: Vec<(u32, String, Option<String>)> = vm
        .pending_host_calls()
        .into_iter()
        .map(|id| {
            let name = vm.host_call(id).map(|c| tools.name(c.fn_id));
            let state = loaded.states.iter().find(|(i, _)| *i == id);
            (id, name.unwrap_or_default(), state.map(|(_, s)| s.clone()))
        })
        .collect();
    let held = crate::dumps::plain_handles(&mut vm);
    let handles: Vec<(Value, String)> = held
        .iter()
        .map(|(n, h)| (vm.heap().get_handle(*h), n.clone()))
        .collect();
    let ts = if ts { " (TypeScript)" } else { "" };
    let all: Vec<activations::Activation> = activations::all(vm.heap())
        .into_iter()
        .map(|a| a.in_file(filename))
        .collect();
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut s = format!(
        "{filename}{ts}: {}, {}",
        plural(calls.len(), "open call", "open calls"),
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
    for (id, name, state) in &calls {
        let state = state.as_deref().map(|j| format!(", state {j}"));
        s.push_str(&format!(
            "#{} {}{}\n",
            id,
            Tools::describe_as(&mut vm, *id, name),
            state.unwrap_or_default()
        ));
        let heap = vm.heap();
        let waiting = activations::call_promise(heap, *id)
            .map(|p| activations::waiting_on(heap, p))
            .unwrap_or_default();
        for a in waiting
            .iter()
            .filter_map(|&g| activations::activation(heap, g))
            .map(|a| a.in_file(filename))
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
    for (_, h) in held {
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
    /// `djs resume`: the run goes on from a dump.
    pub resume: bool,
    /// `--bind NAME=IDENTITY`: more body parameters, bound to host functions (D2972).
    pub binds: Vec<(String, String)>,
}

/// A codemode body's parameters in the CLI: its default bindings (D2972), the `tools`
/// Proxy and `console`; `--bind` adds more.
pub const BODY_PARAMS: &[&str] = &["tools", "console"];

impl RunOptions {
    pub fn vm_options(&self) -> VmOptions {
        VmOptions {
            exec: ExecOptions {
                gc_stress: self.gc_stress,
                native_builtins: !self.bytecode_builtins,
                ..Default::default()
            },
            // Dumps need frozen builtins (D2836), and a resumed run goes on as the dumped
            // one, which had them.
            frozen_builtins: self.resume || self.dumping.after.is_some() || self.dumping.every_idle,
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
    drive_from(vm, None, None, opts, tools, out, err)
}

/// [`drive`] with the handle of the run's promise of a resumed run (else the entry
/// call's result is the run's), and the plain dump `vm` is loading (its hooks are served
/// until the Script stops at something other than a sync host call).
fn drive_from(
    mut vm: Vm,
    mut run: Option<Handle>,
    mut load: Option<crate::dumps::PlainLoad>,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let fuel = opts.suspend_every.unwrap_or(u64::MAX);
    let mut idle_points = 0;
    // A fresh VM loading the dump of the idle point just counted (--dump-every-idle).
    let mut reloaded = false;
    loop {
        let status = vm.run(fuel);
        if load.is_some() {
            match status {
                Status::HostCall { .. } | Status::OutOfFuel => {}
                Status::Threw(e) => {
                    let msg = vm.display(e);
                    let _ = writeln!(err, "djs: the dump did not load: Uncaught {msg}");
                    return 2;
                }
                _ => {
                    let l = load.take().unwrap_or_default();
                    crate::dumps::end_load(&mut vm, tools, &l);
                }
            }
        }
        match status {
            Status::Idle if reloaded => {
                reloaded = false;
                if answer_one(&mut vm, tools, err) {
                    return 1;
                }
            }
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
                    // The dump of this idle point, loaded in a fresh VM, which runs to the
                    // same idle point before the next answer.
                    let fresh = dump_run(&vm, run, tools).and_then(|d| {
                        let mut fresh = new_vm(opts);
                        crate::dumps::begin_plain(&mut fresh, &d.text, "<dump>")?;
                        Ok(fresh)
                    });
                    match fresh {
                        Ok(fresh) => {
                            // Its run's promise is the Script's completion value.
                            (vm, run) = (fresh, None);
                            load = Some(crate::dumps::PlainLoad::default());
                            reloaded = true;
                            continue;
                        }
                        Err(e) => {
                            let _ = writeln!(err, "djs: at idle point {idle_points}: {e}");
                            return 1;
                        }
                    }
                }
                if answer_one(&mut vm, tools, err) {
                    return 1;
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
            Status::HostCall { call_id, fn_id }
                if load.as_mut().is_some_and(|l| {
                    crate::dumps::serve_hook(&mut vm, tools, l, call_id, fn_id)
                }) => {}
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

/// An idle point: answers the oldest call there is an answer for; true (after listing
/// the open calls) when there is none.
fn answer_one(vm: &mut Vm, tools: &mut Tools, err: &mut impl Write) -> bool {
    let open = vm.pending_host_calls();
    let answered = open
        .iter()
        .find_map(|&id| Some((id, tools.answer(vm, id)?)));
    match answered {
        Some((id, r)) => {
            vm.resolve_host_call(id, r).expect("an open call");
            false
        }
        None => {
            for id in open {
                let d = tools.describe(vm, id);
                let _ = writeln!(err, "djs: no answer for #{id} {d}");
            }
            true
        }
    }
}

/// Dumps the run at an idle point (D2830); its result promise (an async body's) is the
/// handle `run`.
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
    crate::dumps::dump(vm, tools, &handles, &|_| None, true)
}

/// The system clock in ms since the epoch.
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
