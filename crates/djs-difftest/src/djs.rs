//! The `djs` engine: djs in-process, a fresh `Vm` per case, running the shared prelude
//! (tests/corpus/prelude.js) exactly as the qjs engine does (D1100, D1440): the prelude
//! is evaluated as a strict Script, its setup function gets a `host` object whose
//! `output` and `done` are the only host functions, and the case, compiled as the body
//! of the arrow `async (tools, console) => {...}` in a Script (D008), goes through
//! `api.run`. Once nothing is
//! left to run and the case has not finished, `api.stalled()` reports it, as the qjs
//! driver does from a timer.
//!
//! Stress variants: `suspend_every` runs N fuel at a time and continues each slice in a
//! fresh VM restored from the heap image; `gc_stress` collects at every safepoint.
//! Their transcripts must equal the plain run's (checked by the runner).

use std::time::{Duration, Instant};

use djs_core::compiler::{CompileErrorKind, CompileOptions};
use djs_core::heap::{DEFAULT_LIMIT, Handle};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

use crate::engine::{Done, Engine, Job, RunResult};

/// Fuel when the case has no `// difftest: fuel` directive. This is the limit that decides
/// a timeout, since it does not depend on machine load (D1790): every corpus case needs
/// less than 1e8, and a debug build runs ~1e8 fuel/s, so a runaway case stops after ~10 s.
/// The wall-clock timeout is only a hang guard.
pub const DEFAULT_FUEL: u64 = 1_000_000_000;
/// Fuel per `run` in the plain mode; the wall-clock timeout is checked between slices.
const SLICE: u64 = 10_000_000;
/// Fuel per `run` under `--gc-stress`.
const STRESS_SLICE: u64 = 20_000;

/// The host functions of the prelude's `host` object.
const OUTPUT: u32 = 0;
const DONE: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Djs {
    /// Suspend every N fuel and continue in a VM restored from the heap image.
    pub suspend_every: Option<u64>,
    /// Collect garbage at every safepoint.
    pub gc_stress: bool,
    /// Report a hash of the heap image after a final collection (`--determinism`).
    pub image: bool,
    /// Run the `@native` builtins as bytecode (`--backend bytecode`, 0063).
    pub bytecode_builtins: bool,
}

impl Djs {
    pub fn label(&self) -> String {
        let mut s = String::from("djs");
        if let Some(n) = self.suspend_every {
            s += &format!(" --suspend-every {n}");
        }
        if self.gc_stress {
            s += " --gc-stress";
        }
        if self.bytecode_builtins {
            s += " --backend bytecode";
        }
        s
    }
}

impl Engine for Djs {
    fn name(&self) -> &str {
        "djs"
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        let mut s = match self.session(job) {
            Ok(s) => s,
            Err(msg) => {
                return RunResult {
                    output: Vec::new(),
                    done: Done::Crash(msg),
                    image: None,
                };
            }
        };
        let done = match s.run_case(job) {
            Ok(()) => s.done.take().unwrap_or_else(|| {
                Done::Crash("the prelude finished without reporting a result".into())
            }),
            Err(Stop::Timeout) => Done::Timeout,
            Err(Stop::Crash(msg)) => Done::Crash(msg),
        };
        let image = if self.image && !matches!(done, Done::Timeout | Done::Crash(_)) {
            let raw = fnv64(s.vm.bytes());
            s.vm.collect().ok().map(|()| (raw, fnv64(s.vm.bytes())))
        } else {
            None
        };
        RunResult {
            output: s.output,
            done,
            image,
        }
    }
}

/// FNV-1a, 64 bits.
fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Why a session stopped early.
enum Stop {
    Timeout,
    Crash(String),
}

impl From<&str> for Stop {
    fn from(msg: &str) -> Stop {
        Stop::Crash(msg.into())
    }
}

impl From<String> for Stop {
    fn from(msg: String) -> Stop {
        Stop::Crash(msg)
    }
}

struct Session {
    vm: Vm,
    /// What the prelude reported through `host.output` and `host.done`.
    output: Vec<String>,
    done: Option<Done>,
    /// Fuel per `run`, the budget and what was used of it, the wall-clock start.
    slice: u64,
    budget: u64,
    used: u64,
    start: Instant,
    timeout: Duration,
    suspend: bool,
}

impl Djs {
    fn session(&self, job: &Job<'_>) -> Result<Session, String> {
        let opts = VmOptions {
            memory_limit: job
                .memory_limit
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT),
            exec: ExecOptions {
                gc_stress: self.gc_stress,
                native_builtins: !self.bytecode_builtins,
                ..ExecOptions::default()
            },
            ..VmOptions::default()
        };
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        // The wall clock, read once per case, in UTC (D1383); the qjs engine runs with
        // TZ=UTC to match.
        vm.set_now(job.now);
        let budget = job.fuel.unwrap_or(DEFAULT_FUEL);
        // Under GC stress a slice can take a long time (a collection at every safepoint),
        // so the wall-clock limit is checked more often.
        let plain_slice = if self.gc_stress { STRESS_SLICE } else { SLICE };
        Ok(Session {
            vm,
            output: Vec::new(),
            done: None,
            slice: self.suspend_every.unwrap_or(plain_slice).min(budget.max(1)),
            budget,
            used: 0,
            start: Instant::now(),
            timeout: job.timeout,
            suspend: self.suspend_every.is_some(),
        })
    }
}

