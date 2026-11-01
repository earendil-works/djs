//! Loops one boundary workload for profiling (`sample`, samply):
//! `cargo run --release --example prof_loop -- <stringify|decode|parse> <dataset> [seconds]`
use djs_bench::data::datasets;
use djs_core::heap::Heap;
use djs_core::json::{ParseOptions, parse_utf8, stringify_fast};
use djs_core::wire::{DecodeOptions, decode};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (what, ds) = (args[1].as_str(), args[2].as_str());
    let secs: u64 = args.get(3).map_or(5, |s| s.parse().unwrap());
    let data = datasets(false)
        .into_iter()
        .find(|(n, _)| *n == ds)
        .expect("dataset")
        .1;
    let (wire, json) = (data.to_wire(), data.to_json());
    let mut h = Heap::new().unwrap();
    let v = decode(&mut h, &wire, &DecodeOptions::default()).unwrap();
    let hv = h.new_handle(v).unwrap();
    let end = Instant::now() + Duration::from_secs(secs);
    let mut n = 0u64;
    while Instant::now() < end {
        match what {
            "stringify" => {
                let v = h.get_handle(hv);
                black_box(stringify_fast(&mut h, v, &[]).unwrap());
            }
            "decode" => {
                black_box(decode(&mut h, &wire, &DecodeOptions::default()).unwrap());
            }
            _ => {
                black_box(parse_utf8(&mut h, json.as_bytes(), &ParseOptions::default()).unwrap());
            }
        }
        n += 1;
        if n.is_multiple_of(8) {
            h.collect().unwrap();
        }
    }
    eprintln!("{n} iterations");
}
