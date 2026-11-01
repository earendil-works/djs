//! The `djs` engine: djs in-process, a fresh `Vm` per case.
//!
//! Until djs can run the shared prelude (tests/corpus/prelude.js needs JSON, Promise,
//! Map, Object.freeze, ...), the codemode globals are host functions implemented here
//! with the prelude's formatting rules (D1160): `console.*`, `text`, `exit`, `store`,
//! `load` and the fake `tools` (`echo`, `fail`), plus `ALL_TOOLS`. The case is compiled as
//! the body of an async function with the parameters `tools` and `console` (D008), called
//! with `this` = the global object, and run in fuel slices.
//!
//! Stress variants: `suspend_every` runs N fuel at a time and continues each slice in a
//! fresh VM restored from the heap image; `gc_stress` collects at every safepoint.
//! Their transcripts must equal the plain run's (checked by the runner).

use std::collections::HashMap;
use std::time::Instant;

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::heap::DEFAULT_LIMIT;
use djs_core::interp::error_summary;
use djs_core::json::{ParseOptions, StringifyError, parse_utf8, stringify_fast};
use djs_core::num;
use djs_core::value::{Kind, Value};
use djs_core::vm::{ErrorKind, ExecOptions, Status, Vm, VmOptions};

use crate::engine::{Done, Engine, Job, RunResult};

/// Fuel when the case has no `// difftest: fuel` directive: generous, but it bounds an
/// in-process infinite loop, which a wall-clock timeout alone cannot interrupt.
pub const DEFAULT_FUEL: u64 = 20_000_000_000;
/// Fuel per `run` in the plain mode; the wall-clock timeout is checked between slices.
const SLICE: u64 = 10_000_000;
/// Fuel per `run` under `--gc-stress`.
const STRESS_SLICE: u64 = 20_000;

const STALL: &str = "Error: The script is waiting on a promise that can never settle: no tool call is pending, and timers do not exist here.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Djs {
    /// Suspend every N fuel and continue in a VM restored from the heap image.
    pub suspend_every: Option<u64>,
    /// Collect garbage at every safepoint.
    pub gc_stress: bool,
}

impl Djs {
    pub fn label(&self) -> String {
        match (self.suspend_every, self.gc_stress) {
            (None, false) => "djs".into(),
            (Some(n), false) => format!("djs --suspend-every {n}"),
            (None, true) => "djs --gc-stress".into(),
            (Some(n), true) => format!("djs --suspend-every {n} --gc-stress"),
        }
    }
}

#[derive(Clone, Copy)]
enum Host {
    Log,
    Text,
    Exit,
    Store,
    Load,
    Echo,
    Fail,
}

const HOSTS: [Host; 7] = [
    Host::Log,
    Host::Text,
    Host::Exit,
    Host::Store,
    Host::Load,
    Host::Echo,
    Host::Fail,
];

impl Engine for Djs {
    fn name(&self) -> &str {
        "djs"
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        match self.run_case(job) {
            Ok(r) => r,
            Err(msg) => RunResult {
                output: Vec::new(),
                done: Done::Crash(msg),
            },
        }
    }
}

struct Session {
    vm: Vm,
    output: Vec<String>,
    store: HashMap<String, String>,
}

