//! The `djs` engine: djs in-process, a fresh `Vm` per case, running the shared prelude
//! (tests/corpus/prelude.js) exactly as the qjs engine does (D1100, D1440): the prelude
//! is evaluated as a strict Script, its setup function gets a `host` object whose
//! `output` and `done` are the only sync host functions and `later` the only async one
//! (D2590: every `later(value, rejected)` call stays pending until the run is idle, then
//! the oldest is settled with its value, one per idle point), and the case, compiled as the body
//! of the arrow `async (tools, console) => {...}` in a Script (D008), goes through
//! `api.run`. Once nothing is
//! left to run and the case has not finished, `api.stalled()` reports it, as the qjs
//! driver does from a timer.
//!
//! Stress variants: `suspend_every` runs N fuel at a time and continues each slice in a
//! fresh VM restored from the heap image; `gc_stress` collects at every safepoint.
//! `dump_every_idle` (0172) dumps the VM at every idle point (a format 2 dump, with the
//! prelude's `api` as a handle) and continues in a fresh VM that loaded the dump.
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
const LATER: u32 = 2;
/// A plain dump's `$djs.seed(lo, hi)` (D2832).
const SEED: u32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Djs {
    /// Suspend every N fuel and continue in a VM restored from the heap image.
    pub suspend_every: Option<u64>,
    /// Collect garbage at every safepoint.
    pub gc_stress: bool,
    /// Report the raw heap image's hash and the canonical hash (`--determinism`).
    pub image: bool,
    /// Run the `@native` builtins as bytecode (`--backend bytecode`, 0063).
    pub bytecode_builtins: bool,
    /// Dump at every idle point and continue in a fresh VM that loaded the dump.
    pub dump_every_idle: bool,
    /// With `dump_every_idle`: plain dumps (format 3, D2830), run as a Script in the
    /// fresh VM; a chain of at most `PLAIN_LINKS` dumps, then the run goes on.
    pub plain: bool,
}

/// The longest chain of plain dumps a case runs through (later idle points are not
/// dumped).
pub const PLAIN_LINKS: u32 = 50;

/// `--plain-out DIR`: every plain dump goes to `DIR/<case>/<link>.js`, with the output
/// lines before it in `<link>.pre` (for the node check, scripts/plain-node.mjs).
pub static PLAIN_OUT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

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
        if self.dump_every_idle {
            s += " --dump-every-idle";
        }
        if self.plain {
            s += " --plain";
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
                    idle_points: None,
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
            // The raw image, and the graph's canonical hash (D1730, 0101): what a final
            // collection used to compare, without depending on the collector's layout.
            let raw = fnv64(s.vm.image());
            s.vm.canonical_hash().ok().map(|h| (raw, h))
        } else {
            None
        };
        RunResult {
            output: s.output,
            done,
            image,
            idle_points: Some(s.idle_points),
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
    /// The idle points the case passed: `host.later` calls delivered.
    idle_points: u32,
    /// Fuel per `run`, the budget and what was used of it, the wall-clock start.
    slice: u64,
    budget: u64,
    used: u64,
    start: Instant,
    timeout: Duration,
    suspend: bool,
    /// Dump at every idle point (`dump_every_idle`): the options of a fresh VM and the
    /// clock to set in it.
    #[cfg_attr(not(feature = "dump"), allow(dead_code))]
    dump: Option<(VmOptions, f64)>,
    /// Plain dumps, and how many were taken.
    plain: bool,
    links: u32,
    /// The case's name (for `--plain-out`).
    #[cfg_attr(not(feature = "dump"), allow(dead_code))]
    name: String,
    /// The prelude's api object.
    api: Option<Handle>,
}

