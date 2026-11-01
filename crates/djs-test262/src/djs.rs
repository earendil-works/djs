//! The `djs` engine: djs in-process, a fresh `Vm` per test (D1070).
//!
//! The harness files and the test are compiled as Scripts (D004) and run one after the
//! other in the same realm, with `this` = the global object. `print` and `$262` are host
//! functions: `$262.evalScript` is a small JS wrapper (run as a setup Script) around a
//! host function that compiles its argument as a Script in the same VM and returns the
//! function, which the wrapper then calls. Each run gets a fuel budget (a timeout when
//! used up) and a memory limit (`InternalError: out of memory` becomes `oom`).

use std::time::Instant;

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::heap::{DEFAULT_LIMIT, atoms};
use djs_core::interp::error_summary;
use djs_core::value::{Kind, Value};
use djs_core::vm::{ErrorKind, Status as VmStatus, Vm, VmOptions};

use crate::engine::{Engine, ErrorInfo, Job, RunResult, Status};

/// Fuel for one test (harness included): generous, since it is only hang protection;
/// the wall-clock timeout is checked between slices.
pub const DEFAULT_FUEL: u64 = 5_000_000_000;
const SLICE: u64 = 10_000_000;

const PRINT: u32 = 0;
const COMPILE_SCRIPT: u32 = 1;
const DETACH: u32 = 2;
const GC: u32 = 3;

/// Defines `$262.evalScript` on top of the host compile function.
const SETUP: &str = "$262.evalScript = function evalScript(src) { return $262.__compileScript(src).call($262.global); };";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Djs {
    pub fuel: u64,
}

impl Default for Djs {
    fn default() -> Djs {
        Djs { fuel: DEFAULT_FUEL }
    }
}

/// How one Script ended.
enum Outcome {
    Completed,
    Threw(ErrorInfo),
    /// Out of fuel or past the wall-clock limit (which one is in `Session::limit`).
    Timeout,
}

struct Session<'a> {
    vm: Vm,
    printed: Vec<String>,
    job: &'a Job<'a>,
    fuel_left: u64,
    start: Instant,
    /// Which limit ended the run, for the failure reason.
    limit: String,
}

impl Engine for Djs {
    fn name(&self) -> &str {
        "djs"
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        match self.run_job(job) {
            Ok(r) => r,
            Err(msg) => RunResult::new(Status::Crash(msg)),
        }
    }
}

impl Djs {
    fn run_job(&self, job: &Job<'_>) -> Result<RunResult, String> {
        let opts = VmOptions {
            memory_limit: job
                .memory_limit
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT),
            ..VmOptions::default()
        };
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        // The wall clock, in UTC (D1383).
        vm.set_now(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0),
        );
        let mut s = Session {
            vm,
            printed: Vec::new(),
            job,
            fuel_left: self.fuel,
            start: Instant::now(),
            limit: String::new(),
        };
        s.install_host()
            .map_err(|e| format!("installing $262: {e:?}"))?;
        match s.script(SETUP.as_bytes(), "<setup>")? {
            Ok(Outcome::Completed) => {}
            Ok(Outcome::Threw(e)) => return Err(format!("$262 setup threw {e}")),
            Ok(Outcome::Timeout) => return Err("$262 setup timed out".into()),
            Err(e) => return Err(format!("$262 setup does not compile: {e}")),
        }
        for h in &job.harness {
            let status = match s.script(h.source.as_bytes(), &h.name)? {
                Ok(Outcome::Completed) => continue,
                Ok(Outcome::Threw(error)) => s.harness_status(&h.name, error),
                Ok(Outcome::Timeout) => Status::Timeout,
                Err(error) => Status::HarnessError {
                    file: h.name.clone(),
                    error,
                },
            };
            return Ok(s.finish(status));
        }
        let status = match s.script(job.source.as_bytes(), job.path)? {
            Ok(Outcome::Completed) => Status::Completed,
            Ok(Outcome::Threw(e)) if is_oom(&e) => Status::OutOfMemory,
            Ok(Outcome::Threw(e)) => Status::RuntimeError(e),
            Ok(Outcome::Timeout) => Status::Timeout,
            Err(e) if is_oom(&e) => Status::OutOfMemory,
            Err(e) => Status::ParseError(e),
        };
        Ok(s.finish(status))
    }
}

fn is_oom(e: &ErrorInfo) -> bool {
    e.name == "InternalError" && e.message == "out of memory"
}

type HeapResult<T> = Result<T, djs_core::heap::HeapError>;