impl Djs {
    fn run_case(&self, job: &Job<'_>) -> Result<RunResult, String> {
        let opts = VmOptions {
            memory_limit: job
                .memory_limit
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT),
            exec: ExecOptions {
                gc_stress: self.gc_stress,
                ..ExecOptions::default()
            },
            ..VmOptions::default()
        };
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        // The wall clock, in UTC (D1383); the qjs engine runs with TZ=UTC to match.
        vm.set_now(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0),
        );
        let mut s = Session {
            vm,
            output: Vec::new(),
            store: HashMap::new(),
        };
        let (tools, console) = s
            .install_globals()
            .map_err(|e| format!("installing globals: {e:?}"))?;

        let params = ["tools", "console"];
        let main = match s.vm.compile(
            job.source.as_bytes(),
            &CompileOptions::async_body(job.name, &params),
        ) {
            Ok(f) => f,
            Err(e) if e.kind == CompileErrorKind::Syntax => {
                return Ok(s.finish(Done::Throw(format!("SyntaxError: {}", e.message))));
            }
            Err(e) => return Err(format!("compile: {}", e.message)),
        };
        let this = Value::object(s.vm.global());
        s.vm.call(main, this, &[tools, console])
            .map_err(|e| format!("Vm::call: {e:?}"))?;

        let budget = job.fuel.unwrap_or(DEFAULT_FUEL);
        // Under GC stress a slice can take a long time (a collection at every safepoint),
        // so the wall-clock limit is checked more often.
        let plain_slice = if self.gc_stress { STRESS_SLICE } else { SLICE };
        let slice = self.suspend_every.unwrap_or(plain_slice).min(budget.max(1));
        let start = Instant::now();
        let mut used = 0u64;
        loop {
            match s.vm.run(slice) {
                Status::Done(v) => {
                    // The async body's promise (D1370: Done comes once no job is left).
                    let done = match s.vm.promise_state(v) {
                        None => s.result(v),
                        Some(Some(Ok(v))) => s.result(v),
                        Some(Some(Err(e))) => Done::Throw(s.describe(e)),
                        Some(None) => Done::Throw(STALL.into()),
                    };
                    return Ok(s.finish(done));
                }
                Status::Threw(e) => {
                    let done = Done::Throw(s.describe(e));
                    return Ok(s.finish(done));
                }
                Status::Idle => {
                    // Settle the pending tool calls (the order they were made in).
                    let ids = s.vm.new_host_calls();
                    if ids.is_empty() {
                        return Ok(s.finish(Done::Throw(STALL.into())));
                    }
                    for id in ids {
                        let info = s.vm.host_call(id).ok_or("host call vanished")?;
                        let host = *HOSTS
                            .get(info.fn_id as usize)
                            .ok_or_else(|| format!("unknown host function {}", info.fn_id))?;
                        let result = s
                            .host_call(host, &info.args)
                            .map_err(|e| format!("host call: {e:?}"))?;
                        s.vm.resolve_host_call(id, result)
                            .map_err(|e| format!("resolve_host_call: {e:?}"))?;
                    }
                }
                Status::OutOfFuel => {
                    used = used.saturating_add(slice);
                    if used >= budget || start.elapsed() >= job.timeout {
                        return Ok(s.finish(Done::Timeout));
                    }
                    if self.suspend_every.is_some() {
                        let o = s.vm.options();
                        s.vm = Vm::from_bytes(s.vm.bytes().to_vec(), o)
                            .map_err(|e| format!("Vm::from_bytes: {e:?}"))?;
                    }
                }
                Status::HostCall { call_id, fn_id } => {
                    let info = s.vm.host_call(call_id).ok_or("host call vanished")?;
                    let host = *HOSTS
                        .get(fn_id as usize)
                        .ok_or_else(|| format!("unknown host function {fn_id}"))?;
                    if let Host::Exit = host {
                        // exit(): the script ends successfully right away.
                        return Ok(s.finish(Done::Completed));
                    }
                    let result = s
                        .host_call(host, &info.args)
                        .map_err(|e| format!("host call: {e:?}"))?;
                    s.vm.resolve_host_call(call_id, result)
                        .map_err(|e| format!("resolve_host_call: {e:?}"))?;
                }
            }
        }
    }
}

type HeapResult<T> = Result<T, djs_core::heap::HeapError>;

impl Session {
    fn finish(self, done: Done) -> RunResult {
        RunResult {
            output: self.output,
            done,
        }
    }

    fn host_fn(&mut self, host: Host, name: &str) -> HeapResult<Value> {
        self.vm.new_host_function(host as u32, name)
    }

    /// Defines the codemode globals; returns the `tools` and `console` arguments.
    fn install_globals(&mut self) -> HeapResult<(Value, Value)> {
        let console = self.vm.new_object()?;
        for level in ["log", "info", "warn", "error", "debug"] {
            let f = self.host_fn(Host::Log, level)?;
            self.vm.define(console, level, f)?;
        }
        let tools = self.vm.new_object()?;
        // Tools return promises, as pi's do (async host functions, D1374).
        for (host, name) in [(Host::Echo, "echo"), (Host::Fail, "fail")] {
            let f = self.vm.new_async_host_function(host as u32, name)?;
            self.vm.define(tools, name, f)?;
        }
        for (host, name) in [
            (Host::Text, "text"),
            (Host::Exit, "exit"),
            (Host::Store, "store"),
            (Host::Load, "load"),
        ] {
            let f = self.host_fn(host, name)?;
            self.vm.define_global(name, f)?;
        }
        let all = br#"[{"name":"echo","description":"Returns its arguments."},{"name":"fail","description":"Rejects with an Error carrying args.message."}]"#;
        let all_tools = parse_utf8(self.vm.heap_mut(), all, &ParseOptions::default())
            .map_err(|_| djs_core::heap::HeapError::OutOfMemory)?;
        self.vm.define_global("ALL_TOOLS", all_tools)?;
        self.vm.define_global("console", Value::object(console))?;
        self.vm.define_global("tools", Value::object(tools))?;
        Ok((Value::object(tools), Value::object(console)))
    }

