//! A tiny codemode host: a script compiled as an async body, two host functions
//! (`tool(name, arg)`: async, `emit(value)`: sync output), tool results computed from the
//! call's sequence number and arguments, so every build and every restore sees the same
//! ones. One result is delivered per idle point, oldest call first.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

pub const TOOL: u32 = 1;
pub const EMIT: u32 = 2;

pub struct Session {
    pub vm: Vm,
    /// Pending tool calls in issue order: (VM call id, sequence number, name, argument).
    pub pending: Vec<(u32, u32, String, String)>,
    /// Calls issued so far (the next sequence number).
    pub issued: u32,
    pub output: Vec<String>,
    pub finished: Option<String>,
}

pub fn new_vm() -> Vm {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).expect("vm");
    vm.set_now(1_700_000_000_000.0);
    vm.set_time_zone_offset(0);
    let tool = vm.new_async_host_function(TOOL, "tool").expect("oom");
    vm.define_global("tool", tool).expect("oom");
    let emit = vm.new_host_function(EMIT, "emit").expect("oom");
    vm.define_global("emit", emit).expect("oom");
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
        "list" => (0..3).map(|i| format!("{arg}-{seq}-{i}")).collect::<Vec<_>>().join(","),
        _ => format!("{name}({arg})#{seq}"),
    }
}

impl Session {
    pub fn start(source: &str) -> Session {
        let mut vm = new_vm();
        let main = vm
            .compile(source.as_bytes(), &CompileOptions::async_body("script.js", &[]))
            .unwrap_or_else(|e| panic!("compile: {} at {}:{}", e.message, e.line, e.col));
        vm.call(main, Value::UNDEFINED, &[]).expect("call");
        let mut s = Session::with_vm(vm);
        s.run();
        s
    }

    pub fn with_vm(vm: Vm) -> Session {
        Session {
            vm,
            pending: Vec::new(),
            issued: 0,
            output: Vec::new(),
            finished: None,
        }
    }

    /// Runs until the VM is idle (waiting on tool calls) or done.
    pub fn run(&mut self) {
        loop {
            match self.vm.run(10_000_000) {
                Status::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).expect("call");
                    assert_eq!(fn_id, EMIT);
                    let text = info
                        .args
                        .first()
                        .map(|&v| self.vm.display(v))
                        .unwrap_or_default();
                    self.output.push(text);
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .expect("resolve");
                }
                Status::OutOfFuel => {}
                Status::Idle => {
                    self.note_calls();
                    if self.pending.is_empty() && self.finished.is_none() {
                        self.finished = Some("stalled".into());
                    }
                    return;
                }
                Status::Done(v) => {
                    self.note_calls();
                    let d = self.vm.display(v);
                    self.finished = Some(format!("done {d}"));
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
            let name = info.args.first().map(|&v| self.vm.display(v)).unwrap_or_default();
            let arg = info.args.get(1).map(|&v| self.vm.display(v)).unwrap_or_default();
            self.pending.push((id, self.issued, name, arg));
            self.issued += 1;
        }
    }

    /// Delivers the oldest pending call's result and runs to the next idle point.
    pub fn deliver_one(&mut self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        let (id, seq, name, arg) = self.pending.remove(0);
        let r = tool_result(seq, &name, &arg);
        let s = self.vm.heap_mut().alloc_str(&r).expect("oom");
        self.vm
            .resolve_host_call(id, Ok(Value::string(s)))
            .expect("resolve");
        self.run();
        true
    }

    pub fn run_to_end(&mut self) {
        while self.finished.is_none() && self.deliver_one() {}
    }
}
