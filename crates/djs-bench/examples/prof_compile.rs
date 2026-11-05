//! Compiles files in a loop, for profiling the compiler (`samply record`) and native
//! compile throughput (0092):
//! `cargo run --release -p djs-bench --example prof_compile -- [seconds] files...`
//! A file ending in `.body.js` compiles as a codemode body, `.ts` as TypeScript.
//! `nCOUNT` instead of seconds runs exactly COUNT compiles per file (for instruction
//! counts with `/usr/bin/time -l`). With `DJS_LAZY=1` the compile is lazy (D2990):
//! functions not run at load get lazy templates.
use djs_core::compiler::{CompileOptions, compile, compile_lazy_unit};
use djs_core::heap::Heap;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (secs, count) = match args[0].strip_prefix('n') {
        Some(n) => (0.0, n.parse::<u64>().expect("count")),
        None => (args[0].parse::<f64>().expect("seconds"), u64::MAX),
    };
    let lazy = std::env::var("DJS_LAZY").is_ok_and(|v| v == "1");
    let compile = if lazy { compile_lazy_unit } else { compile };
    for path in &args[1..] {
        let src = std::fs::read(path).expect("read");
        let mut opts = if path.ends_with(".body.js") {
            CompileOptions::async_body("x.js", &[])
        } else {
            CompileOptions::script("x.js")
        };
        opts.typescript = path.ends_with(".ts");
        let end = Instant::now() + Duration::from_secs_f64(secs);
        let (mut n, mut best) = (0u64, f64::INFINITY);
        while n < count && (n == 0 || count != u64::MAX || Instant::now() < end) {
            let mut heap = Heap::new().unwrap();
            let t = Instant::now();
            std::hint::black_box(compile(&mut heap, &src, &opts).expect("compile"));
            best = best.min(t.elapsed().as_secs_f64());
            n += 1;
        }
        println!(
            "{path}\t{} KB\t{:.3} ms\t{:.1} MB/s\t({n} runs)",
            src.len() / 1000,
            best * 1e3,
            src.len() as f64 / best / 1e6
        );
    }
}
