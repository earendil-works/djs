//! `djs run [--body] [--ts] [--gc-stress] [--suspend-every N] [--tools answers.json]
//! <file.js>`: run
//! a script, serving `console.log` and `print` as sync host calls, `later(v)` and
//! `tools.x()` as async host calls (0174, D2602): `later` is answered with its argument,
//! tools from `--tools answers.json`, one call at every idle point. A codemode body's
//! parameters are the CLI's default bindings `tools` and `console`, and the functions
//! `--bind NAME=IDENTITY` adds (D2972).
//!
//! `djs resume [same options] <file>` loads a snapshot (D3720) into a fresh VM and goes on
//! the same way.
//!
//! Exit codes: 0 ok, 1 uncaught exception (printed as `Uncaught Name: message`) or a tool
//! call the answers file has no answer for, 2 usage or compile error.

use std::io::Write;

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

use crate::tools::{MAKE_TOOL, Tools};

const USAGE: &str = "usage: djs run [--body] [--ts] [--gc-stress] [--suspend-every N]\n\
                [--tools answers.json] [--bind NAME=IDENTITY] <file.js>\n\
  --body             run as a codemode async body with parameters (tools, console)\n\
  --bind NAME=ID     with --body: one more parameter NAME, the async host function\n\
                     ID (`--bind myTool=@sdk/our-stuff#myTool`), answered from\n\
                     --tools under ID like a tool (repeatable)\n\
  --ts               parse as TypeScript (the default for .ts files)\n\
  --gc-stress        collect garbage at every safepoint\n\
  --suspend-every N  run N fuel at a time; at every stop, continue in a fresh VM\n\
                     restored from the heap image\n\
  --tools FILE       answer `tools.x()` calls from FILE, one at a time whenever the\n\
                     run is idle (docs/cli.md)\n\
  --snapshot-after K OUT\n\
                     write the run as a snapshot (D3720) to OUT at idle point K (from\n\
                     1) and stop; `djs resume OUT` continues it\n\
  --snapshot-every-idle\n\
                     at every idle point, snapshot the run and continue in a fresh VM\n\
                     that loaded the snapshot\n\
  --debugger OUT     `debugger;` in an async function (or a --body) pauses: at the\n\
                     first pause, write the run as a snapshot to OUT and stop; `djs\n\
                     resume OUT` continues it";

const RESUME_USAGE: &str = "usage: djs resume [--tools answers.json] [--gc-stress]\n\
                [--suspend-every N] [--debugger OUT] <file.snapshot>\n\
  loads a snapshot (D3720) into a fresh VM and goes on to the end, answering open calls\n\
  as `djs run` does (--snapshot-after, --snapshot-every-idle and --debugger as for run);\n\
  a `debugger;` pause it holds continues at once (`djs describe` shows what it holds)";

