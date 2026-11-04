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
    /// djs fuel for cases without a `difftest: fuel` directive (the engine's default if
    /// `None`).
    pub fuel: Option<u64>,
    pub memory_limit: Option<u64>,
    pub progress: bool,
    /// Engine is djs: honor `skip-djs`.
    pub is_djs: bool,
    /// Stress variants of the engine (label, engine): each case also runs on them, and
    /// their transcripts must equal the main run's (a hard failure, independent of the
    /// `.expected` comparison).
    pub stress: Vec<(String, Box<dyn Engine>)>,
    /// Each run's time, read to start the slowest runs first and rewritten after the run
    /// (`--times`, 0183).
    pub times: Option<std::path::PathBuf>,
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
    /// djs: the idle points the plain run passed (D2590).
    pub idle_points: Option<u32>,
}

/// The plain run's verdict: its transcript against `.expected`.
fn verdict(
    case: &Case,
    result: &RunResult,
    opts: &Options,
    timeout: Duration,
) -> (Verdict, String) {
    let stacks = case.meta.stacks.then_some(case.name.as_str());
    let actual = transcript(result, stacks);
    let verdict = match fs::read_to_string(&case.expected_file) {
        Ok(expected) => {
            let mut expected = crate::engine::normalize(&expected, stacks);
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
    (verdict, actual)
}

/// A stress variant's run against the plain run's transcript, idle points and image.
fn compare_stress(
    case: &Case,
    label: &str,
    plain: &RunResult,
    actual: &str,
    r: &RunResult,
    stress_timeout: Duration,
) -> Option<StressMismatch> {
    let stacks = case.meta.stacks.then_some(case.name.as_str());
    let t = transcript(r, stacks);
    if t == actual
        && let (Some(a), Some(b)) = (plain.idle_points, r.idle_points)
        && a != b
    {
        // Suspension and collection must not move a tool result to another idle point.
        Some(StressMismatch {
            label: label.to_string(),
            reason: format!("the same transcript, but {b} idle points instead of {a}"),
            diff: String::new(),
        })
    } else if t == actual {
        // `--determinism`: the same heap image, raw and by canonical hash (D1720, D1725,
        // D2040). A variant that collects on another policy (GC stress) does not hash
        // its image.
        if let (Some(a), Some(b)) = (plain.image, r.image)
            && a != b
        {
            let which = if a.1 != b.1 {
                "canonical hash"
            } else {
                "as left (raw)"
            };
            Some(StressMismatch {
                label: label.to_string(),
                reason: format!(
                    "the same transcript, but the heap image {which} differs ({:016x}/{:016x} vs {:016x}/{:016x})",
                    a.0, a.1, b.0, b.1
                ),
                diff: String::new(),
            })
        } else {
            None
        }
    } else {
        let timed_out = matches!(r.done, Done::Timeout) && !matches!(plain.done, Done::Timeout);
        Some(StressMismatch {
            label: label.to_string(),
            reason: if timed_out {
                format!(
                    "timed out after {:.0}s (mark it `difftest: no-stress <reason>` if it \
                     cannot finish under stress)",
                    stress_timeout.as_secs_f64()
                )
            } else {
                diff::first_difference(actual, &t)
            },
            diff: diff::line_diff(actual, &t, 3),
        })
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
                fuel: case.meta.fuel.or(opts.fuel),
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
                        idle_points: None,
                    }
                })
        }
        Err(e) => RunResult {
            output: Vec::new(),
            done: Done::Crash(format!("cannot read case: {e}")),
            image: None,
            idle_points: None,
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
            return format!("throws {}", diff::excerpt(first, 0));
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

/// One unit of work: a case's plain run (`variant: None`) or one of its stress variants.
/// Variants are tasks of their own, so a slow case's five runs spread over the workers
/// instead of running one after another on one of them (0183).
#[derive(Clone, Copy)]
struct Task {
    case: usize,
    variant: Option<usize>,
}

/// The key of a task in a times file: the case and the run's label.
fn task_key(cases: &[Case], opts: &Options, t: Task) -> String {
    let label = t.variant.map_or("plain", |v| opts.stress[v].0.as_str());
    format!("{}\t{label}", cases[t.case].name)
}

/// `secs<TAB>case<TAB>label` lines of an earlier run (missing or unreadable: none).
fn read_times(path: &Path) -> std::collections::HashMap<String, f64> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let (secs, key) = l.split_once('\t')?;
            Some((key.to_string(), secs.parse().ok()?))
        })
        .collect()
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
    let mut tasks = Vec::new();
    let mut remaining = vec![0usize; cases.len()];
    for (i, case) in cases.iter().enumerate() {
        if opts.is_djs && case.meta.skip_djs.is_some() {
            continue;
        }
        tasks.push(Task {
            case: i,
            variant: None,
        });
        if case.meta.no_stress.is_none() {
            tasks.extend((0..opts.stress.len()).map(|v| Task {
                case: i,
                variant: Some(v),
            }));
        }
        remaining[i] = 1 + if case.meta.no_stress.is_none() {
            opts.stress.len()
        } else {
            0
        };
    }
    // Longest first by the last run's times (new tasks first: they may be slow), so the
    // run does not end waiting for one slow task started last.
    if let Some(path) = &opts.times {
        let times = read_times(path);
        let cost = |t: &Task| {
            times
                .get(&task_key(cases, opts, *t))
                .copied()
                .unwrap_or(f64::INFINITY)
        };
        tasks.sort_by(|a, b| cost(b).total_cmp(&cost(a)));
    }
    // One clock reading, shared by every run: a case's stress variants see the time its
    // plain run saw.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let total = cases.len();
    let show = opts.progress && std::io::stderr().is_terminal();
    let mut done = cases.len() - remaining.iter().filter(|&&n| n > 0).count();
    let mut last = Instant::now();
    let results = djs_test262::par::map_with_progress(
        tasks.clone(),
        opts.jobs,
        |t: Task| {
            let case = &cases[t.case];
            let base = case.meta.timeout.unwrap_or(opts.timeout);
            let (e, timeout) = match t.variant {
                None => (engine, base),
                Some(v) => (opts.stress[v].1.as_ref(), base * STRESS_TIMEOUT_FACTOR),
            };
            let start = Instant::now();
            let r = execute(e, case, prelude_file, &prelude, opts, now, timeout);
            (t.case, start.elapsed(), r)
        },
        |_, (case, _, _)| {
            remaining[*case] -= 1;
            if remaining[*case] == 0 {
                done += 1;
            }
            if show && (done == total || last.elapsed() >= Duration::from_millis(200)) {
                last = Instant::now();
                eprint!("\r\x1b[K[{done}/{total}]");
                let _ = std::io::stderr().flush();
            }
        },
    );
    if show {
        eprint!("\r\x1b[K");
    }
    if let Some(path) = &opts.times {
        let mut out = String::new();
        for (t, (_, elapsed, _)) in tasks.iter().zip(&results) {
            out.push_str(&format!(
                "{:.3}\t{}\n",
                elapsed.as_secs_f64(),
                task_key(cases, opts, *t)
            ));
        }
        let _ = fs::write(path, out);
    }
    // Back to cases: each plain run with its variants' runs, in variant order.
    let mut runs: Vec<(Option<RunResult>, Vec<Option<RunResult>>, Duration)> = cases
        .iter()
        .map(|_| {
            (
                None,
                (0..opts.stress.len()).map(|_| None).collect(),
                Duration::ZERO,
            )
        })
        .collect();
    for (t, (_, elapsed, r)) in tasks.into_iter().zip(results) {
        let slot = &mut runs[t.case];
        slot.2 += elapsed;
        match t.variant {
            None => slot.0 = Some(r),
            Some(v) => slot.1[v] = Some(r),
        }
    }
    Ok(cases
        .iter()
        .zip(runs)
        .map(|(case, (plain, variants, elapsed))| {
            let Some(plain) = plain else {
                let reason = case.meta.skip_djs.clone().unwrap_or_default();
                return Outcome {
                    verdict: Verdict::Skipped(reason),
                    transcript: String::new(),
                    done: None,
                    elapsed,
                    stress: Vec::new(),
                    idle_points: None,
                };
            };
            if let Some(dir) = crate::djs::PLAIN_OUT.get() {
                // What every plain dump's continuation must end in (scripts/plain-node.mjs).
                let d = dir.join(&case.name);
                let t = crate::engine::transcript(&plain, None);
                let _ = fs::create_dir_all(&d).and_then(|_| fs::write(d.join("transcript"), t));
            }
            let timeout = case.meta.timeout.unwrap_or(opts.timeout);
            let (verdict, actual) = verdict(case, &plain, opts, timeout);
            let stress = variants
                .iter()
                .zip(&opts.stress)
                .filter_map(|(r, (label, _))| {
                    let r = r.as_ref()?;
                    compare_stress(
                        case,
                        label,
                        &plain,
                        &actual,
                        r,
                        timeout * STRESS_TIMEOUT_FACTOR,
                    )
                })
                .collect();
            Outcome {
                verdict,
                transcript: actual,
                done: Some(plain.done),
                elapsed,
                stress,
                idle_points: plain.idle_points,
            }
        })
        .collect())
}
