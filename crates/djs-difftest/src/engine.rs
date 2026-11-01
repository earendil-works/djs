//! The engine abstraction and the transcript that is compared with `.expected`.
//!
//! An engine runs the shared prelude and one case in the codemode model and reports the
//! output items and the result as the prelude produced them. Rendering them into a
//! transcript and normalizing it is shared code, so every engine is compared the same way.

use std::path::Path;
use std::time::Duration;

/// One case execution, prepared by the runner.
pub struct Job<'a> {
    /// Case name relative to the corpus root (for messages).
    pub name: &'a str,
    pub file: &'a Path,
    pub source: &'a str,
    /// Parse the case as TypeScript.
    pub ts: bool,
    pub prelude_file: &'a Path,
    pub prelude: &'a str,
    pub timeout: Duration,
    /// Fuel limit for engines that meter instructions (djs).
    pub fuel: Option<u64>,
    pub memory_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// The script finished without a value to show (undefined, or exit()).
    Completed,
    /// The returned value, as the model sees it.
    Return(String),
    /// The uncaught error, as the model sees it.
    Throw(String),
    Timeout,
    /// The engine failed outside the codemode model (crash, protocol error, ...).
    Crash(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    /// Output items in order (`console.*` and `text()`), each possibly multi-line.
    pub output: Vec<String>,
    pub done: Done,
}

pub trait Engine: Sync {
    fn name(&self) -> &str;
    fn run(&self, job: &Job<'_>) -> RunResult;
}

/// Renders a result as the transcript stored in `.expected`: the output items, then a
/// status line (`=== completed`, `=== return`, `=== throw`, ...) followed by the value or
/// error text. The status is always the last `=== ` line.
pub fn transcript(result: &RunResult) -> String {
    let mut s = String::new();
    for item in &result.output {
        s.push_str(item);
        s.push('\n');
    }
    match &result.done {
        Done::Completed => s.push_str("=== completed\n"),
        Done::Return(v) => {
            s.push_str("=== return\n");
            s.push_str(v);
            s.push('\n');
        }
        Done::Throw(e) => {
            s.push_str("=== throw\n");
            s.push_str(e);
            s.push('\n');
        }
        Done::Timeout => s.push_str("=== timeout\n"),
        Done::Crash(msg) => {
            s.push_str("=== crash\n");
            s.push_str(msg);
            s.push('\n');
        }
    }
    normalize(&s)
}

/// Normalizes a transcript for comparison: stack frame lines (`    at ...`) are dropped,
/// because stack trace formats are not compared yet (D1100), and line endings are `\n`.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.replace("\r\n", "\n").split_inclusive('\n') {
        if line.starts_with("    at ") {
            continue;
        }
        out.push_str(line);
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcripts() {
        let r = RunResult {
            output: vec![
                "a".into(),
                "Error: x\n    at f (codemode.js:1:2)\n    at g".into(),
            ],
            done: Done::Return("{\"v\":1}".into()),
        };
        assert_eq!(transcript(&r), "a\nError: x\n=== return\n{\"v\":1}\n");
        let r = RunResult {
            output: vec![],
            done: Done::Throw("TypeError: no\n    at <eval>".into()),
        };
        assert_eq!(transcript(&r), "=== throw\nTypeError: no\n");
        let r = RunResult {
            output: vec!["".into()],
            done: Done::Completed,
        };
        assert_eq!(transcript(&r), "\n=== completed\n");
    }

    #[test]
    fn normalizing() {
        assert_eq!(normalize("x\r\n    at y\n  at z"), "x\n  at z\n");
        assert_eq!(normalize(""), "");
    }
}
