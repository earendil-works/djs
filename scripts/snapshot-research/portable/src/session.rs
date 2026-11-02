//! Two tiny codemode hosts.
//!
//! - `Session::start`: a script compiled as an async body with two host functions,
//!   `tool(name, arg)` (async) and `emit(value)` (sync output); tool results are computed
//!   from the call's sequence number and arguments, so every build and every restore
//!   sees the same ones.
//! - `Session::start_case`: a harvested codemode case (tests/corpus/codemode-harvest) run
//!   like the difftest runner runs it: the corpus prelude (a Script) gets a host object
//!   with `output` and `done`, the case is `(async (tools, console) => {...})` in a
//!   Script, and `api.run` starts it. The case's fake tools resolve through promise jobs
//!   only, so each one first awaits `__tick()`, an async host function: every tool call
//!   becomes an idle point, as with a real host.
//!
//! One result is delivered per idle point, oldest call first.

use djs_core::compiler::CompileOptions;
use djs_core::heap::Handle;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

pub const TOOL: u32 = 1;
pub const EMIT: u32 = 2;
pub const TICK: u32 = 3;
pub const OUTPUT: u32 = 10;
pub const DONE: u32 = 11;

pub const PRELUDE: &str = include_str!("../../../../tests/corpus/prelude.js");

pub struct Session {
    pub vm: Vm,
    /// Pending async host calls in issue order: (VM call id, sequence number, function
    /// name, arguments as text).
    pub pending: Vec<(u32, u32, String, Vec<String>)>,
    /// Calls issued so far (the next sequence number).
    pub issued: u32,
    pub output: Vec<String>,
    pub finished: Option<String>,
    /// The scripts the VM's code came from: (name, source). Code is never exported.
    pub scripts: Vec<(String, String)>,
    /// The host's named roots (the corpus prelude's api object).
    pub roots: Vec<(String, Handle)>,
    stalled_called: bool,
}

pub fn new_vm() -> Vm {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).expect("vm");
    vm.set_now(1_700_000_000_000.0);
    vm.set_time_zone_offset(0);
    for (id, name, is_async) in [
        (TOOL, "tool", true),
        (EMIT, "emit", false),
        (TICK, "__tick", true),
    ] {
        let f = if is_async {
            vm.new_async_host_function(id, name)
        } else {
            vm.new_host_function(id, name)
        }
        .expect("oom");
        vm.define_global(name, f).expect("oom");
    }
    vm
}

/// The result of tool call number `seq`.
pub fn tool_result(seq: u32, name: &str, arg: &str) -> String {
    match name {
        "json" => format!(
            "{{\"seq\":{seq},\"arg\":{arg:?},\"items\":[{},{},{}],\"ok\":true}}",
            seq * 3,
            seq * 3 + 1,
            seq * 3 + 2
        ),
        "list" => (0..3)
            .map(|i| format!("{arg}-{seq}-{i}"))
            .collect::<Vec<_>>()
            .join(","),
        _ => format!("{name}({arg})#{seq}"),
    }
}

/// A harvested case as the difftest runner compiles it, its fake tools made to wait
/// for the host. Returns the source and the line where the script proper starts.
pub fn case_source(case: &str) -> (String, u32) {
    let case = case.replacen(
        "fake[name] = async (args) => {",
        "fake[name] = async (args) => { await __tick();",
        1,
    );
    let src = format!("(async (tools, console) => {{{case}\n}})");
    let line = src
        .lines()
        .position(|l| l.starts_with("return await (async (tools"))
        .map(|i| i as u32 + 1)
        .unwrap_or(0);
    (src, line)
}

impl Session {
    pub fn start(source: &str) -> Session {
        let mut vm = new_vm();
        let main = vm
            .compile(
                source.as_bytes(),
                &CompileOptions::async_body("script.js", &[]),
            )
            .unwrap_or_else(|e| panic!("compile: {} at {}:{}", e.message, e.line, e.col));
        vm.call(main, Value::UNDEFINED, &[]).expect("call");
        let mut s = Session::with_vm(vm);
        s.scripts = vec![("script.js".into(), source.into())];
        s.run();
        s
    }

    pub fn start_case(case: &str) -> Result<Session, String> {
        let vm = new_vm();
        let mut s = Session::with_vm(vm);
        let prelude =
            s.vm.compile(PRELUDE.as_bytes(), &CompileOptions::script("prelude.js"))
                .map_err(|e| e.message)?;
        let global = Value::object(s.vm.global());
        let setup = s.call_sync(prelude, global, &[])?;
        let host = s.vm.new_object().map_err(|e| format!("{e:?}"))?;
        for (id, name) in [(OUTPUT, "output"), (DONE, "done")] {
            let f =
                s.vm.new_host_function(id, name)
                    .map_err(|e| format!("{e:?}"))?;
            s.vm.define(host, name, f).map_err(|e| format!("{e:?}"))?;
        }
        let api = s.call_sync(setup, Value::UNDEFINED, &[Value::object(host)])?;
        let api =
            s.vm.heap_mut()
                .new_handle(api)
                .map_err(|e| format!("{e:?}"))?;
        s.roots.push(("api".into(), api));
        let (src, _) = case_source(case);
        let script =
            s.vm.compile(src.as_bytes(), &CompileOptions::script("case.js"))
                .map_err(|e| format!("compile: {}", e.message))?;
        let global = Value::object(s.vm.global());
        let f = s.call_sync(script, global, &[])?;
        s.scripts = vec![
            ("prelude.js".into(), PRELUDE.into()),
            ("case.js".into(), src),
        ];
        let run = s.api_method("run")?;
        let api = s.vm.heap().get_handle(api);
        s.vm.call(run, api, &[f]).map_err(|e| format!("{e:?}"))?;
        s.run();
        Ok(s)
    }

