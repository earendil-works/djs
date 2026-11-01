//! `djs-abidrive <body.js> <slice> [trace-every] [raw-out]`: runs a codemode body through
//! djs-wasm's exports natively, with exactly the export calls of
//! `scripts/determinism/abidrive.mjs` (which makes them in the wasm build), and prints the
//! build hash, a hash of the console.log arguments' raw bits, the raw and collected heap
//! image hashes, the number of `vm_run` stops and the result. The gate compares the two
//! outputs (scripts/determinism/native-vs-wasm.sh, 0072): native and wasm builds must
//! produce byte-identical heaps and spend the same fuel.
//!
//! The body gets `console.log` (sync) and `tool(x)` (async, answered with `undefined` at
//! IDLE); the clock is fixed and the time zone is +90 minutes.

use djs_wasm::*;

// The exports hand out addresses of their buffers, as they do to a wasm host; natively
// those are pointers into this process, valid until the next export call.

fn stage(b: &[u8]) {
    let p = djs_staging(b.len().max(1));
    // SAFETY: djs_staging made its buffer at least `b.len()` bytes and returned its address.
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), p as *mut u8, b.len()) };
}

fn out() -> Vec<u8> {
    let (p, n) = (djs_out_ptr(), djs_out_len());
    // SAFETY: the out buffer holds `n` bytes at `p` until the next export call.
    unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec()
}

fn words() -> Vec<u32> {
    out()
        .chunks(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn arena(vm: u32) -> Vec<u8> {
    let (p, n) = (vm_arena_ptr(vm), vm_arena_len(vm));
    // SAFETY: the arena's bytes, valid until the next export call.
    unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec()
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
    if a.len() < 3 {
        eprintln!("usage: djs-abidrive <body.js> <slice> [trace-every] [raw-out]");
        std::process::exit(2);
    }
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
        if trace > 0 && stops.is_multiple_of(trace) {
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
                result = format!(
                    "{} {}",
                    if ps == 3 || st == 1 { "threw" } else { "ok" },
                    String::from_utf8_lossy(&out())
                );
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
    println!(
        "raw {rawh:08x} len {} collected {col:08x} stops {stops}",
        raw.len()
    );
    println!("result {result}");
    if let Some(p) = a.get(4) {
        std::fs::write(p, &raw).unwrap();
    }
}
