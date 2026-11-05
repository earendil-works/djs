//! Continuation dumps in the CLI (0174, D2604): writing a waiting session as a dump and
//! loading one, with the host's names for its functions from [`Tools`].
//!
//! A dump (`"djs-dump";`, D2830, D2835) is an ordinary Script: loading it is running it
//! with the host hooks of D2832 in the global object `$djs` (D2890): `$djs.seed(lo, hi)`
//! sets `Math.random`'s state, `$djs.resume(id, name, args, state)` opens the call `id`
//! of the host function `name` again (the CLI answers it as any other call). `$djs` also
//! holds the format's helpers, the dump runtime (D2903). The hooks are served while the
//! Script is evaluated, and `$djs` is deleted after that.

use djs_core::compiler::CompileOptions;
use djs_core::heap::{Handle, ObjectClass};
use djs_core::value::Value;
use djs_core::vm::dump::{DumpOptions, Dumped};
use djs_core::vm::{ErrorKind, Status, Vm};

use crate::tools::Tools;

/// The globals the CLI defines in every VM: never dumped, referenced by name.
pub const HOST_GLOBALS: &[&str] = &["console", "tools", "print", "later"];

/// The global object of the plain dump's host hooks (D2890).
pub const HOOKS: &str = "$djs";
/// `$djs.resume(id, name, args, state)`.
pub const RESUME: u32 = 4;
/// `$djs.seed(lo, hi)`.
pub const SEED: u32 = 5;
/// `$djs.source(name, hash, params)` (D2882): the codemode body the dump continues, the
/// file `name` compiled again.
pub const SOURCE: u32 = 6;
/// `$djs.host(name)` (D2971): a host value by name, the same object the CLI binds (a tool
/// in the `tools` Proxy's table, else the global of the name's path).
pub const HOST: u32 = 7;

/// A plain dump's header: `Some(typescript)` for `"djs-dump";` or
/// `"djs-dump typescript";`.
pub fn plain_format(text: &str) -> Option<bool> {
    let t = text.trim_start();
    if t.starts_with("\"djs-dump\";") {
        Some(false)
    } else if t.starts_with("\"djs-dump typescript\";") {
        Some(true)
    } else {
        None
    }
}

/// A plain dump being loaded: the states its `resume` calls gave, by call id, and what
/// the id counter continues from (D2897).
#[derive(Default)]
pub struct PlainLoad {
    pub states: Vec<(u32, String)>,
    /// `$djs.seed`'s third argument: the dumped VM's next call id.
    next: Option<u32>,
    /// The highest id `resume` reopened.
    last: u32,
    /// The first hook call it could not serve: the load fails (a hook called inside a
    /// continuation throws into its activation, not the Script, D3001).
    pub hook_error: Option<String>,
}

/// Starts loading the plain dump `text` into `vm` (a fresh one): defines `$djs` (the dump
/// runtime and the hooks) and calls the Script. The caller runs the VM, serving the hooks with [`serve_hook`], and calls
/// [`end_load`] at the first stop that is not a sync host call.
pub fn begin_plain(vm: &mut Vm, text: &str, filename: &str) -> Result<(), String> {
    let ts = plain_format(text).ok_or("not a djs dump (no \"djs-dump\"; header)")?;
    let oom = |_| String::from("out of memory");
    // The dump runtime's helpers, with the hooks added (D2903).
    let hooks = vm
        .dump_runtime_for(text)?
        .as_object()
        .ok_or("the dump runtime is not an object")?;
    let resume = vm.new_host_function(RESUME, "resume").map_err(oom)?;
    vm.define(hooks, "resume", resume).map_err(oom)?;
    let seed = vm.new_host_function(SEED, "seed").map_err(oom)?;
    vm.define(hooks, "seed", seed).map_err(oom)?;
    let source = vm.new_host_function(SOURCE, "source").map_err(oom)?;
    vm.define(hooks, "source", source).map_err(oom)?;
    let host = vm.new_host_function(HOST, "host").map_err(oom)?;
    vm.define(hooks, "host", host).map_err(oom)?;
    vm.define_global(HOOKS, Value::object(hooks)).map_err(oom)?;
    let copts = CompileOptions {
        typescript: ts,
        ..CompileOptions::script(filename)
    };
    let f = vm.compile(text.as_bytes(), &copts).map_err(|e| {
        format!(
            "SyntaxError: {} at {filename}:{}:{}",
            e.message, e.line, e.col
        )
    })?;
    let this = Value::object(vm.global());
    vm.call(f, this, &[])
        .map_err(|_| String::from("the VM is busy"))
}