impl Session<'_> {
    fn finish(self, status: Status) -> RunResult {
        RunResult {
            status,
            printed: self.printed,
            diagnostics: self.limit,
        }
    }

    fn harness_status(&self, file: &str, error: ErrorInfo) -> Status {
        if is_oom(&error) {
            Status::OutOfMemory
        } else {
            Status::HarnessError {
                file: file.into(),
                error,
            }
        }
    }

    fn install_host(&mut self) -> HeapResult<()> {
        let print = self.vm.new_host_function(PRINT, "print")?;
        self.vm.define_global("print", print)?;
        let o = self.vm.new_object()?;
        let global = Value::object(self.vm.global());
        self.vm.define(o, "global", global)?;
        for (id, name) in [
            (COMPILE_SCRIPT, "__compileScript"),
            (DETACH, "detachArrayBuffer"),
            (GC, "gc"),
        ] {
            let f = self.vm.new_host_function(id, name)?;
            self.vm.define(o, name, f)?;
        }
        self.vm.define_global("$262", Value::object(o))
    }

    /// Compiles and runs one Script. The outer error is an engine failure; the inner
    /// `Err` is a compile error (the parse phase).
    fn script(&mut self, src: &[u8], name: &str) -> Result<Result<Outcome, ErrorInfo>, String> {
        let f = match self.vm.compile(src, &CompileOptions::script(name)) {
            Ok(f) => f,
            Err(e) => return Ok(Err(compile_error(e.kind, &e.message))),
        };
        let this = Value::object(self.vm.global());
        self.vm
            .call(f, this, &[])
            .map_err(|e| format!("Vm::call: {e:?}"))?;
        loop {
            let slice = SLICE.min(self.fuel_left.max(1));
            match self.vm.run(slice) {
                VmStatus::Done(_) | VmStatus::Idle => return Ok(Ok(Outcome::Completed)),
                VmStatus::Threw(e) => return Ok(Ok(Outcome::Threw(self.error_info(e)))),
                VmStatus::OutOfFuel => {
                    self.fuel_left = self.fuel_left.saturating_sub(slice);
                    if self.fuel_left == 0 || self.start.elapsed() >= self.job.timeout {
                        self.limit = if self.fuel_left == 0 {
                            "the fuel budget is used up".into()
                        } else {
                            format!(
                                "past the wall-clock limit of {:.1}s",
                                self.job.timeout.as_secs_f64()
                            )
                        };
                        self.vm.abort();
                        return Ok(Ok(Outcome::Timeout));
                    }
                }
                VmStatus::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).ok_or("host call vanished")?;
                    let result = self
                        .host_call(fn_id, &info.args)
                        .map_err(|e| format!("host call: {e:?}"))?;
                    self.vm
                        .resolve_host_call(call_id, result)
                        .map_err(|e| format!("resolve_host_call: {e:?}"))?;
                }
            }
        }
    }

    fn host_call(&mut self, fn_id: u32, args: &[Value]) -> HeapResult<Result<Value, Value>> {
        let arg = args.first().copied().unwrap_or(Value::UNDEFINED);
        match fn_id {
            PRINT => {
                let line = match arg.as_string() {
                    Some(s) => self.vm.heap().str_to_string(s),
                    None => self.vm.display(arg),
                };
                self.printed.push(line);
                Ok(Ok(Value::UNDEFINED))
            }
            COMPILE_SCRIPT => {
                // Compiling while parked only allocates; the wrapper calls the result.
                let Some(src) = arg.as_string().map(|s| self.vm.heap().str_to_string(s)) else {
                    return Ok(Err(self
                        .vm
                        .new_error(ErrorKind::TypeError, "evalScript needs a string")?));
                };
                match self
                    .vm
                    .compile(src.as_bytes(), &CompileOptions::script("<evalScript>"))
                {
                    Ok(f) => Ok(Ok(f)),
                    Err(e) if e.kind == CompileErrorKind::Syntax => Ok(Err(self
                        .vm
                        .new_error(ErrorKind::SyntaxError, &e.message)?)),
                    Err(_) => Ok(Err(self
                        .vm
                        .new_error(ErrorKind::InternalError, "out of memory")?)),
                }
            }
            DETACH => Ok(Err(self.vm.new_error(
                ErrorKind::TypeError,
                "$262.detachArrayBuffer is not supported by djs yet",
            )?)),
            // A collection here would be safe, but nothing needs it yet.
            GC => Ok(Ok(Value::UNDEFINED)),
            _ => Ok(Err(self
                .vm
                .new_error(ErrorKind::TypeError, "unknown host function")?)),
        }
    }

    /// The thrown value's constructor name (what negative tests specify) and message,
    /// read from data properties only: the host never runs JS.
    fn error_info(&mut self, e: Value) -> ErrorInfo {
        let Some(o) = e.as_object() else {
            let name = match e.kind() {
                Kind::Undefined => "undefined",
                Kind::Null => "object",
                Kind::Bool => "boolean",
                Kind::Number => "number",
                Kind::String => "string",
                Kind::Symbol => "symbol",
                _ => "value",
            };
            return ErrorInfo {
                name: name.into(),
                message: self.vm.display(e),
            };
        };
        let heap = self.vm.heap();
        let string_prop = |obj, key| {
            heap.find_property(obj, key)
                .and_then(|(_, p)| p.value())
                .and_then(|v| v.as_string())
                .map(|s| heap.str_to_string(s))
        };
        let ctor_name = heap
            .find_property(o, atoms::CONSTRUCTOR)
            .and_then(|(_, p)| p.value())
            .and_then(|v| v.as_object())
            .and_then(|c| string_prop(c, atoms::NAME))
            .filter(|n| !n.is_empty());
        let name = ctor_name
            .or_else(|| string_prop(o, atoms::NAME))
            .unwrap_or_else(|| "Object".into());
        let message = string_prop(o, atoms::MESSAGE)
            .unwrap_or_else(|| error_summary(heap, e).unwrap_or_default());
        ErrorInfo { name, message }
    }
}

/// A compile error as a parse-phase error. The compiler reports constructs it does not
/// implement yet as SyntaxErrors whose message ends in "not supported yet"; those are not
/// JS early errors, so they get the name `Unsupported` and a negative test that expects a
/// SyntaxError does not pass by accident (D1071).
fn compile_error(kind: CompileErrorKind, message: &str) -> ErrorInfo {
    match kind {
        CompileErrorKind::Syntax if is_unsupported(message) => ErrorInfo {
            name: "Unsupported".into(),
            message: message.into(),
        },
        CompileErrorKind::Syntax => ErrorInfo {
            name: "SyntaxError".into(),
            message: message.into(),
        },
        CompileErrorKind::OutOfMemory => ErrorInfo {
            name: "InternalError".into(),
            message: "out of memory".into(),
        },
    }
}

fn is_unsupported(message: &str) -> bool {
    message.contains("not supported yet")
}
