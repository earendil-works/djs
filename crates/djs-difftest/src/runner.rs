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
    /// Stress variants of the engine (label, engine): each case also runs on them, and
    /// their transcripts must equal the main run's (a hard failure, independent of the
    /// `.expected` comparison).
    pub stress: Vec<(String, Box<dyn Engine>)>,
}

/// Stressed runs get this many times the case's time limit: a stress mode is several
/// times slower than the plain run, and a timeout must not depend on machine load. A
/// stressed run that still times out is a failure; cases that cannot finish under stress
/// carry `difftest: no-stress` instead (D1530).
pub const STRESS_TIMEOUT_FACTOR: u32 = 3;

/// A stress variant whose transcript differs from the main run's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StressMismatch {
    pub label: String,
    pub reason: String,
    pub diff: String,
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
    pub stress: Vec<StressMismatch>,
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
            stress: Vec::new(),
        };
    }
    // One clock reading per case, shared by the main run and its stress variants.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let timeout = case.meta.timeout.unwrap_or(opts.timeout);
    let result = execute(engine, case, prelude_file, prelude, opts, now, timeout);
    let actual = transcript(&result);
    let verdict = match fs::read_to_string(&case.expected_file) {
        Ok(expected) => {
            let mut expected = crate::engine::normalize(&expected);
            if opts.is_djs && !case.meta.djs_expect.is_empty() {
                expected = expect_djs(&expected, &case.meta.djs_expect);
            }
            if expected == actual {
                Verdict::Pass
            } else if let Some(reason) = engine_failure(&result.done, timeout) {
                // Not a behavior difference: say so instead of pointing at a line.
                Verdict::Fail {
                    reason,
                    diff: diff::line_diff(&expected, &actual, 3),
                }
            } else {
                Verdict::Fail {
                    reason: failure_reason(&expected, &actual, &result.done),
                    diff: diff::line_diff(&expected, &actual, 3),
                }
            }
        }
        Err(_) => Verdict::Fail {
            reason: "no .expected file (run --update)".into(),
            diff: String::new(),
        },
    };
    let mut stress = Vec::new();
    let variants = if case.meta.no_stress.is_some() {
        &[][..]
    } else {
        &opts.stress[..]
    };
    let stress_timeout = timeout * STRESS_TIMEOUT_FACTOR;
    for (label, variant) in variants {
        let r = execute(
            variant.as_ref(),
            case,
            prelude_file,
            prelude,
            opts,
            now,
            stress_timeout,
        );
        let t = transcript(&r);
        if t == actual {
            // `--determinism`: the same graph, so the same heap image after a collection
            // (D1720). A variant that collects on another policy (GC stress) does not
            // hash its image.
            if let (Some(a), Some(b)) = (result.image, r.image)
                && a != b
            {
                stress.push(StressMismatch {
                    label: label.clone(),
                    reason: format!(
                        "the same transcript, but the heap image after a collection differs ({a:016x} vs {b:016x})"
                    ),
                    diff: String::new(),
                });
            }
        } else {
            let timed_out =
                matches!(r.done, Done::Timeout) && !matches!(result.done, Done::Timeout);
            stress.push(StressMismatch {
                label: label.clone(),
                reason: if timed_out {
                    format!(
                        "timed out after {:.0}s (mark it `difftest: no-stress <reason>` if it \
                         cannot finish under stress)",
                        stress_timeout.as_secs_f64()
                    )
                } else {
                    diff::first_difference(&actual, &t)
                },
                diff: diff::line_diff(&actual, &t, 3),
            });
        }
    }
    Outcome {
        verdict,
        transcript: actual,
        done: Some(result.done),
        elapsed: start.elapsed(),
        stress,
    }
}

/// Runs one case on one engine (engine panics become crashes).
fn execute(
    engine: &dyn Engine,
    case: &Case,
    prelude_file: &Path,
    prelude: &str,
    opts: &Options,
    now: f64,
    timeout: Duration,
) -> RunResult {
    match fs::read_to_string(&case.file) {
        Ok(source) => {
            let job = Job {
                name: &case.name,
                file: &case.file,
                source: &source,
                ts: case.meta.ts,
                legacy_decorators: case.meta.legacy_decorators,
                decorator_metadata: case.meta.decorator_metadata,
                prelude_file,
                prelude,
                timeout,
                fuel: case.meta.fuel,
                memory_limit: opts.memory_limit,
                now,
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
                        image: None,
                    }
                })
        }
        Err(e) => RunResult {
            output: Vec::new(),
            done: Done::Crash(format!("cannot read case: {e}")),
            image: None,
        },
    }
}

/// The one-line reason for a transcript difference: the uncaught error when the engine
/// threw one the expectation does not have (that is what needs fixing), else the first
/// differing line.
/// The oracle's transcript with the case's expected differences applied: whole lines
/// equal to an oracle line are replaced by the djs line.
fn expect_djs(expected: &str, rules: &[(String, String)]) -> String {
    let mut out = String::with_capacity(expected.len());
    for line in expected.split_inclusive('\n') {
        let (text, end) = match line.strip_suffix('\n') {
            Some(t) => (t, "\n"),
            None => (line, ""),
        };
        match rules.iter().find(|(a, _)| a == text) {
            Some((_, b)) => out.push_str(b),
            None => out.push_str(text),
        }
        out.push_str(end);
    }
    out
}

fn failure_reason(expected: &str, actual: &str, done: &Done) -> String {
    if let Done::Throw(msg) = done {
        let first = msg.lines().next().unwrap_or("");
        if !expected.lines().any(|l| l == first) {
            return format!("throws {first}");
        }
    }
    diff::first_difference(expected, actual)
}

/// A timeout or crash is reported as such rather than as a transcript difference.
fn engine_failure(done: &Done, timeout: Duration) -> Option<String> {
    match done {
        Done::Timeout => Some(format!(
            "timeout: no result within {:.1}s",
            timeout.as_secs_f64()
        )),
        Done::Crash(msg) => Some(format!("crash: {msg}")),
        _ => None,
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