/// Serves `$djs.resume` and `$djs.seed`, the parked sync call `call_id` of `fn_id`;
/// false for another function.
pub fn serve_hook(
    vm: &mut Vm,
    tools: &mut Tools,
    load: &mut PlainLoad,
    call_id: u32,
    fn_id: u32,
) -> bool {
    if fn_id != RESUME && fn_id != SEED && fn_id != SOURCE && fn_id != HOST {
        return false;
    }
    let args = vm.host_call(call_id).expect("parked call").args;
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::UNDEFINED);
    if fn_id == HOST {
        let name = arg(0).as_string().map(|s| vm.heap().str_to_string(s));
        let r = match name {
            Some(n) => tools
                .host_value(vm, &n)
                .ok_or_else(|| format!("host: unknown host value {n}")),
            None => Err(String::from("host: not a name")),
        };
        let r = r.map_err(|msg| {
            load.hook_error.get_or_insert_with(|| msg.clone());
            vm.new_error(ErrorKind::Error, &msg).expect("oom")
        });
        let _ = vm.resolve_host_call(call_id, r);
        return true;
    }
    if fn_id == SOURCE {
        let text = |v: Value| v.as_string().map(|s| vm.heap().str_to_string(s));
        let r = match (text(arg(0)), text(arg(1))) {
            (Some(name), Some(hash)) => {
                let params = text(arg(2)).unwrap_or_default();
                source_function(vm, &name, &hash, &params)
            }
            _ => Err(String::from("source: not a name and a hash")),
        };
        let r = r.map_err(|msg| {
            load.hook_error.get_or_insert_with(|| msg.clone());
            vm.new_error(ErrorKind::Error, &msg).expect("oom")
        });
        let _ = vm.resolve_host_call(call_id, r);
        return true;
    }
    // `$djs.seed(lo, hi[, next])`, or `$djs.seed(next)` (D2837).
    let r = if fn_id == SEED {
        match (arg(0).as_int(), arg(1).as_int(), arg(1).is_undefined()) {
            (Some(n), _, true) => {
                load.next = Some(n as u32);
                Ok(())
            }
            (Some(lo), Some(hi), false) => {
                vm.restore_math_random(u64::from(lo as u32) | u64::from(hi as u32) << 32);
                load.next = arg(2).as_int().map(|n| n as u32);
                Ok(())
            }
            _ => Err(String::from("seed: not two int32 halves")),
        }
    } else {
        let name = arg(1).as_string().map(|s| vm.heap().str_to_string(s));
        // The state first: its `made` lists the tools in the dumped run's order (D2897).
        let state = (!arg(3).is_undefined()).then(|| crate::tools::to_json(vm, arg(3)));
        if let Some(s) = &state {
            tools.restore_state(s);
        }
        match name.as_deref().map(|n| (n, tools.id_for_call_name(n))) {
            Some((_, Some(f))) => {
                let id = arg(0).as_int();
                vm.open_host_call(call_id, f)
                    .map_err(String::from)
                    .map(|()| {
                        let id = id.unwrap_or(0) as u32;
                        load.last = load.last.max(id);
                        if let Some(s) = state {
                            load.states.push((id, s));
                        }
                    })
            }
            Some((n, None)) => Err(format!("resume: unknown host function {n}")),
            None => Err(String::from("resume: not a function name")),
        }
    };
    match r {
        Ok(()) => {
            if fn_id == SEED {
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED))
                    .expect("parked on this call");
            }
        }
        Err(msg) => {
            // (Not parked any more only when out of memory: the load fails anyway.)
            load.hook_error.get_or_insert_with(|| msg.clone());
            let e = vm.new_error(ErrorKind::Error, &msg).expect("oom");
            let _ = vm.resolve_host_call(call_id, Err(e));
        }
    }
    true
}

