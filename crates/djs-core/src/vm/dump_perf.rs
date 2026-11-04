//! Dump and load times at every idle point (0198), natively: the harvested corpus run as
//! the difftest runs it (dump_harvest.rs's `idle_run`), and the data-heavy cases of
//! scripts/dump-costs/native (fake tools that answer through `later`, made in JS).
//!
//! `DJS_DUMP_PERF=1 cargo test --release -p djs-core --lib vm::dump_perf -- --ignored --nocapture`
//!
//! - `DJS_DUMP_PERF=corpus|cases|all` (default all); `DJS_DUMP_PERF_FILTER=substr` runs only
//!   matching files;
//! - `DJS_DUMP_PERF_REPS=n`: dumps and loads per idle point, the best counts (default 3);
//! - `DJS_DUMP_PERF_OUT=dir`: writes every dump there (`<case>.<idle>.js`) for profiling;
//! - `DJS_DUMP_PERF_SKIP=load`: dumps only, `=all`: neither (for counting the
//!   instructions of each side with `/usr/bin/time -l` when the machine is too busy for
//!   timings: scripts/dump-costs/instructions.sh).

extern crate std;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::time::{Duration, Instant};

use super::dump::DumpOptions;
use super::dump_harvest::{base, host_fn, host_fn_name, idle_run_with, load_vm};
use super::*;

#[derive(Default)]
struct Point {
    dump: Duration,
    load: Duration,
    bytes: usize,
}

fn reps() -> usize {
    std::env::var("DJS_DUMP_PERF_REPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3)
        .max(1)
}

/// Dumps `vm` and loads the dump into fresh VMs `reps` times; the best of each.
fn measure(vm: &Vm, api: Value, name: &str, idle: usize, points: &mut Vec<Point>) {
    // The api is the dump's completion value (D2895).
    let handles = [("run", api)];
    let opts = DumpOptions {
        base: base(),
        host_fn_name: &host_fn_name,
        handles: &handles,
        ..Default::default()
    };
    let skip = std::env::var("DJS_DUMP_PERF_SKIP").unwrap_or_default();
    if skip == "all" {
        return;
    }
    let mut p = Point {
        dump: Duration::MAX,
        load: Duration::MAX,
        bytes: 0,
    };
    let mut text = String::new();
    // Each repetition with what the VM's earlier dumps left it, as the first dump here.
    let cache = vm.dump_cache();
    for _ in 0..reps() {
        vm.set_dump_cache(cache.clone());
        let t = Instant::now();
        match vm.dump(&opts, None) {
            Ok(s) => text = s,
            Err(_) => return,
        }
        p.dump = p.dump.min(t.elapsed());
    }
    p.bytes = text.len();
    if let Some(dir) = std::env::var_os("DJS_DUMP_PERF_OUT") {
        let path = std::path::Path::new(&dir).join(format!("{name}.{idle}.js"));
        std::fs::write(path, &text).unwrap();
    }
    if skip == "load" {
        points.push(p);
        return;
    }
    let mut serve = |vm: &mut Vm, id: u32, _: u32| {
        vm.resolve_host_call(id, Ok(Value::UNDEFINED)).unwrap();
    };
    for _ in 0..reps() {
        let mut b = load_vm();
        let t = Instant::now();
        if super::dump_tests::run_plain_dump(&mut b, &text, &host_fn, &mut serve).is_err() {
            return;
        }
        p.load = p.load.min(t.elapsed());
    }
    points.push(p);
}

fn pct(xs: &mut [Duration], p: usize) -> f64 {
    xs.sort();
    let i = (p * xs.len() / 100).min(xs.len() - 1);
    xs[i].as_secs_f64() * 1e3
}

fn summary(label: &str, ps: &[Point]) {
    if ps.is_empty() {
        std::println!("{label}: no idle points");
        return;
    }
    let mut d: Vec<Duration> = ps.iter().map(|p| p.dump).collect();
    let mut l: Vec<Duration> = ps.iter().map(|p| p.load).collect();
    let total = |xs: &[Duration]| xs.iter().map(|d| d.as_secs_f64()).sum::<f64>() * 1e3;
    let bytes: usize = ps.iter().map(|p| p.bytes).sum();
    std::println!(
        "{label}: {} idle points, {:.1} MB; dump total {:.0} ms, p50/p90/p99/max {:.3}/{:.3}/{:.3}/{:.3} ms; load total {:.0} ms, p50/p90/p99/max {:.3}/{:.3}/{:.3}/{:.3} ms",
        ps.len(),
        bytes as f64 / 1e6,
        total(&d),
        pct(&mut d, 50),
        pct(&mut d, 90),
        pct(&mut d, 99),
        pct(&mut d, 100),
        total(&l),
        pct(&mut l, 50),
        pct(&mut l, 90),
        pct(&mut l, 99),
        pct(&mut l, 100),
    );
}

fn files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let filter = std::env::var("DJS_DUMP_PERF_FILTER").unwrap_or_default();
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "js")
                && !p.file_name().unwrap().to_string_lossy().starts_with('_')
                && p.to_string_lossy().contains(filter.as_str())
        })
        .collect();
    v.sort();
    v
}

#[test]
#[ignore]
fn dump_perf() {
    let Ok(mode) = std::env::var("DJS_DUMP_PERF") else {
        return;
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let prelude = std::fs::read_to_string(root.join("tests/corpus/prelude.js")).unwrap();
    // The base and the VM's derived state outside the timings.
    let _ = base();
    if mode != "cases" {
        let mut ps = Vec::new();
        for f in files(&root.join("tests/corpus/codemode-harvest")) {
            let name = f.file_stem().unwrap().to_string_lossy().into_owned();
            let case = std::fs::read_to_string(&f).unwrap();
            let mut idle = 0;
            let _ = idle_run_with(&prelude, &case, &mut |vm, api| {
                idle += 1;
                measure(vm, api, &name, idle, &mut ps);
            });
        }
        summary("corpus", &ps);
    }
    if mode != "corpus" {
        let dir = root.join("scripts/dump-costs/native");
        let gen_src = std::fs::read_to_string(dir.join("_gen.js")).unwrap();
        for f in files(&dir) {
            let name = f.file_stem().unwrap().to_string_lossy().into_owned();
            let body = std::fs::read_to_string(&f).unwrap();
            let case =
                format!("{gen_src}\nreturn await (async (tools) => {{\n{body}\n}})(fakeTools);");
            let mut ps = Vec::new();
            let mut idle = 0;
            let r = idle_run_with(&prelude, &case, &mut |vm, api| {
                idle += 1;
                measure(vm, api, &name, idle, &mut ps);
            });
            if let Err(e) = r {
                std::println!("{name}: {e}");
            }
            summary(&name, &ps);
        }
    }
}
