//! The engine abstraction and the shared judgement of test outcomes.
//!
//! An engine only reports what happened when it ran a job (which phase threw what,
//! what was printed). Deciding pass or fail from that, including negative and async
//! tests, is shared code, so every engine is judged by the same rules.

use std::path::Path;
use std::time::Duration;

use crate::frontmatter::{Meta, Phase};
use crate::suite::HarnessFile;

/// One test execution, prepared by the runner.
pub struct Job<'a> {
    /// Path relative to `test/` (for messages).
    pub path: &'a str,
    /// Absolute path of the test file.
    pub file: &'a Path,
    /// The test source, unmodified.
    pub source: &'a str,
    /// Harness files to evaluate first, in order, each as its own Script in the same
    /// realm. Empty for `raw` tests.
    pub harness: Vec<&'a HarnessFile>,
    /// Whether the test and harness are to run as strict code. Engines that are
    /// always strict (djs) ignore this; others prepend `"use strict";` when it is set.
    /// It is false only for `raw` (and, when explicitly selected, `noStrict`) tests.
    pub strict: bool,
    /// The test has the `async` flag: it signals completion via `print`.
    pub is_async: bool,
    /// Wall-clock limit for the whole job.
    pub timeout: Duration,
    /// Memory limit in bytes, if any.
    pub memory_limit: Option<u64>,
}

/// An uncaught exception, described by the thrown value's constructor name (as test262
/// negative tests specify it) and its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    pub name: String,
    pub message: String,
}

