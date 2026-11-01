//! num::to_string against Rust's shortest float formatting (scratch comparison).
use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;
fn main() {
    let mut seed = 7u32;
    let xs: Vec<f64> = (0..250_000)
        .map(|_| {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            (seed % 1_000_000_000) as f64 / 1000.0
        })
        .collect();
    for _ in 0..3 {
        let t = Instant::now();
        let mut n = 0;
        for &x in &xs {
            n += black_box(djs_core::num::to_string(x)).as_bytes().len();
        }
        let a = t.elapsed();
        let t = Instant::now();
        let mut s = String::new();
        for &x in &xs {
            s.clear();
            write!(s, "{x}").unwrap();
            n += black_box(&s).len();
        }
        let b = t.elapsed();
        println!(
            "djs {:.1} ns/num, std {:.1} ns/num ({n})",
            a.as_nanos() as f64 / 250e3,
            b.as_nanos() as f64 / 250e3
        );
    }
}