    fn type_error(&mut self, msg: &str) -> HeapResult<Result<Value, Value>> {
        Ok(Err(self.vm.new_error(ErrorKind::TypeError, msg)?))
    }

    fn host_call(&mut self, host: Host, args: &[Value]) -> HeapResult<Result<Value, Value>> {
        let arg = |i: usize| args.get(i).copied().unwrap_or(Value::UNDEFINED);
        match host {
            Host::Log => {
                let parts: Vec<String> = args.iter().map(|&a| self.format(a)).collect();
                self.output.push(parts.join(" "));
                Ok(Ok(Value::UNDEFINED))
            }
            Host::Text => match self.output_text(arg(0)) {
                Ok(t) => {
                    self.output.push(t);
                    Ok(Ok(Value::UNDEFINED))
                }
                Err(msg) => self.type_error(&msg),
            },
            Host::Exit => unreachable!("handled by the run loop"),
            Host::Store => {
                let Some(key) = arg(0).as_string().map(|s| self.vm.heap().str_to_string(s)) else {
                    return self.type_error("store() key must be a string");
                };
                let v = arg(1);
                if v.kind() == Kind::Undefined {
                    self.store.remove(&key);
                    return Ok(Ok(Value::UNDEFINED));
                }
                let quoted = json_quote(&key);
                match stringify_fast(self.vm.heap_mut(), v, &[]) {
                    Ok(Some(s)) => {
                        let json = self.vm.heap().str_to_string(s);
                        self.store.insert(key, json);
                        Ok(Ok(Value::UNDEFINED))
                    }
                    Ok(None) => self.type_error(&format!("store({quoted}) value is not JSON-serializable")),
                    Err(StringifyError::Circular) => self.type_error(&format!(
                        "store({quoted}) value is not JSON-serializable: TypeError: circular reference"
                    )),
                    Err(e) => self.type_error(&format!("store({quoted}): djs cannot serialize this value yet ({e:?})")),
                }
            }
            Host::Load => {
                let Some(key) = arg(0).as_string().map(|s| self.vm.heap().str_to_string(s)) else {
                    return self.type_error("load() key must be a string");
                };
                match self.store.get(&key).cloned() {
                    None => Ok(Ok(Value::UNDEFINED)),
                    Some(json) => Ok(Ok(self.parse(&json)?)),
                }
            }
            Host::Echo => {
                // A JSON round trip, like pi's tools.
                match stringify_fast(self.vm.heap_mut(), arg(0), &[]) {
                    Ok(Some(s)) => {
                        let json = self.vm.heap().str_to_string(s);
                        Ok(Ok(self.parse(&json)?))
                    }
                    Ok(None) => Ok(Ok(Value::UNDEFINED)),
                    Err(StringifyError::Circular) => self.type_error("circular reference"),
                    Err(e) => {
                        self.type_error(&format!("djs cannot serialize this value yet ({e:?})"))
                    }
                }
            }
            Host::Fail => {
                let mut message = String::from("tool failed");
                if let Some(o) = arg(0).as_object() {
                    let k = self.vm.heap_mut().atom_str("message")?;
                    let v = self
                        .vm
                        .heap()
                        .find_property(o, k)
                        .and_then(|(_, p)| p.value());
                    if let Some(v) = v.filter(|v| v.kind() != Kind::Undefined) {
                        message = self.string_of(v);
                    }
                }
                Ok(Err(self.vm.new_error(ErrorKind::Error, &message)?))
            }
        }
    }