/// Snapshots (D3720) at idle points. An idle point is a stop with only host calls open,
/// before the run answers one, so each answer starts a new one.
#[derive(Clone, Debug, Default)]
pub struct Snapshotting {
    /// `--snapshot-after K OUT`.
    pub after: Option<(u64, String)>,
    /// `--snapshot-every-idle`.
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
            "--gc-stress" => opts.gc_stress = true,
            "--snapshot-after" => match (it.next().and_then(|k| k.parse().ok()), it.next()) {
                (Some(k), Some(out)) if k > 0 => opts.snapshots.after = Some((k, out.clone())),
                _ => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--snapshot-every-idle" => opts.snapshots.every_idle = true,
            "--debugger" => match it.next() {
                Some(out) => opts.debugger = Some(out.clone()),
                None => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
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
        return resume_snapshot(
            &src,
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
        debugger_awaits: opts.debugger.is_some(),
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
                // Only with an elastic stack (D3280), which the CLI does not set.
                CompileErrorKind::NeedStack => {
                    let _ = writeln!(err, "djs: {}", e.message);
                }
            }
            return 2;
        }
    };
    let args: Vec<Value> = if opts.body {
        // `tools` and `console` are the globals; a bound function is made in the tools'
        // table under its identity, so that a snapshot names it so (D2971).
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

/// Loads a snapshot (D3720) into a fresh VM of its builtins' mode and runs it to its end;
/// exit codes as `run`.
pub fn resume_snapshot(
    bytes: &[u8],
    filename: &str,
    opts: &RunOptions,
    tools: &mut Tools,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let frozen = match djs_core::vm::snapshot::needs(bytes) {
        Ok(n) => n.frozen_builtins,
        Err(e) => {
            let _ = writeln!(err, "djs: cannot load {filename}: {}", e.0.join("; "));
            return 2;
        }
    };
    let mut vm = new_vm_with(opts, frozen);
    match crate::snapshots::load(&mut vm, tools, bytes) {
        Ok(_) => drive(vm, opts, tools, out, err),
        Err(e) => {
            let _ = writeln!(err, "djs: cannot load {filename}: {e}");
            2
        }
    }
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
    /// Snapshots at idle points.
    pub snapshots: Snapshotting,
    /// `djs resume`: the run goes on from a snapshot.
    pub resume: bool,
    /// `--bind NAME=IDENTITY`: more body parameters, bound to host functions (D2972).
    pub binds: Vec<(String, String)>,
    /// `--debugger OUT`: `debugger;` pauses (D3050); the first pause snapshots the run to
    /// OUT and stops it (D3060).
    pub debugger: Option<String>,
}

/// A codemode body's parameters in the CLI: its default bindings (D2972), the `tools`
/// Proxy and `console`; `--bind` adds more.
pub const BODY_PARAMS: &[&str] = &["tools", "console"];

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
    new_vm_with(opts, opts.vm_options().frozen_builtins)
}

/// [`new_vm`] with frozen builtins or not (a snapshot's mode).
pub fn new_vm_with(opts: &RunOptions, frozen: bool) -> Vm {
    let vo = VmOptions {
        frozen_builtins: frozen,
        ..opts.vm_options()
    };
    let mut vm = djs_prelude::new_vm(vo).expect("out of memory creating the VM");
    // The VM reads no clock itself (D1383): give it the wall clock, in UTC.
    vm.set_now(now_ms());
    let console = vm.new_object().expect("oom");
    let log = vm.new_host_function(CONSOLE_LOG, "log").expect("oom");
    vm.define(console, "log", log).expect("oom");
    vm.define_global("console", Value::object(console))
        .expect("oom");
    crate::tools::install(&mut vm);
    // A snapshot may hold a pause, or continue a body compiled with `debugger_awaits`, so
    // a resumed run has `debugger` too: it answers such a pause at once (D3060).
    if opts.debugger.is_some() || opts.resume {
        crate::tools::install_debugger(&mut vm);
    }
    vm
}

/// Calls `main` and runs it to completion. Output goes to `out`; an uncaught exception
/// is printed to `err`. Returns the process exit code.
#[allow(clippy::too_many_arguments)]
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
    let mut idle_points = 0;
    // A fresh VM loaded from the snapshot of the idle point just counted
    // (--snapshot-every-idle).
    let mut reloaded = false;
    loop {
        let status = vm.run(fuel);
        match status {
            Status::Idle if reloaded => {
                reloaded = false;
                if answer_one(&mut vm, tools, err) {
                    return 1;
                }
            }
            Status::Idle if !vm.pending_host_calls().is_empty() => {
                idle_points += 1;
                // The calls made since the last idle point: a pause among them stops the
                // run with `--debugger` (the pauses a loaded snapshot holds are not new).
                let new = vm.new_host_calls();
                if let Some((k, path)) = &opts.snapshots.after
                    && *k == idle_points
                {
                    let _ = out.flush();
                    return match snapshot_run(&mut vm, tools) {
                        Ok(b) => match std::fs::write(path, &b) {
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
                if let Some(path) = &opts.debugger
                    && let Some(&id) = new.iter().find(|&&id| tools.is_pause(&vm, id))
                {
                    let _ = out.flush();
                    let at = pause_position(&vm, id);
                    return match snapshot_run(&mut vm, tools) {
                        Ok(b) => match std::fs::write(path, &b) {
                            Ok(()) => {
                                let _ = writeln!(err, "djs: paused at {at}, wrote {path}");
                                0
                            }
                            Err(e) => {
                                let _ = writeln!(err, "djs: cannot write {path}: {e}");
                                2
                            }
                        },
                        Err(e) => {
                            let _ = writeln!(err, "djs: paused at {at}: {e}");
                            1
                        }
                    };
                }
                if opts.snapshots.every_idle {
                    // The snapshot of this idle point, loaded in a fresh VM, which is at
                    // the same idle point before the next answer.
                    let fresh = snapshot_run(&mut vm, tools).and_then(|b| {
                        let mut fresh = new_vm_with(opts, vm.frozen_builtins());
                        crate::snapshots::load(&mut fresh, tools, &b)?;
                        Ok(fresh)
                    });
                    match fresh {
                        Ok(f) => {
                            vm = f;
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
                if let Some((k, _)) = &opts.snapshots.after {
                    let _ = writeln!(
                        err,
                        "djs: --snapshot-after {k}: the run ended after {idle_points} idle points"
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

/// An idle point: answers the oldest call there is an answer for; true (after listing
/// the open calls) when there is none.
fn answer_one(vm: &mut Vm, tools: &mut Tools, err: &mut impl Write) -> bool {
    let open = vm.pending_host_calls();
    // A pause that does not stop the run (a resumed snapshot's) continues at once.
    let answered = open.iter().find_map(|&id| {
        let r = tools.answer(vm, id);
        let r = r.or_else(|| tools.is_pause(vm, id).then_some(Ok(Value::UNDEFINED)));
        Some((id, r?))
    });
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

/// Where the activation waiting on the pause `id` stopped (`file.js:3:5`), else the call.
fn pause_position(vm: &Vm, id: u32) -> String {
    use crate::repl::activations;
    let heap = vm.heap();
    activations::call_promise(heap, id)
        .and_then(|p| activations::waiting_on(heap, p).first().copied())
        .and_then(|g| activations::activation(heap, g))
        .map(|a| a.at)
        .unwrap_or_else(|| format!("call #{id}"))
}

/// Snapshots the run at an idle point (D3720).
fn snapshot_run(vm: &mut Vm, tools: &Tools) -> Result<Vec<u8>, String> {
    crate::snapshots::snapshot(vm, tools, &[], &|_| None)
}

/// The system clock in ms since the epoch.
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
