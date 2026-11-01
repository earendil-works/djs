//! Timing: warmup, repeated samples, median and minimum.

use std::time::{Duration, Instant};

/// What a result measures. Throughput units are higher-is-better; the rest lower.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// nanoseconds per operation
    NsPerOp,
    /// megabytes (10^6 bytes) per second
    MBps,
    /// milliseconds per run
    Ms,
    /// milliseconds per MiB (GC: collection time per MiB live)
    MsPerMiB,
    /// bytes (sizes)
    Bytes,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::NsPerOp => "ns/op",
            Unit::MBps => "MB/s",
            Unit::Ms => "ms",
            Unit::MsPerMiB => "ms/MiB",
            Unit::Bytes => "bytes",
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        [
            Unit::NsPerOp,
            Unit::MBps,
            Unit::Ms,
            Unit::MsPerMiB,
            Unit::Bytes,
        ]
        .into_iter()
        .find(|u| u.as_str() == s)
    }

    pub fn higher_is_better(self) -> bool {
        self == Unit::MBps
    }
}

/// One benchmark result.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub name: String,
    pub unit: Unit,
    /// From the median sample (the recorded number).
    pub value: f64,
    /// From the best sample (for throughput: the fastest), for reference.
    pub best: f64,
    pub samples: usize,
}

#[derive(Clone)]
pub struct Config {
    /// Run each measurement once with no warmup (smoke test).
    pub quick: bool,
    /// Sampling stops after this much time (and at least `min_samples`).
    pub budget: Duration,
    pub min_samples: usize,
    pub max_samples: usize,
    /// Only results whose name contains one of these substrings (all if empty).
    pub filters: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            quick: false,
            budget: Duration::from_secs(1),
            min_samples: 5,
            max_samples: 50,
            filters: Vec::new(),
        }
    }
}

/// Called with each result as it is measured.
pub type ResultHook = Box<dyn FnMut(&Measurement)>;

/// Collects results. Benchmark functions call `ops`, `bytes`, `time` or `value`; each
/// does nothing unless its name passes the filters.
pub struct Runner {
    pub config: Config,
    pub results: Vec<Measurement>,
    /// Benchmarks that could not run, with the reason.
    pub skipped: Vec<(String, String)>,
    /// Called after each result (progress output).
    pub on_result: Option<ResultHook>,
}

impl Runner {
    pub fn new(config: Config) -> Runner {
        Runner {
            config,
            results: Vec::new(),
            skipped: Vec::new(),
            on_result: None,
        }
    }

    pub fn wants(&self, name: &str) -> bool {
        self.config.filters.is_empty()
            || self
                .config
                .filters
                .iter()
                .any(|f| name.contains(f.as_str()))
    }

    /// Scale for input sizes: 1.0 normally, small in quick mode.
    pub fn scale(&self, full: usize, quick: usize) -> usize {
        if self.config.quick { quick } else { full }
    }

    pub fn skip(&mut self, name: &str, why: impl Into<String>) {
        if self.wants(name) {
            self.skipped.push((name.to_string(), why.into()));
        }
    }

    /// Times `f`, which performs `ops` operations per call: reports ns/op.
    pub fn ops(&mut self, name: &str, ops: usize, f: impl FnMut()) {
        if let Some(samples) = self.sample(name, || {}, f) {
            let per = |d: Duration| d.as_nanos() as f64 / ops as f64;
            self.push(
                name,
                Unit::NsPerOp,
                per(median(&samples)),
                per(samples[0]),
                samples.len(),
            );
        }
    }

    /// Times `f`, which processes `bytes` bytes per call: reports MB/s.
    pub fn bytes(&mut self, name: &str, bytes: usize, f: impl FnMut()) {
        self.bytes_with_setup(name, bytes, || {}, f)
    }

    /// Like `bytes`, with `setup` run untimed before every call of `f` (for example a
    /// GC, so that each call starts from the same heap).
    pub fn bytes_with_setup(
        &mut self,
        name: &str,
        bytes: usize,
        setup: impl FnMut(),
        f: impl FnMut(),
    ) {
        if let Some(samples) = self.sample(name, setup, f) {
            let rate = |d: Duration| bytes as f64 / d.as_secs_f64().max(1e-12) / 1e6;
            self.push(
                name,
                Unit::MBps,
                rate(median(&samples)),
                rate(samples[0]),
                samples.len(),
            );
        }
    }

