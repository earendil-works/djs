//! Runs cases on an engine in parallel and compares the transcripts with `.expected`.

use std::fs;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::case::Case;
use crate::diff;
use crate::engine::{Done, Engine, Job, RunResult, transcript};

pub struct Options {
    pub jobs: usize,
    pub timeout: Duration,
    pub memory_limit: Option<u64>,
    pub progress: bool,
    /// Engine is djs: honor `skip-djs`.
    pub is_djs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// The transcript differs from `.expected` (or `.expected` is missing).
    Fail {
        reason: String,
        diff: String,
    },
    Skipped(String),
}

pub struct Outcome {
    pub verdict: Verdict,
    /// The normalized transcript (empty for skipped cases).
    pub transcript: String,
    pub done: Option<Done>,
    pub elapsed: Duration,
}

pub fn run_one(
    engine: &dyn Engine,
    case: &Case,
    prelude_file: &Path,
    prelude: &str,
    opts: &Options,
) -> Outcome {
    let start = Instant::now();
    if opts.is_djs
        && let Some(reason) = &case.meta.skip_djs
    {
        return Outcome {
            verdict: Verdict::Skipped(reason.clone()),
            transcript: String::new(),
            done: None,
            elapsed: start.elapsed(),
        };
    }
    let result = match fs::read_to_string(&case.file) {
        Ok(source) => {
            let job = Job {
                name: &case.name,
                file: &case.file,
                source: &source,
                ts: case.meta.ts,
                prelude_file,
                prelude,
                timeout: case.meta.timeout.unwrap_or(opts.timeout),
                fuel: case.meta.fuel,
                memory_limit: opts.memory_limit,
            };
            // An engine bug must fail one case, not abort the run.
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.run(&job)))
                .unwrap_or_else(|panic| {
                    let msg = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    RunResult {
                        output: Vec::new(),
                        done: Done::Crash(format!("engine panicked: {msg}")),
                    }
                })
        }
        Err(e) => RunResult {
            output: Vec::new(),
            done: Done::Crash(format!("cannot read case: {e}")),
        },
    };
    let actual = transcript(&result);
    let verdict = match fs::read_to_string(&case.expected_file) {
        Ok(expected) => {
            let expected = crate::engine::normalize(&expected);
            if expected == actual {
                Verdict::Pass
            } else {
                Verdict::Fail {
                    reason: diff::first_difference(&expected, &actual),
                    diff: diff::line_diff(&expected, &actual, 3),
                }
            }
        }
        Err(_) => Verdict::Fail {
            reason: "no .expected file (run --update)".into(),
            diff: String::new(),
        },
    };
    Outcome {
        verdict,
        transcript: actual,
        done: Some(result.done),
        elapsed: start.elapsed(),
    }
}

/// Runs `cases` and returns their outcomes in the same order.
pub fn run_all(
    engine: &dyn Engine,
    cases: &[Case],
    prelude_file: &Path,
    opts: &Options,
) -> Result<Vec<Outcome>, String> {
    let prelude =
        fs::read_to_string(prelude_file).map_err(|e| format!("{}: {e}", prelude_file.display()))?;
    let total = cases.len();
    let show = opts.progress && std::io::stderr().is_terminal();
    let mut failed = 0;
    let mut last = Instant::now();
    let outcomes = djs_test262::par::map_with_progress(
        cases.iter().collect(),
        opts.jobs,
        |c: &Case| run_one(engine, c, prelude_file, &prelude, opts),
        |done, o: &Outcome| {
            if matches!(o.verdict, Verdict::Fail { .. }) {
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
    Ok(outcomes)
}
