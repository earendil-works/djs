//! Drives djs-wasm's exports natively, with exactly the call sequence of abidrive.mjs
//! (which drives the same exports in the wasm build), so the two images can be compared.
//! usage: abidrive <body.js> <slice> [trace-every]

use djs_wasm::*;

fn stage(b: &[u8]) {
    let p = djs_staging(b.len().max(1));
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), p as *mut u8, b.len()) };
}

fn out() -> Vec<u8> {
    let (p, n) = (djs_out_ptr(), djs_out_len());
    unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec()
}

fn words() -> Vec<u32> {
    out().chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn arena(vm: u32) -> Vec<u8> {
    let (p, n) = (vm_arena_ptr(vm), vm_arena_len(vm));
    unsafe { std::slice::from_raw_parts(p as *const u8, n as usize) }.to_vec()
}

fn fnv(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in bytes {
        h = (h ^ b as u32).wrapping_mul(0x01000193);
    }
    h
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read(&a[1]).unwrap();
    let slice: u32 = a[2].parse().unwrap();
    let trace: u64 = a.get(3).map_or(0, |s| s.parse().unwrap());
    let vm = vm_new(0);
    vm_set_now(vm, 1_700_000_000_000.0);
    vm_set_time_zone_offset(vm, 90);
    let g = vm_global(vm);
    let console = vm_new_object(vm);
    stage(b"log");
    let log = vm_new_host_function(vm, 1, 3);
    stage(b"log");
    vm_define(vm, console, 3, log);
    stage(b"console");
    vm_define(vm, g, 7, console);
    stage(b"tool");
    let tool = vm_new_async_host_function(vm, 2, 4);
    stage(b"tool");
    vm_define(vm, g, 4, tool);
    stage(&src);
    let f = vm_compile(vm, 1, 0, src.len());
    assert!(f != 0, "compile: {}", String::from_utf8_lossy(&out()));
    stage(&[]);
    vm_call(vm, f, 0, 0);
    let mut stops: u64 = 0;
    let mut pending: Vec<u32> = Vec::new();
    let mut logs = String::new();
    let result;
    loop {
        stops += 1;
        let st = vm_run(vm, slice);
        if trace > 0 && stops % trace == 0 {
            println!("stop {stops} {:08x}", fnv(&arena(vm)));
        }
        let n = vm_new_host_calls(vm);
        if n > 0 {
            pending.extend(words());
        }
        match st {
            0 | 1 => {
                let r = vm_result(vm);
                let ps = vm_promise_state(vm, r);
                let h = if ps >= 2 { words()[0] } else { r };
                vm_display(vm, h);
                result = format!("{} {}", if ps == 3 || st == 1 { "threw" } else { "ok" }, String::from_utf8_lossy(&out()));
                break;
            }
            2 => {}
            3 => {
                let argc = vm_host_call(vm);
                let w = words();
                let mut parts = Vec::new();
                for i in 0..argc as usize {
                    let bits = (w[4 + 2 * i] as u64) | ((w[5 + 2 * i] as u64) << 32);
                    parts.push(format!("{bits:016x}"));
                }
                logs += &parts.join(" ");
                logs.push('\n');
                vm_resolve_host_call(vm, w[0], 1, 0);
            }
            4 => {
                if pending.is_empty() {
                    result = "idle".into();
                    break;
                }
                for id in pending.drain(..) {
                    vm_resolve_host_call(vm, id, 1, 0);
                }
            }
            _ => panic!(),
        }
    }
    let raw = arena(vm);
    let rawh = fnv(&raw);
    vm_collect(vm);
    let col = fnv(&arena(vm));
    println!("build {:08x}", djs_build_hash());
    println!("logs {:08x}", fnv(logs.as_bytes()));
    println!("raw {rawh:08x} len {} collected {col:08x} stops {stops}", raw.len());
    println!("result {result}");
    if let Some(p) = a.get(4) {
        std::fs::write(p, &raw).unwrap();
    }
}