    /// Times `f` per call: reports ms, optionally divided by `per` (e.g. MiB live).
    pub fn time(&mut self, name: &str, unit: Unit, per: f64, f: impl FnMut()) {
        if let Some(samples) = self.sample(name, || {}, f) {
            let ms = |d: Duration| d.as_secs_f64() * 1e3 / per;
            self.push(
                name,
                unit,
                ms(median(&samples)),
                ms(samples[0]),
                samples.len(),
            );
        }
    }

    /// Records a value that is not a timing (sizes), or a timing measured elsewhere.
    pub fn value(&mut self, name: &str, unit: Unit, value: f64) {
        if self.wants(name) {
            self.push(name, unit, value, value, 1);
        }
    }

    /// Like `time`, for callers that measure each sample themselves (subprocesses):
    /// `f` returns the duration of one run, or an error that skips the benchmark.
    pub fn time_with(&mut self, name: &str, mut f: impl FnMut() -> Result<Duration, String>) {
        if !self.wants(name) {
            return;
        }
        let mut samples = Vec::new();
        let start = Instant::now();
        let warmup = if self.config.quick { 0 } else { 1 };
        for i in 0.. {
            let d = match f() {
                Ok(d) => d,
                Err(e) => {
                    self.skipped.push((name.to_string(), e));
                    return;
                }
            };
            if i >= warmup {
                samples.push(d);
            }
            if self.enough(samples.len(), start.elapsed()) {
                break;
            }
        }
        samples.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        self.push(
            name,
            Unit::Ms,
            ms(median(&samples)),
            ms(samples[0]),
            samples.len(),
        );
    }

    fn enough(&self, n: usize, elapsed: Duration) -> bool {
        if self.config.quick {
            return n >= 1;
        }
        n >= self.config.max_samples
            || (n >= self.config.min_samples && elapsed >= self.config.budget)
    }

    /// Warmup, then samples until the budget is used; sorted ascending.
    fn sample(
        &self,
        name: &str,
        mut setup: impl FnMut(),
        mut f: impl FnMut(),
    ) -> Option<Vec<Duration>> {
        if !self.wants(name) {
            return None;
        }
        if !self.config.quick {
            // Warm caches and the allocator: at least two calls or 100 ms.
            let t = Instant::now();
            let mut n = 0;
            while n < 2 || t.elapsed() < Duration::from_millis(100) {
                setup();
                f();
                n += 1;
                if n >= self.config.max_samples {
                    break;
                }
            }
        }
        let mut samples = Vec::new();
        let start = Instant::now();
        loop {
            setup();
            let t = Instant::now();
            f();
            samples.push(t.elapsed());
            if self.enough(samples.len(), start.elapsed()) {
                break;
            }
        }
        samples.sort();
        Some(samples)
    }

    fn push(&mut self, name: &str, unit: Unit, value: f64, best: f64, samples: usize) {
        let m = Measurement {
            name: name.to_string(),
            unit,
            value,
            best,
            samples,
        };
        if let Some(cb) = &mut self.on_result {
            cb(&m);
        }
        self.results.push(m);
    }
}

fn median(sorted: &[Duration]) -> Duration {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medians_and_units() {
        let d = Duration::from_millis;
        assert_eq!(median(&[d(1), d(2), d(9)]), d(2));
        assert_eq!(median(&[d(1), d(3)]), d(2));
        for u in [
            Unit::NsPerOp,
            Unit::MBps,
            Unit::Ms,
            Unit::MsPerMiB,
            Unit::Bytes,
        ] {
            assert_eq!(Unit::parse(u.as_str()), Some(u));
        }
        assert!(Unit::MBps.higher_is_better() && !Unit::Ms.higher_is_better());
    }

    #[test]
    fn filters_and_quick() {
        let mut r = Runner::new(Config {
            quick: true,
            filters: vec!["num/".into()],
            ..Config::default()
        });
        let mut calls = 0;
        r.ops("num/a", 10, || calls += 1);
        r.ops("lex/b", 10, || panic!("filtered out"));
        r.bytes("num/c", 1_000_000, || {});
        r.value("num/size", Unit::Bytes, 5.0);
        assert_eq!(calls, 1);
        assert_eq!(r.results.len(), 3);
        assert_eq!(r.results[0].samples, 1);
        assert_eq!(r.results[2].value, 5.0);
    }
}