impl Session {
    fn run_case(&mut self, job: &Job<'_>) -> Result<(), Stop> {
        // The prelude: a strict Script evaluating to `(host) => api` (its completion
        // value).
        let prelude = match self.vm.compile(
            job.prelude.as_bytes(),
            &CompileOptions::script("prelude.js"),
        ) {
            Ok(f) => f,
            Err(e) => return Err(Stop::Crash(format!("prelude: {}", e.message))),
        };
        let global = Value::object(self.vm.global());
        let setup = self.call(prelude, global, &[], "the prelude")?;
        let host = self.vm.new_object().map_err(|e| format!("{e:?}"))?;
        for (id, name) in [(OUTPUT, "output"), (DONE, "done")] {
            let f = self
                .vm
                .new_host_function(id, name)
                .map_err(|e| format!("{e:?}"))?;
            self.vm
                .define(host, name, f)
                .map_err(|e| format!("{e:?}"))?;
        }
        let api = self.call(setup, Value::UNDEFINED, &[Value::object(host)], "setup")?;
        let api = self
            .vm
            .heap_mut()
            .new_handle(api)
            .map_err(|e| format!("{e:?}"))?;

        // The case, as the qjs driver builds it: a strict Script evaluating the arrow
        // `async (tools, console) => {...}` (D008), so `this` is the global object. The
        // arrow starts on the case's first line, keeping its line numbers.
        let src = format!("(async (tools, console) => {{{}\n}})", job.source);
        match self.vm.compile(
            src.as_bytes(),
            &CompileOptions {
                typescript: job.ts,
                experimental_decorators: job.legacy_decorators,
                emit_decorator_metadata: job.decorator_metadata,
                ..CompileOptions::script(job.name)
            },
        ) {
            Ok(script) => {
                // (Read again: the global object moves when the heap is collected.)
                let global = Value::object(self.vm.global());
                let f = self.call(script, global, &[], "the case script")?;
                self.call_api(api, "run", &[f])?;
            }
            // The qjs driver hands a function that throws the SyntaxError to api.run,
            // which reports it as the uncaught error.
            Err(e) if e.kind == CompileErrorKind::Syntax => {
                self.done = Some(Done::Throw(format!("SyntaxError: {}", e.message)));
                return Ok(());
            }
            Err(e) => return Err(Stop::Crash(format!("compile: {}", e.message))),
        }
        // Nothing is left to run (the job queue is empty and no host call is pending).
        if self.done.is_none() {
            self.call_api(api, "stalled", &[])?;
        }
        Ok(())
    }

    /// Calls the method `name` of the prelude's api object to completion.
    fn call_api(&mut self, api: Handle, name: &str, args: &[Value]) -> Result<(), Stop> {
        let api = self.vm.heap().get_handle(api);
        let o = api.as_object().ok_or("api is not an object")?;
        let key = self
            .vm
            .heap_mut()
            .atom_str(name)
            .map_err(|e| format!("{e:?}"))?;
        let f = self
            .vm
            .heap()
            .find_property(o, key)
            .and_then(|(_, p)| p.value())
            .ok_or_else(|| format!("api.{name} is missing"))?;
        self.call(f, api, args, name)?;
        Ok(())
    }

    /// Calls `f` and runs until the call and every job it queued are done; the call's
    /// result. Host calls are the prelude's `host.output` and `host.done`.
    fn call(&mut self, f: Value, this: Value, args: &[Value], what: &str) -> Result<Value, Stop> {
        self.vm
            .call(f, this, args)
            .map_err(|e| format!("Vm::call ({what}): {e:?}"))?;
        loop {
            match self.vm.run(self.slice) {
                Status::Done(v) => return Ok(v),
                Status::Threw(e) => {
                    let msg = self.vm.display(e);
                    return Err(Stop::Crash(format!("{what} threw: {msg}")));
                }
                Status::HostOutOfMemory => {
                    return Err(Stop::Crash(format!("{what}: out of host memory")));
                }
                Status::Idle => {
                    // The prelude makes no async host calls.
                    return Err(Stop::Crash(format!("{what}: idle with pending host calls")));
                }
                Status::OutOfFuel => {
                    self.used = self.used.saturating_add(self.slice);
                    if self.used >= self.budget || self.start.elapsed() >= self.timeout {
                        return Err(Stop::Timeout);
                    }
                    if self.suspend {
                        let o = self.vm.options();
                        self.vm = Vm::from_bytes(self.vm.bytes().to_vec(), o)
                            .map_err(|e| format!("Vm::from_bytes: {e:?}"))?;
                    }
                }
                Status::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).ok_or("host call vanished")?;
                    let arg = |i: usize| info.args.get(i).copied().unwrap_or(Value::UNDEFINED);
                    let text = |s: &Session, v: Value| {
                        v.as_string()
                            .map(|t| s.vm.heap().str_to_string(t))
                            .ok_or_else(|| Stop::Crash("host: expected a string".into()))
                    };
                    match fn_id {
                        OUTPUT => {
                            let t = text(self, arg(0))?;
                            self.output.push(t);
                        }
                        DONE => {
                            let kind = text(self, arg(0))?;
                            let t = text(self, arg(1))?;
                            self.done = Some(match kind.as_str() {
                                "completed" => Done::Completed,
                                "return" => Done::Return(t),
                                "throw" => Done::Throw(t),
                                k => return Err(Stop::Crash(format!("host.done: kind {k}"))),
                            });
                        }
                        id => return Err(Stop::Crash(format!("unknown host function {id}"))),
                    }
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .map_err(|e| format!("resolve_host_call: {e:?}"))?;
                }
            }
        }
    }
}