    pub fn with_vm(vm: Vm) -> Session {
        Session {
            vm,
            pending: Vec::new(),
            issued: 0,
            output: Vec::new(),
            finished: None,
            scripts: Vec::new(),
            roots: Vec::new(),
            stalled_called: false,
        }
    }

    fn api_method(&mut self, name: &str) -> Result<Value, String> {
        let (_, h) = self
            .roots
            .iter()
            .find(|(n, _)| n == "api")
            .ok_or("no api")?;
        let api = self.vm.heap().get_handle(*h).as_object().ok_or("api")?;
        let key = self
            .vm
            .heap_mut()
            .atom_str(name)
            .map_err(|e| format!("{e:?}"))?;
        self.vm
            .heap()
            .find_property(api, key)
            .and_then(|(_, p)| p.value())
            .ok_or(format!("no api.{name}"))
    }

    /// Calls `f`, which must finish without waiting on the host; its result.
    fn call_sync(&mut self, f: Value, this: Value, args: &[Value]) -> Result<Value, String> {
        self.vm.call(f, this, args).map_err(|e| format!("{e:?}"))?;
        loop {
            match self.vm.run(10_000_000) {
                Status::Done(v) => return Ok(v),
                Status::Threw(e) => return Err(format!("threw {}", self.vm.display(e))),
                Status::OutOfFuel => {}
                s => return Err(format!("{s:?} during a sync call")),
            }
        }
    }

    fn text(&mut self, v: Value) -> String {
        match v.as_string() {
            Some(s) => self.vm.heap().str_to_string(s),
            None => self.vm.display(v),
        }
    }

    /// Runs until the VM is idle (waiting on host calls) or done.
    pub fn run(&mut self) {
        loop {
            match self.vm.run(10_000_000) {
                Status::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).expect("call");
                    let arg = |i: usize| info.args.get(i).copied().unwrap_or(Value::UNDEFINED);
                    match fn_id {
                        EMIT | OUTPUT => {
                            let t = self.text(arg(0));
                            self.output.push(t);
                        }
                        DONE => {
                            let k = self.text(arg(0));
                            let t = self.text(arg(1));
                            self.finished = Some(format!("{k}: {t}"));
                        }
                        id => panic!("host function {id}"),
                    }
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .expect("resolve");
                }
                Status::OutOfFuel => {}
                st @ (Status::Idle | Status::Done(_)) => {
                    self.note_calls();
                    if self.pending.is_empty() && self.finished.is_none() {
                        if !self.roots.is_empty() && !self.stalled_called {
                            // Nothing left to run: the corpus prelude reports the stall.
                            self.stalled_called = true;
                            let f = self.api_method("stalled").expect("stalled");
                            let api = self.vm.heap().get_handle(self.roots[0].1);
                            self.vm.call(f, api, &[]).expect("call");
                            continue;
                        }
                        let done = matches!(st, Status::Done(_)) && self.roots.is_empty();
                        self.finished = Some(if done { "done" } else { "stalled" }.into());
                    }
                    return;
                }
                Status::Threw(e) => {
                    let d = self.vm.display(e);
                    self.finished = Some(format!("threw {d}"));
                    return;
                }
                Status::HostOutOfMemory => panic!("host out of memory"),
            }
        }
    }

    fn note_calls(&mut self) {
        for id in self.vm.new_host_calls() {
            let info = self.vm.host_call(id).expect("call");
            let name = match info.fn_id {
                TOOL => "tool",
                TICK => "__tick",
                _ => "?",
            };
            let args: Vec<String> = info.args.iter().map(|&v| self.text(v)).collect();
            self.pending.push((id, self.issued, name.into(), args));
            self.issued += 1;
        }
    }

    /// Delivers the oldest pending call's result and runs to the next idle point.
    pub fn deliver_one(&mut self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        let (id, seq, name, args) = self.pending.remove(0);
        let v = if name == "tool" {
            let a = |i: usize| args.get(i).cloned().unwrap_or_default();
            let r = tool_result(seq, &a(0), &a(1));
            Value::string(self.vm.heap_mut().alloc_str(&r).expect("oom"))
        } else {
            Value::UNDEFINED
        };
        self.vm.resolve_host_call(id, Ok(v)).expect("resolve");
        self.run();
        true
    }

    pub fn run_to_end(&mut self) {
        while self.deliver_one() {}
        if self.finished.is_none() {
            self.run();
        }
    }

    pub fn waiting(&self) -> bool {
        !self.pending.is_empty()
    }
}
