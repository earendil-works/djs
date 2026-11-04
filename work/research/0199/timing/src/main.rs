//! 0199 timing: load time to the first idle point of today's format 2 dump (Vm::load_dump)
//! and of the plain dump of the same state (compile + run as an ordinary Script), best of
//! N fresh VMs each. Input: lines "today.dump.js plain.js"; output: "name today_ns plain_ns".
use std::time::Instant;

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::dump_load::LoadOptions;
use djs_core::vm::{Status, Vm, VmOptions};

const LATER: u32 = 3;
const PRINT: u32 = 1;

fn vm() -> Vm {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    vm.set_now(0.0);
    let later = vm.new_async_host_function(LATER, "later").unwrap();
    vm.define_global("later", later).unwrap();
    let print = vm.new_host_function(PRINT, "print").unwrap();
    vm.define_global("print", print).unwrap();
    let console = vm.new_object().unwrap();
    vm.define(console, "log", print).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let tools = vm.new_object().unwrap();
    vm.define_global("tools", Value::object(tools)).unwrap();
    vm
}

fn run_to_idle(vm: &mut Vm) {
    loop {
        match vm.run(u64::MAX) {
            Status::Idle | Status::Done(_) => return,
            Status::HostCall { call_id, .. } => vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap(),
            s => panic!("{s:?}"),
        }
    }
}

fn main() {
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let list = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    for line in list.lines() {
        let mut it = line.split_whitespace();
        let (t, p) = (it.next().unwrap(), it.next().unwrap());
        let today = std::fs::read_to_string(t).unwrap();
        let plain = std::fs::read_to_string(p).unwrap();
        let mut best_t = u128::MAX;
        let mut best_p = u128::MAX;
        for _ in 0..reps {
            let mut v = vm();
            let host_fn = |name: &str| (name == "later").then_some(LATER);
            let opts = LoadOptions { host_fn: &host_fn, ..LoadOptions::default() };
            let start = Instant::now();
            let r = v.load_dump(&today, &opts);
            let ns = start.elapsed().as_nanos();
            if r.is_err() {
                best_t = 0;
                break;
            }
            best_t = best_t.min(ns);
        }
        for _ in 0..reps {
            let mut v = vm();
            let start = Instant::now();
            let f = v.compile(plain.as_bytes(), &CompileOptions::script("plain.js")).unwrap();
            let this = Value::object(v.global());
            v.call(f, this, &[]).unwrap();
            run_to_idle(&mut v);
            best_p = best_p.min(start.elapsed().as_nanos());
        }
        println!("{t} {best_t} {best_p}");
    }
}