/// The corpus areas whose cases never change builtins, run with the prefix write barrier
/// armed (D2065, TESTING.md): in debug builds a write into the builtin prefix fails the
/// case. Other areas change builtins on purpose (spec tests of `defineProperty`, Proxy).
pub const ARMED_AREAS: &[&str] = &["codemode/", "codemode-harvest/"];

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
            prefix_barrier: ARMED_AREAS.iter().any(|a| job.name.starts_with(a)),
            ..VmOptions::default()
        };
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        #[cfg(not(feature = "dump"))]
        if self.dump_every_idle {
            return Err("--dump-every-idle needs the `dump` feature".into());
        }
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
            idle_points: 0,
            slice: self.suspend_every.unwrap_or(plain_slice).min(budget.max(1)),
            budget,
            used: 0,
            start: Instant::now(),
            timeout: job.timeout,
            suspend: self.suspend_every.is_some(),
            dump: self.dump_every_idle.then_some((opts, job.now)),
            plain: self.plain,
            links: 0,
            name: job.name.to_string(),
            api: None,
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
        for (id, name) in [(OUTPUT, "output"), (DONE, "done"), (LATER, "later")] {
            let f = if id == LATER {
                self.vm.new_async_host_function(id, name)
            } else {
                self.vm.new_host_function(id, name)
            }
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
        self.api = Some(api);

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
                self.call_api("run", &[f])?;
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
            self.call_api("stalled", &[])?;
        }
        Ok(())
    }

    /// Calls the method `name` of the prelude's api object to completion.
    fn call_api(&mut self, name: &str, args: &[Value]) -> Result<(), Stop> {
        let api = self.api.ok_or("no api")?;
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

    /// Dumps the idle VM and replaces it by a fresh VM that loaded the dump.
    #[cfg(feature = "dump")]
    fn dump_and_load(&mut self) -> Result<(), Stop> {
        use djs_core::vm::dump::DumpOptions;
        use djs_core::vm::dump_load::LoadOptions;
        let (opts, now) = self.dump.ok_or("no dump options")?;
        if self.plain {
            return self.plain_dump_and_load(opts, now);
        }
        let api = self.vm.heap().get_handle(self.api.ok_or("no api")?);
        let handles = [("api", api)];
        let text = self
            .vm
            .dump(
                &DumpOptions {
                    base: djs_prelude::BASE,
                    host_fn_name: &host_fn_name,
                    handles: &handles,
                    ..DumpOptions::default()
                },
                None,
            )
            .map_err(|e| {
                format!(
                    "dump at idle point {}: {}",
                    self.idle_points + 1,
                    e.message()
                )
            })?;
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        vm.set_now(now);
        let loaded = vm
            .load_dump(
                &text,
                &LoadOptions {
                    host_fn: &host_fn,
                    ..LoadOptions::default()
                },
            )
            .map_err(|e| {
                format!(
                    "load at idle point {}: {}\n{text}",
                    self.idle_points + 1,
                    e.message()
                )
            })?;
        let api = loaded
            .handles
            .iter()
            .find(|h| h.0 == "api")
            .map(|h| h.1)
            .ok_or("the dump lost the api handle")?;
        self.vm = vm;
        self.api = Some(api);
        Ok(())
    }

    /// [`Session::dump_and_load`] with a plain dump: the fresh VM gets the host's
    /// functions as globals (`output`, `done`, `later`, and `$djs` with `seed` and
    /// `resume`, which issues the call again) and runs the dump as a Script, whose
    /// completion value is the api.
    #[cfg(feature = "dump")]
    fn plain_dump_and_load(&mut self, opts: VmOptions, now: f64) -> Result<(), Stop> {
        use djs_core::vm::dump::DumpOptions;
        if self.links >= PLAIN_LINKS {
            return Ok(());
        }
        self.links += 1;
        let api = self.vm.heap().get_handle(self.api.ok_or("no api")?);
        let handles = [("run", api)];
        let text = self
            .vm
            .dump(
                &DumpOptions {
                    base: djs_prelude::BASE,
                    host_fn_name: &host_fn_name,
                    handles: &handles,
                    plain: true,
                    ..DumpOptions::default()
                },
                None,
            )
            .map_err(|e| {
                format!(
                    "plain dump at idle point {}: {}",
                    self.idle_points + 1,
                    e.message()
                )
            })?;
        if let Some(dir) = PLAIN_OUT.get() {
            let d = dir.join(&self.name);
            let w = std::fs::create_dir_all(&d)
                .and_then(|_| std::fs::write(d.join(format!("{}.js", self.links)), &text))
                .and_then(|_| {
                    let mut pre = self.output.join("\n");
                    if !pre.is_empty() {
                        pre.push('\n');
                    }
                    std::fs::write(d.join(format!("{}.pre", self.links)), pre)
                });
            w.map_err(|e| format!("--plain-out: {e}"))?;
        }
        let mut vm = djs_prelude::new_vm(opts).map_err(|e| format!("new_vm: {e:?}"))?;
        vm.set_now(now);
        let err = |e: djs_core::heap::HeapError| Stop::Crash(format!("{e:?}"));
        for (id, name) in [(OUTPUT, "output"), (DONE, "done"), (LATER, "later")] {
            let f = if id == LATER {
                vm.new_async_host_function(id, name)
            } else {
                vm.new_host_function(id, name)
            }
            .map_err(err)?;
            vm.define_global(name, f).map_err(err)?;
        }
        let hooks = vm.new_object().map_err(err)?;
        let seed = vm.new_host_function(SEED, "seed").map_err(err)?;
        vm.define(hooks, "seed", seed).map_err(err)?;
        vm.define_global("$djs", Value::object(hooks))
            .map_err(err)?;
        let old = std::mem::replace(&mut self.vm, vm);
        let resume = self.eval("(id, name, args) => later(...args)", "hooks.js")?;
        let key = self.vm.heap_mut().atom_str("$djs").map_err(err)?;
        let hooks = self
            .vm
            .heap()
            .find_property(self.vm.global(), key)
            .and_then(|(_, p)| p.value())
            .and_then(|v| v.as_object())
            .ok_or("no $djs")?;
        self.vm.define(hooks, "resume", resume).map_err(err)?;
        let api = match self.load_plain(&text) {
            Ok(api) => api,
            Err(e) => {
                self.vm = old;
                let msg = match e {
                    Stop::Crash(m) => m,
                    Stop::Timeout => "timeout".into(),
                };
                return Err(Stop::Crash(format!(
                    "plain load at idle point {}: {msg}\n{text}",
                    self.idle_points + 1
                )));
            }
        };
        let api = self.vm.heap_mut().new_handle(api).map_err(err)?;
        self.api = Some(api);
        Ok(())
    }

    /// Compiles and runs a small Script to its completion value (no idle point).
    #[cfg(feature = "dump")]
    fn eval(&mut self, src: &str, name: &str) -> Result<Value, Stop> {
        let f = self
            .vm
            .compile(src.as_bytes(), &CompileOptions::script(name))
            .map_err(|e| format!("{name}: {}", e.message))?;
        let global = Value::object(self.vm.global());
        self.call(f, global, &[], name)
    }

    /// Runs a plain dump to its first idle point: its completion value.
    #[cfg(feature = "dump")]
    fn load_plain(&mut self, text: &str) -> Result<Value, Stop> {
        let f = self
            .vm
            .compile(text.as_bytes(), &CompileOptions::script("dump.js"))
            .map_err(|e| format!("compile: {} at {}:{}", e.message, e.line, e.col))?;
        let global = Value::object(self.vm.global());
        self.vm
            .call(f, global, &[])
            .map_err(|e| format!("Vm::call: {e:?}"))?;
        loop {
            match self.vm.run(self.slice) {
                Status::Done(v) => return Ok(v),
                Status::Idle => {
                    return match self.vm.outcome() {
                        Some(Ok(v)) => Ok(v),
                        Some(Err(e)) => Err(Stop::Crash(format!("threw: {}", self.vm.display(e)))),
                        None => Err("idle without an outcome".into()),
                    };
                }
                Status::Threw(e) => {
                    return Err(Stop::Crash(format!("threw: {}", self.vm.display(e))));
                }
                Status::HostOutOfMemory => return Err("out of host memory".into()),
                Status::OutOfFuel => return Err(Stop::Timeout),
                Status::HostCall { call_id, fn_id } => self.host_call(call_id, fn_id)?,
            }
        }
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
                    // An idle point: no job, no frame, only `host.later` calls pending.
                    // The oldest gets its argument back (D2590).
                    #[cfg(feature = "dump")]
                    if self.dump.is_some() {
                        self.dump_and_load()?;
                    }
                    let pending = self.vm.pending_host_calls();
                    let &call_id = pending
                        .iter()
                        .min()
                        .ok_or_else(|| format!("{what}: idle with nothing pending"))?;
                    let info = self.vm.host_call(call_id).ok_or("host call vanished")?;
                    if info.fn_id != LATER {
                        return Err(Stop::Crash(format!(
                            "{what}: async host call to function {}",
                            info.fn_id
                        )));
                    }
                    let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
                    let rejected = info.args.get(1) == Some(&Value::TRUE);
                    self.idle_points += 1;
                    self.vm
                        .resolve_host_call(call_id, if rejected { Err(v) } else { Ok(v) })
                        .map_err(|e| format!("resolve_host_call: {e:?}"))?;
                }
                Status::OutOfFuel => {
                    self.used = self.used.saturating_add(self.slice);
                    if self.used >= self.budget || self.start.elapsed() >= self.timeout {
                        return Err(Stop::Timeout);
                    }
                    if self.suspend {
                        let o = self.vm.options();
                        self.vm = Vm::from_bytes(self.vm.image().to_vec(), o)
                            .map_err(|e| format!("Vm::from_bytes: {e:?}"))?;
                    }
                }
                Status::HostCall { call_id, fn_id } => self.host_call(call_id, fn_id)?,
            }
        }
    }

    /// A sync host call: `host.output`, `host.done`, a plain dump's `$djs.seed`.
    fn host_call(&mut self, call_id: u32, fn_id: u32) -> Result<(), Stop> {
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
            SEED => {
                let half = |v: Value| v.as_number().unwrap_or(0.0) as i32 as u32 as u64;
                let seed = (half(arg(1)) << 32) | half(arg(0));
                self.vm.set_math_random_seed(seed);
            }
            id => return Err(Stop::Crash(format!("unknown host function {id}"))),
        }
        self.vm
            .resolve_host_call(call_id, Ok(Value::UNDEFINED))
            .map_err(|e| format!("resolve_host_call: {e:?}"))?;
        Ok(())
    }
}

/// The host's names of its functions, for dumps (0172).
#[cfg(feature = "dump")]
fn host_fn_name(id: u32) -> Option<String> {
    match id {
        OUTPUT => Some("output".into()),
        DONE => Some("done".into()),
        LATER => Some("later".into()),
        _ => None,
    }
}

#[cfg(feature = "dump")]
fn host_fn(name: &str) -> Option<u32> {
    match name {
        "output" => Some(OUTPUT),
        "done" => Some(DONE),
        "later" => Some(LATER),
        _ => None,
    }
}
