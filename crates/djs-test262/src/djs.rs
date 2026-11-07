//! The `djs` engine: djs in-process, a fresh `Vm` per test (D1070).
//!
//! The harness files and the test are compiled as Scripts (D004) and run one after the
//! other in the same realm, with `this` = the global object. `print` and `$262` are host
//! functions: `$262.evalScript` is a small JS wrapper (run as a setup Script) around a
//! host function that compiles its argument as a Script in the same VM and returns the
//! function, which the wrapper then calls. Each run gets a fuel budget (a timeout when
//! used up) and a memory limit (`InternalError: out of memory` becomes `oom`).
//!
//! A `module` test is the root of a module graph (D3230): its key is its path relative to
//! `test/`, a specifier names the file relative to the importing module's directory, and
//! the loader reads the `*_FIXTURE.js` files from disk. The test alone is compiled first
//! (the parse phase), then the graph is loaded and linked (the resolution phase), then
//! evaluated; the evaluation's promise says how it ended. `import()` (in modules and
//! Scripts alike) loads through the same loader; a Script's key is its path too.

use std::time::Instant;

use std::path::{Path, PathBuf};

use djs_core::compiler::{CompileErrorKind, CompileOptions, compile_module};
use djs_core::heap::{DEFAULT_LIMIT, atoms};
use djs_core::interp::error_summary;
use djs_core::value::{Kind, Value};
use djs_core::vm::{
    ErrorKind, ModuleKind, ModuleLoader, ModuleSource, Status as VmStatus, Vm, VmOptions,
};

use crate::engine::{Engine, ErrorInfo, Job, RunResult, Status};

/// Fuel for one test (harness included). This is the limit that decides a timeout, since
/// it does not depend on machine load (D1790): the heaviest active tests (decodeURI's
/// A2.5_T1) need 2-4e8, and a debug build runs ~1e8 fuel/s, so a runaway test stops after
/// ~20 s. The wall-clock timeout, checked between slices, is only a hang guard.
pub const DEFAULT_FUEL: u64 = 2_000_000_000;
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
    /// Run the `@native` builtins as bytecode (`--backend bytecode`, 0063).
    pub bytecode_builtins: bool,
    /// Lazy functions (`--lazy`, `VmOptions::lazy_functions`, D2990).
    pub lazy: bool,
}

impl Default for Djs {
    fn default() -> Djs {
        Djs {
            fuel: DEFAULT_FUEL,
            bytecode_builtins: false,
            lazy: false,
        }
    }
}

/// How one Script ended.
enum Outcome {
    /// With the value the call returned (`undefined` if there was none).
    Completed(Value),
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
            exec: djs_core::vm::ExecOptions {
                native_builtins: !self.bytecode_builtins,
                ..Default::default()
            },
            #[cfg(feature = "lazy")]
            lazy_functions: self.lazy,
            ..VmOptions::default()
        };
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        vm.set_now(now());
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
        // `import()` loads from the fixture files (a no-op without modules).
        s.vm.set_dynamic_import(true)
            .map_err(|e| format!("set_dynamic_import: {e:?}"))?;
        match s.script(SETUP.as_bytes(), "<setup>")? {
            Ok(Outcome::Completed(_)) => {}
            Ok(Outcome::Threw(e)) => return Err(format!("$262 setup threw {e}")),
            Ok(Outcome::Timeout) => return Err("$262 setup timed out".into()),
            Err(e) => return Err(format!("$262 setup does not compile: {e}")),
        }
        for h in &job.harness {
            let status = match s.script(h.source.as_bytes(), &h.name)? {
                Ok(Outcome::Completed(_)) => continue,
                Ok(Outcome::Threw(error)) => s.harness_status(&h.name, error),
                Ok(Outcome::Timeout) => Status::Timeout,
                Err(error) => Status::HarnessError {
                    file: h.name.clone(),
                    error,
                },
            };
            return Ok(s.finish(status));
        }
        if job.module {
            let status = s.module()?;
            return Ok(s.finish(status));
        }
        let status = match s.script(job.source.as_bytes(), job.path)? {
            Ok(Outcome::Completed(_)) => Status::Completed,
            Ok(Outcome::Threw(e)) if is_oom(&e) => Status::OutOfMemory,
            Ok(Outcome::Threw(e)) => Status::RuntimeError(e),
            Ok(Outcome::Timeout) => Status::Timeout,
            Err(e) if is_oom(&e) => Status::OutOfMemory,
            Err(e) => Status::ParseError(e),
        };
        Ok(s.finish(status))
    }
}

