//! 0055 determinism probe: runs a script under different embedder schedules and compares
//! outputs and heap images.
//!
//! usage: detprobe <script.js> [--body] [--limit BYTES] [--async-policy eager|idle]
//!        [--slices a,b,c] [--no-restore]
//! Prints, per configuration: slice, restore, gc-stress, the output hash, the raw image
//! hash, the image hash after an explicit collect, the number of stops, and the output.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

const LOG: u32 = 1;
const TOOL: u32 = 2;

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Clone, Copy, PartialEq)]
enum Policy {
    /// Resolve async calls at the first stop after they were made (any status).
    Eager,
    /// Resolve async calls only when the VM is Idle.
    Idle,
}

struct Cfg {
    slice: u64,
    restore: bool,
    gc_stress: bool,
    limit: usize,
    body: bool,
    policy: Policy,
}

struct Outcome {
    out: String,
    raw: u64,
    collected: u64,
    stops: u64,
    len: usize,
}

fn run(src: &str, c: &Cfg) -> Outcome {
    let opts = VmOptions {
        memory_limit: c.limit,
        exec: ExecOptions {
            gc_stress: c.gc_stress,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut vm = djs_prelude::new_vm(opts).unwrap();
    vm.set_now(1_700_000_000_000.0);
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let tool = vm.new_async_host_function(TOOL, "tool").unwrap();
    vm.define_global("tool", tool).unwrap();
    let params = ["tools", "console"];
    let copts = if c.body {
        CompileOptions::async_body("probe.js", &params)
    } else {
        CompileOptions::script("probe.js")
    };
    let main = vm.compile(src.as_bytes(), &copts).expect("compiles");
    let args = if c.body {
        vec![Value::UNDEFINED, Value::object(console)]
    } else {
        vec![]
    };
    let this = Value::object(vm.global());
    vm.call(main, this, &args).unwrap();
    let mut out = String::new();
    let mut stops = 0u64;
    // Async calls made and not resolved yet, oldest first.
    let mut pending: Vec<u32> = Vec::new();
    loop {
        stops += 1;
        let st = vm.run(c.slice);
        for id in vm.new_host_calls() {
            pending.push(id);
        }
        let idle = matches!(st, Status::Idle);
        if !pending.is_empty() && (c.policy == Policy::Eager || idle) {
            for id in pending.drain(..) {
                let info = vm.host_call(id).unwrap();
                let x = info.args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
                vm.resolve_host_call(id, Ok(Value::number(x * 2.0))).unwrap();
            }
            if idle {
                continue;
            }
        }
        match st {
            Status::Done(v) => {
                if let Some(Some(Err(e))) = vm.promise_state(v) {
                    out += &format!("Uncaught {}\n", vm.display(e));
                }
                for r in vm.take_unhandled_rejections() {
                    out += &format!("unhandled {}\n", vm.display(r));
                }
                break;
            }
            Status::Threw(e) => {
                out += &format!("Uncaught {}\n", vm.display(e));
                break;
            }
            Status::Idle => {
                out += "IDLE with nothing pending\n";
                break;
            }
            Status::OutOfFuel => {}
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                out += &parts.join(" ");
                out.push('\n');
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
        }
        if c.restore {
            let o = vm.options();
            vm = Vm::from_bytes(vm.bytes().to_vec(), o).unwrap();
        }
    }
    let raw = fnv(vm.bytes());
    let len = vm.bytes().len();
    vm.collect().unwrap();
    let collected = fnv(vm.bytes());
    Outcome {
        out,
        raw,
        collected,
        stops,
        len,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut path = None;
    let mut body = false;
    let mut limit = 64 << 20;
    let mut policy = Policy::Idle;
    let mut slices: Vec<u64> = vec![u64::MAX, 1_000_000, 10_007, 997, 31];
    let mut restores = vec![false, true];
    let mut stress = vec![false, true];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--body" => body = true,
            "--limit" => limit = it.next().unwrap().parse().unwrap(),
            "--async-policy" => {
                policy = match it.next().unwrap().as_str() {
                    "eager" => Policy::Eager,
                    _ => Policy::Idle,
                }
            }
            "--slices" => {
                slices = it
                    .next()
                    .unwrap()
                    .split(',')
                    .map(|s| if s == "max" { u64::MAX } else { s.parse().unwrap() })
                    .collect()
            }
            "--no-restore" => restores = vec![false],
            "--no-stress" => stress = vec![false],
            _ => path = Some(a.clone()),
        }
    }
    let src = std::fs::read_to_string(path.expect("script")).unwrap();
    let mut first: Option<String> = None;
    for &gc_stress in &stress {
        for &restore in &restores {
            for &slice in &slices {
                if restore && slice == u64::MAX {
                    continue;
                }
                let c = Cfg {
                    slice,
                    restore,
                    gc_stress,
                    limit,
                    body,
                    policy,
                };
                let o = run(&src, &c);
                let same = match &first {
                    None => {
                        first = Some(o.out.clone());
                        "ref"
                    }
                    Some(f) if *f == o.out => "same",
                    Some(_) => "DIFF",
                };
                let slice_s = if slice == u64::MAX {
                    "max".to_string()
                } else {
                    slice.to_string()
                };
                println!(
                    "slice={slice_s:>8} restore={restore:<5} stress={gc_stress:<5} out={:016x} {same:<4} raw={:016x} len={:>8} collected={:016x} stops={}",
                    fnv(o.out.as_bytes()),
                    o.raw,
                    o.len,
                    o.collected,
                    o.stops
                );
                if same == "DIFF" || same == "ref" {
                    for l in o.out.lines().take(12) {
                        println!("    | {l}");
                    }
                }
            }
        }
    }
}
