//! The djs benchmark harness: component benchmarks of djs-core, the engine track
//! (JS microbenchmarks on qjs, later djs), and the wasm size track. Results can be
//! recorded in tests/scores/bench.tsv and compared against earlier runs.
//!
//! Adding a benchmark: write `fn(&mut Runner)` that calls `r.ops`, `r.bytes`, `r.time`
//! or `r.value` for each result, and add it to `GROUPS`.

pub mod core;
pub mod data;
pub mod engine;
pub mod runner;
pub mod scores;
pub mod size;
pub mod wasm;

use runner::Runner;

/// A benchmark group: reports its results through the runner.
pub type Group = fn(&mut Runner);

/// Benchmark groups in run order. Result names start with the group name.
pub const GROUPS: &[(&str, Group)] = &[
    ("lex", core::lexer),
    ("num", core::num),
    ("regexp", core::regexp),
    ("json", core::json),
    ("gc", core::gc),
    ("wire", core::wire),
    ("engine", engine::engine),
    ("size", size::size),
    ("wasm", wasm::wasm),
];

/// Runs every group (results are filtered by name inside the runner).
pub fn run_all(r: &mut Runner) {
    for (_, f) in GROUPS {
        f(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runner::Config;

    /// Quick mode: every benchmark once with small inputs. Part of `cargo test`, so the
    /// gate keeps the harness working.
    #[test]
    fn quick_smoke() {
        let mut r = Runner::new(Config {
            quick: true,
            ..Config::default()
        });
        let start = std::time::Instant::now();
        run_all(&mut r);
        let names: Vec<&str> = r.results.iter().map(|m| m.name.as_str()).collect();
        for prefix in [
            "lex/synthetic",
            "num/",
            "regexp/compile",
            "regexp/scan",
            "json/parse",
            "json/stringify",
            "gc/",
            "wire/decode",
            "wire/stringify",
        ] {
            assert!(
                names.iter().any(|n| n.starts_with(prefix)),
                "no {prefix} result in {names:?}"
            );
        }
        for m in &r.results {
            assert!(m.value.is_finite() && m.value >= 0.0, "{m:?}");
        }
        assert!(
            start.elapsed().as_secs() < 30,
            "quick mode took {:?}",
            start.elapsed()
        );
    }
}