    fn parse(&mut self, json: &str) -> HeapResult<Value> {
        parse_utf8(
            self.vm.heap_mut(),
            json.as_bytes(),
            &ParseOptions::default(),
        )
        .map_err(|_| djs_core::heap::HeapError::OutOfMemory)
    }

    /// pi's `format` (console arguments): strings raw, errors as `Name: message`,
    /// everything else JSON, falling back to `String()`.
    fn format(&mut self, v: Value) -> String {
        if let Some(s) = v.as_string() {
            return self.vm.heap().str_to_string(s);
        }
        if let Some(e) = error_summary(self.vm.heap(), v) {
            return e;
        }
        if matches!(v.kind(), Kind::Undefined | Kind::Symbol) || self.is_callable(v) {
            return self.string_of(v);
        }
        match stringify_fast(self.vm.heap_mut(), v, &[]) {
            Ok(Some(s)) => self.vm.heap().str_to_string(s),
            Ok(None) => self.string_of(v),
            Err(StringifyError::Circular | StringifyError::StackOverflow) => self.string_of(v),
            Err(e) => format!("<djs: JSON.stringify needs the slow path ({e:?})>"),
        }
    }

    /// pi's `outputText` (text()): primitives as `String()`, objects as JSON.
    fn output_text(&mut self, v: Value) -> Result<String, String> {
        if v.as_object().is_none() || self.is_callable(v) {
            return Ok(self.string_of(v));
        }
        match stringify_fast(self.vm.heap_mut(), v, &[]) {
            Ok(Some(s)) => Ok(self.vm.heap().str_to_string(s)),
            Ok(None) => Ok(self.string_of(v)),
            Err(StringifyError::Circular) => Err("circular reference".into()),
            Err(e) => Ok(format!("<djs: JSON.stringify needs the slow path ({e:?})>")),
        }
    }

    /// `String(v)` for the values the glue meets. Objects only get approximations
    /// (no ToPrimitive from the host); the markers make the difference visible.
    fn string_of(&mut self, v: Value) -> String {
        let heap = self.vm.heap();
        match v.kind() {
            Kind::Undefined => "undefined".into(),
            Kind::Null => "null".into(),
            Kind::Bool => v.as_bool().unwrap().to_string(),
            Kind::Number => num::to_string(v.as_number().unwrap()).as_str().to_string(),
            Kind::String => heap.str_to_string(v.as_string().unwrap()),
            Kind::Symbol => {
                let d = heap.symbol_description(v.as_symbol().unwrap());
                let d = d
                    .as_string()
                    .map(|s| heap.str_to_string(s))
                    .unwrap_or_default();
                format!("Symbol({d})")
            }
            Kind::Object => {
                let o = v.as_object().unwrap();
                if heap.object_class(o).is_callable() {
                    "<djs: function source>".into()
                } else if heap.object_class(o) == djs_core::heap::ObjectClass::Array {
                    "<djs: String(array)>".into()
                } else {
                    "[object Object]".into()
                }
            }
            _ => format!("<djs: String() of {:?}>", v.kind()),
        }
    }

    fn is_callable(&self, v: Value) -> bool {
        v.as_object()
            .is_some_and(|o| self.vm.heap().object_class(o).is_callable())
    }

    /// The return value as the model sees it.
    fn result(&mut self, v: Value) -> Done {
        if let Some(s) = v.as_string() {
            return Done::Return(self.vm.heap().str_to_string(s));
        }
        // JSON.stringify gives undefined for these: there is no value to show.
        if matches!(v.kind(), Kind::Undefined | Kind::Symbol) || self.is_callable(v) {
            return Done::Completed;
        }
        match stringify_fast(self.vm.heap_mut(), v, &[]) {
            Ok(Some(s)) => Done::Return(self.vm.heap().str_to_string(s)),
            Ok(None) => Done::Completed,
            Err(StringifyError::Circular) => Done::Throw("TypeError: circular reference".into()),
            Err(StringifyError::StackOverflow) => {
                Done::Throw("InternalError: stack overflow".into())
            }
            Err(e) => Done::Crash(format!(
                "the return value needs JSON.stringify's slow path ({e:?})"
            )),
        }
    }

    /// The uncaught error as the model sees it.
    fn describe(&mut self, e: Value) -> String {
        match error_summary(self.vm.heap(), e) {
            Some(s) => s,
            None => format!("Error: {}", self.format(e)),
        }
    }
}

fn json_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