/// The wall clock, in UTC (D1383): `ms` since the epoch.
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
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
        self.drive().map(Ok)
    }

    /// Runs the module test: the parse phase (the test alone), the resolution phase
    /// (loading and linking its graph), then its evaluation.
    fn module(&mut self) -> Result<Status, String> {
        let job = self.job;
        let opts = CompileOptions::module(job.path);
        if let Err(e) = compile_module(self.vm.heap_mut(), job.source.as_bytes(), &opts) {
            let e = compile_error(e.kind, &e.message);
            return Ok(if is_oom(&e) {
                Status::OutOfMemory
            } else {
                Status::ParseError(e)
            });
        }
        let mut loader = Fixtures {
            dir: test_dir(job),
            root: job.path,
            source: job.source,
        };
        let key = match self.vm.load_module(job.path, &mut loader) {
            Ok(key) => key,
            Err(e) => {
                let e = self.error_info(e);
                return Ok(if is_oom(&e) {
                    Status::OutOfMemory
                } else {
                    Status::ResolutionError(e)
                });
            }
        };
        if !self
            .vm
            .evaluate_module(&key)
            .map_err(|e| format!("Vm::evaluate_module: {e:?}"))?
        {
            return Err(format!("module {key} is not loaded after loading it"));
        }
        let e = match self.drive()? {
            Outcome::Completed(p) => match self.vm.promise_state(p) {
                Some(Some(Ok(_))) => return Ok(Status::Completed),
                Some(Some(Err(e))) => self.error_info(e),
                // Only top-level await leaves it pending with nothing left to run.
                Some(None) => return Err("the module's evaluation never settled".into()),
                None => return Err("Evaluate did not return a promise".into()),
            },
            Outcome::Threw(e) => e,
            Outcome::Timeout => return Ok(Status::Timeout),
        };
        Ok(if is_oom(&e) {
            Status::OutOfMemory
        } else {
            Status::RuntimeError(e)
        })
    }

    /// Runs the call in progress (and the jobs it queues) to the end, within the fuel
    /// budget, answering the host calls.
    fn drive(&mut self) -> Result<Outcome, String> {
        // The call's result, held while `import()`s are still answered and run.
        let mut result = None;
        loop {
            let slice = SLICE.min(self.fuel_left.max(1));
            let status = self.vm.run(slice);
            if let VmStatus::Done(_) | VmStatus::Idle = status {
                if let (VmStatus::Done(v), None) = (status, result) {
                    let h = self.vm.heap_mut().new_handle(v);
                    result = Some(h.map_err(|_| "out of memory for a handle")?);
                }
                if self.answer_imports()? {
                    continue;
                }
                let v = result.map_or(Value::UNDEFINED, |h| {
                    let v = self.vm.heap().get_handle(h);
                    self.vm.heap_mut().drop_handle(h);
                    v
                });
                return Ok(Outcome::Completed(v));
            }
            match status {
                VmStatus::Done(_) | VmStatus::Idle => unreachable!(),
                VmStatus::Threw(e) => return Ok(Outcome::Threw(self.error_info(e))),
                VmStatus::HostOutOfMemory => return Err("out of host memory".into()),
                VmStatus::OutOfFuel => {
                    // The clock moves between slices: a test may wait for it in a loop.
                    self.vm.set_now(now());
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
                        return Ok(Outcome::Timeout);
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

    /// Answers the `import()` calls made since the last time (they load from the fixture
    /// files, as a module test's graph does): whether there were any.
    fn answer_imports(&mut self) -> Result<bool, String> {
        let calls = self.vm.new_host_calls();
        let mut loader = Fixtures {
            dir: test_dir(self.job),
            root: self.job.path,
            source: self.job.source,
        };
        for &c in &calls {
            self.vm
                .answer_import(c, &mut loader)
                .map_err(|e| format!("Vm::answer_import: {e:?}"))?;
        }
        Ok(!calls.is_empty())
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
            DETACH => {
                // DetachArrayBuffer: the buffer drops its data (heap/typed.rs, D1390).
                match args.first().and_then(|v| v.as_object()) {
                    Some(b) if self.vm.heap_mut().detach_buffer(b) => Ok(Ok(Value::NULL)),
                    _ => Ok(Err(self
                        .vm
                        .new_error(ErrorKind::TypeError, "not an ArrayBuffer")?)),
                }
            }
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

/// `test/` of the checkout: the test's file without its path relative to it.
fn test_dir(job: &Job<'_>) -> PathBuf {
    let mut dir = job.file.to_path_buf();
    for _ in Path::new(job.path).components() {
        dir.pop();
    }
    dir
}

/// The module loader of a module test: keys are paths relative to `test/`, a specifier
/// names a file relative to the directory of the module that imports it (`./x.js`,
/// `x.js` and `../d/x.js` alike, as test262's fixtures use them), and the test itself
/// is the source the runner read.
struct Fixtures<'a> {
    dir: PathBuf,
    root: &'a str,
    source: &'a str,
}

impl ModuleLoader for Fixtures<'_> {
    fn resolve(
        &mut self,
        specifier: &str,
        referrer: Option<&str>,
        _attributes: &[(&str, &str)],
    ) -> Result<String, String> {
        let Some(referrer) = referrer else {
            return Ok(specifier.into());
        };
        let mut parts: Vec<&str> = referrer.split('/').collect();
        parts.pop();
        for seg in specifier.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    if parts.pop().is_none() {
                        return Err(format!("{specifier} from {referrer} is outside test/"));
                    }
                }
                s => parts.push(s),
            }
        }
        Ok(parts.join("/"))
    }

    fn load(&mut self, key: &str) -> Result<ModuleSource, String> {
        let text = if key == self.root {
            self.source.to_string()
        } else {
            std::fs::read_to_string(self.dir.join(key))
                .map_err(|e| format!("cannot load module {key}: {e}"))?
        };
        let kind = if key.ends_with(".json") {
            ModuleKind::Json
        } else {
            ModuleKind::JavaScript
        };
        Ok(ModuleSource { text, kind })
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
        CompileErrorKind::HostOutOfMemory => ErrorInfo {
            name: "HostOutOfMemory".into(),
            message: "out of host memory".into(),
        },
    }
}

fn is_unsupported(message: &str) -> bool {
    message.contains("not supported yet")
}