/// `$djs.source(name, hash, params)`: the file `name` (the path the run was given), as
/// the codemode body it ran as (TypeScript and decorator options tried until one is the
/// script the dump refers to).
fn source_function(vm: &mut Vm, name: &str, hash: &str, params: &str) -> Result<Value, String> {
    let src = std::fs::read(name).map_err(|e| {
        format!("source: the dump continues the script {name}, which cannot be read ({e})")
    })?;
    let params: Vec<&str> = params.split(',').filter(|p| !p.is_empty()).collect();
    let mut why = String::new();
    for (typescript, legacy, meta) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (true, true, true),
    ] {
        let opts = CompileOptions {
            typescript,
            experimental_decorators: legacy,
            emit_decorator_metadata: meta,
            debugger_awaits: false,
            ..CompileOptions::async_body(name, &params)
        };
        match vm.source_function(&src, &opts, hash) {
            Ok(f) => return Ok(f),
            Err(e) => why = e,
        }
    }
    Err(why)
}

/// The plain dump's Script was evaluated: its hooks and holder go away (D2916), and the id counter goes on
/// from the dumped VM's (the hooks took ids of their own, D2897).
pub fn end_load(vm: &mut Vm, tools: &Tools, load: &PlainLoad) {
    tools.premake(vm);
    // (Without `next`: one past the highest reopened call, D2837.)
    vm.set_next_call_id(load.next.unwrap_or(0).max(load.last + 1));
    let _ = vm.end_dump_load();
}

/// Loads the plain dump `text` into `vm` (a fresh one) and runs it until it stops at
/// something other than a sync host call (idle, done): serves the hooks, `make` and the
/// printing function (`log`, with the call's arguments); other sync calls throw. Its
/// run's promise is the VM's outcome (the Script's completion value). Err: the dump does
/// not compile, or its Script threw.
pub fn run_plain(
    vm: &mut Vm,
    tools: &mut Tools,
    text: &str,
    filename: &str,
    log: &mut dyn FnMut(&Vm, &[Value]),
) -> Result<PlainLoad, String> {
    begin_plain(vm, text, filename)?;
    let mut load = PlainLoad::default();
    loop {
        match vm.run(u64::MAX) {
            Status::HostCall { call_id, fn_id } => {
                if serve_hook(vm, tools, &mut load, call_id, fn_id) {
                    continue;
                }
                if fn_id == crate::tools::MAKE_TOOL {
                    tools.make(vm, call_id);
                    continue;
                }
                let r = if fn_id == crate::run::CONSOLE_LOG {
                    let args = vm.host_call(call_id).expect("parked call").args;
                    log(vm, &args);
                    Ok(Value::UNDEFINED)
                } else {
                    Err(Value::UNDEFINED)
                };
                vm.resolve_host_call(call_id, r)
                    .expect("parked on this call");
            }
            Status::Threw(e) => return Err(format!("the dump threw: {}", vm.display(e))),
            Status::HostOutOfMemory => return Err(String::from("out of host memory")),
            _ => break,
        }
    }
    end_load(vm, tools, &load);
    if let Some(e) = &load.hook_error {
        return Err(e.clone());
    }
    Ok(load)
}

