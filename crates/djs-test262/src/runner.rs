//! Runs selected tests on an engine, in parallel.

use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use crate::engine::{Engine, Job, RunResult, Status, Verdict, judge};
use crate::par;
use crate::suite::{Suite, TestCase};

pub struct Options {
    pub jobs: usize,
    pub timeout: Duration,
    pub memory_limit: Option<u64>,
    /// Show a progress line on stderr (only when it is a terminal).
    pub progress: bool,
}

pub struct Outcome {
    pub verdict: Verdict,
    pub elapsed: Duration,
}

/// Runs one test and judges the result.
pub fn run_one(engine: &dyn Engine, suite: &Suite, test: &TestCase, opts: &Options) -> Outcome {
    let start = Instant::now();
    let verdict = match prepare_and_run(engine, suite, test, opts) {
        // Engines that meter fuel say which limit it was; subprocess engines only have
        // the wall clock.
        Ok(result) if result.status == Status::Timeout => {
            Verdict::Fail(if result.diagnostics.is_empty() {
                format!(
                    "timeout: no result within {:.1}s",
                    opts.timeout.as_secs_f64()
                )
            } else {
                format!("timeout: {}", result.diagnostics)
            })
        }
        Ok(result) => judge(&test.meta, &result),
        Err(e) => Verdict::Fail(e),
    };
    Outcome {
        verdict,
        elapsed: start.elapsed(),
    }
}

fn prepare_and_run(
    engine: &dyn Engine,
    suite: &Suite,
    test: &TestCase,
    opts: &Options,
) -> Result<RunResult, String> {
    if let Some(e) = &test.error {
        return Err(e.clone());
    }
    let file = suite.test_dir().join(&test.path);
    let source = std::fs::read_to_string(&file).map_err(|e| format!("cannot read: {e}"))?;
    let raw = test.meta.has_flag("raw");
    let harness = if raw {
        Vec::new()
    } else {
        suite.harness_for(&test.meta)?
    };
    let job = Job {
        path: &test.path,
        file: &file,
        source: &source,
        harness,
        strict: !(raw || test.meta.has_flag("noStrict")),
        is_async: test.meta.has_flag("async"),
        timeout: opts.timeout,
        memory_limit: opts.memory_limit,
    };
    // An engine bug must fail one test, not abort the whole run (worker panics would
    // propagate out of the thread scope).
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.run(&job)));
    Ok(result.unwrap_or_else(|panic| {
        let msg = panic
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        RunResult::new(Status::Crash(format!("engine panicked: {msg}")))
    }))
}

/// Runs `tests` and returns their outcomes in the same order.
pub fn run_all(
    engine: &dyn Engine,
    suite: &Suite,
    tests: &[&TestCase],
    opts: &Options,
) -> Vec<Outcome> {
    let total = tests.len();
    let show = opts.progress && std::io::stderr().is_terminal();
    let mut failed = 0;
    let mut last = Instant::now();
    let outcomes = par::map_with_progress(
        tests.to_vec(),
        opts.jobs,
        |t| run_one(engine, suite, t, opts),
        |done, o: &Outcome| {
            if matches!(o.verdict, Verdict::Fail(_)) {
                failed += 1;
            }
            if show && (done == total || last.elapsed() >= Duration::from_millis(200)) {
                last = Instant::now();
                eprint!("\r\x1b[K[{done}/{total}] {failed} failed");
                let _ = std::io::stderr().flush();
            }
        },
    );
    if show {
        eprint!("\r\x1b[K");
    }
    outcomes
}