impl std::fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            f.write_str(&self.name)
        } else {
            write!(f, "{}: {}", self.name, self.message)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The test script was evaluated without an uncaught exception, and the job queue
    /// ran until it was empty.
    Completed,
    /// A harness file threw (at parse or run time).
    HarnessError { file: String, error: ErrorInfo },
    /// The test source failed to parse (including early errors). Nothing ran.
    ParseError(ErrorInfo),
    /// The test script threw during evaluation.
    RuntimeError(ErrorInfo),
    /// The wall-clock or fuel limit was hit.
    Timeout,
    /// The memory limit was hit.
    OutOfMemory,
    /// The engine failed in some other way (crash, protocol error, ...).
    Crash(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub status: Status,
    /// Lines written with `print` (including the async test markers).
    pub printed: Vec<String>,
    /// Extra engine diagnostics (e.g. a subprocess' stderr), for failure reasons.
    pub diagnostics: String,
}

impl RunResult {
    pub fn new(status: Status) -> RunResult {
        RunResult {
            status,
            printed: Vec::new(),
            diagnostics: String::new(),
        }
    }
}

pub trait Engine: Sync {
    fn name(&self) -> &str;
    fn run(&self, job: &Job<'_>) -> RunResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail(String),
}

const ASYNC_COMPLETE: &str = "Test262:AsyncTestComplete";
const ASYNC_FAILURE: &str = "Test262:AsyncTestFailure";

/// Decides whether a run passed, following test262's INTERPRETING.md.
pub fn judge(meta: &Meta, result: &RunResult) -> Verdict {
    let fail = |s: String| Verdict::Fail(s);
    let got_error = |phase: Phase, e: &ErrorInfo| -> Verdict {
        match &meta.negative {
            Some(neg) if neg.phase == phase && neg.error_type == e.name => Verdict::Pass,
            Some(neg) => fail(format!(
                "expected {} {}, got {phase} {e}",
                neg.phase, neg.error_type
            )),
            None if phase == Phase::Parse => fail(format!("parse: {e}")),
            None => fail(e.to_string()),
        }
    };
    match &result.status {
        Status::HarnessError { file, error } => fail(format!("harness {file}: {error}")),
        Status::ParseError(e) => got_error(Phase::Parse, e),
        Status::RuntimeError(e) => got_error(Phase::Runtime, e),
        Status::Timeout => fail("timeout".into()),
        Status::OutOfMemory => fail("oom".into()),
        Status::Crash(msg) => fail(format!("crash: {msg}")),
        Status::Completed => {
            if let Some(neg) = &meta.negative {
                return fail(format!(
                    "expected {} {}, but no error was thrown",
                    neg.phase, neg.error_type
                ));
            }
            if !meta.has_flag("async") {
                return Verdict::Pass;
            }
            if let Some(line) = result.printed.iter().find(|l| l.starts_with(ASYNC_FAILURE)) {
                let msg = line[ASYNC_FAILURE.len()..].trim_start_matches(':');
                return fail(format!("async: {msg}"));
            }
            match result
                .printed
                .iter()
                .filter(|l| *l == ASYNC_COMPLETE)
                .count()
            {
                1 => Verdict::Pass,
                0 => {
                    let mut msg = "async: $DONE was not called".to_string();
                    if let Some(first) = result.diagnostics.lines().find(|l| !l.trim().is_empty()) {
                        msg.push_str(" (");
                        msg.push_str(first.trim());
                        msg.push(')');
                    }
                    fail(msg)
                }
                n => fail(format!("async: $DONE was called {n} times")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::Negative;

    fn meta(flags: &[&str], negative: Option<(Phase, &str)>) -> Meta {
        Meta {
            flags: flags.iter().map(|s| s.to_string()).collect(),
            negative: negative.map(|(phase, t)| Negative {
                phase,
                error_type: t.into(),
            }),
            ..Meta::default()
        }
    }

    fn err(name: &str) -> ErrorInfo {
        ErrorInfo {
            name: name.into(),
            message: "m".into(),
        }
    }

    fn printed(lines: &[&str]) -> RunResult {
        RunResult {
            printed: lines.iter().map(|s| s.to_string()).collect(),
            ..RunResult::new(Status::Completed)
        }
    }

    #[test]
    fn plain() {
        let m = meta(&[], None);
        assert_eq!(judge(&m, &RunResult::new(Status::Completed)), Verdict::Pass);
        assert_eq!(
            judge(
                &m,
                &RunResult::new(Status::RuntimeError(err("Test262Error")))
            ),
            Verdict::Fail("Test262Error: m".into())
        );
        assert_eq!(
            judge(&m, &RunResult::new(Status::ParseError(err("SyntaxError")))),
            Verdict::Fail("parse: SyntaxError: m".into())
        );
        assert_eq!(
            judge(&m, &RunResult::new(Status::Timeout)),
            Verdict::Fail("timeout".into())
        );
    }

    #[test]
    fn negative() {
        let m = meta(&[], Some((Phase::Parse, "SyntaxError")));
        assert_eq!(
            judge(&m, &RunResult::new(Status::ParseError(err("SyntaxError")))),
            Verdict::Pass
        );
        // Right type, wrong phase.
        assert!(matches!(
            judge(
                &m,
                &RunResult::new(Status::RuntimeError(err("SyntaxError")))
            ),
            Verdict::Fail(_)
        ));
        assert!(matches!(
            judge(&m, &RunResult::new(Status::ParseError(err("TypeError")))),
            Verdict::Fail(_)
        ));
        assert!(matches!(
            judge(&m, &RunResult::new(Status::Completed)),
            Verdict::Fail(_)
        ));
        let m = meta(&[], Some((Phase::Runtime, "ReferenceError")));
        assert_eq!(
            judge(
                &m,
                &RunResult::new(Status::RuntimeError(err("ReferenceError")))
            ),
            Verdict::Pass
        );
        let harness = Status::HarnessError {
            file: "assert.js".into(),
            error: err("ReferenceError"),
        };
        assert!(matches!(
            judge(&m, &RunResult::new(harness)),
            Verdict::Fail(_)
        ));
    }

    #[test]
    fn asynchronous() {
        let m = meta(&["async"], None);
        assert_eq!(judge(&m, &printed(&["x", ASYNC_COMPLETE])), Verdict::Pass);
        assert_eq!(
            judge(&m, &printed(&[])),
            Verdict::Fail("async: $DONE was not called".into())
        );
        assert_eq!(
            judge(
                &m,
                &printed(&[
                    "Test262:AsyncTestFailure:Test262Error: boom",
                    ASYNC_COMPLETE
                ])
            ),
            Verdict::Fail("async: Test262Error: boom".into())
        );
        assert!(matches!(
            judge(&m, &printed(&[ASYNC_COMPLETE, ASYNC_COMPLETE])),
            Verdict::Fail(_)
        ));
        // Without the async flag, prints do not matter.
        assert_eq!(judge(&meta(&[], None), &printed(&[])), Verdict::Pass);
    }
}