/// The values a loaded plain dump names for the host (D2895), from its Script's
/// completion value: a promise is the run (`run`); a null-prototype object holds several
/// by name (the REPL's waiting inputs). Handles the caller drops.
pub fn plain_handles(vm: &mut Vm) -> Vec<(String, Handle)> {
    let Some(Ok(v)) = vm.outcome() else {
        return Vec::new();
    };
    let heap = vm.heap_mut();
    if vm_promise(heap, v) {
        return heap
            .new_handle(v)
            .map(|h| vec![(String::from("run"), h)])
            .unwrap_or_default();
    }
    let Some(o) = v.as_object().filter(|&o| {
        heap.object_class(o) == ObjectClass::Ordinary && heap.get_prototype_of(o).is_none()
    }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for k in heap.own_keys(o) {
        let Some(v) = heap.get_own_property(o, k).and_then(|p| p.value()) else {
            continue;
        };
        let name = heap.atom_name(k);
        if let Ok(h) = heap.new_handle(v) {
            out.push((name, h));
        }
    }
    out
}

fn vm_promise(heap: &djs_core::heap::Heap, v: Value) -> bool {
    v.as_object()
        .is_some_and(|o| heap.object_class(o) == ObjectClass::Promise)
}

/// The text is a dump: it starts with the header `"djs-dump";` (D2835).
pub fn is_dump(text: &[u8]) -> bool {
    std::str::from_utf8(text).is_ok_and(|t| plain_format(t).is_some())
}

/// Dumps the idle VM with the host's `handles`; the refusal's message otherwise (a VM
/// without frozen builtins cannot dump, D2836). Each
/// open call gets the CLI's state ([`Tools::call_state`], D2894), with its `origin`.
pub fn dump(
    vm: &Vm,
    tools: &Tools,
    handles: &[(&str, Value)],
    origin: &dyn Fn(u32) -> Option<String>,
    source_refs: bool,
) -> Result<Dumped, String> {
    dump_names(vm, tools, handles, None, origin, source_refs)
}

/// [`dump`] of the named globals only (all of them with `None`).
pub fn dump_names(
    vm: &Vm,
    tools: &Tools,
    handles: &[(&str, Value)],
    names: Option<&[&str]>,
    origin: &dyn Fn(u32) -> Option<String>,
    source_refs: bool,
) -> Result<Dumped, String> {
    let name = |id: u32| tools.call_name(id);
    let states: Vec<(u32, String)> = vm
        .pending_host_calls()
        .into_iter()
        .filter_map(|id| Some((id, tools.call_state(origin(id).as_deref())?)))
        .collect();
    let states: Vec<(u32, &str)> = states.iter().map(|(id, s)| (*id, s.as_str())).collect();
    let opts = DumpOptions {
        base: djs_prelude::BASE,
        host: HOST_GLOBALS,
        host_roots: &[],
        host_fn_name: &name,
        handles,
        call_states: &states,
        // `djs run`: the body by reference, `djs resume` reads it from its path again
        // (D2882); the REPL writes its inputs' text (a dump that stands alone).
        source_refs,
        base_names: Some(djs_core::vm::dump::BaseNames {
            builtins: djs_prelude::BASE_NAMES,
            globals: djs_prelude::BASE_GLOBALS,
        }),
        check: true,
    };
    vm.dump_report(&opts, names).map_err(|e| e.message())
}

/// `(1 activation, 2 pending calls)`, or nothing when nothing is in flight.
pub fn in_flight(d: &Dumped) -> String {
    let (a, c) = (d.activations.len(), d.open_calls);
    if a == 0 && c == 0 {
        return String::new();
    }
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(" ({a} activation{}, {c} pending call{})", s(a), s(c))
}

/// Takes a handle by name out of a load's handles.
pub fn take_handle(handles: &mut Vec<(String, Handle)>, name: &str) -> Option<Handle> {
    let i = handles.iter().position(|(n, _)| n == name)?;
    Some(handles.remove(i).1)
}
