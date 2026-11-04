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
    /// TypeScript legacy decorators, and their metadata.
    pub legacy_decorators: bool,
    pub decorator_metadata: bool,
    pub prelude_file: &'a Path,
    pub prelude: &'a str,
    pub timeout: Duration,
    /// Fuel limit for engines that meter instructions (djs).
    pub fuel: Option<u64>,
    pub memory_limit: Option<u64>,
    /// The clock the case sees (ms since the epoch): read once per case, so the plain run
    /// and its stress variants see the same time (and leave comparable heap images).
    pub now: f64,
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
    /// djs with `--determinism`: hashes of the final heap image, as left and after a
    /// collection (D1720, D1725).
    pub image: Option<(u64, u64)>,
    /// djs: the idle points the run passed, i.e. the `host.later` calls it delivered
    /// (D2590). None for engines without them.
    pub idle_points: Option<u32>,
}

pub trait Engine: Sync {
    fn name(&self) -> &str;
    fn run(&self, job: &Job<'_>) -> RunResult;
    /// A stress variant that is compared only for cases whose plain run passes an idle
    /// point: the plain-dump variant, which runs with frozen builtins (D2836), so a case
    /// that changes builtins on purpose and takes no dump would differ for that alone.
    fn only_with_idle_points(&self) -> bool {
        false
    }
}

/// Renders a result as the transcript stored in `.expected`: the output items, then a
/// status line (`=== completed`, `=== return`, `=== throw`, ...) followed by the value or
/// error text. The status is always the last `=== ` line.
///
/// `stacks`: the case's file name with `difftest: stacks`, which keeps its frame lines
/// (see [`normalize`]).
pub fn transcript(result: &RunResult, stacks: Option<&str>) -> String {
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
    normalize(&s, stacks)
}

/// Normalizes a transcript for comparison: line endings are `\n`, and stack frame lines
/// (`    at ...`) are dropped (D1100), except with `stacks` (the case's file name, for
/// cases marked `difftest: stacks`, D2032). Then a frame in the case, which djs names
/// by the case's file and qjs `<evalScript>`, reads `(case:line:col)`, a `(native)`
/// frame stays, and frames of the harness (the prelude, qjs's driver) are dropped:
/// they differ by engine.
pub fn normalize(text: &str, stacks: Option<&str>) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.replace("\r\n", "\n").split_inclusive('\n') {
        if let Some(frame) = line.strip_prefix("    at ") {
            let Some(case) = stacks else {
                continue;
            };
            match case_frame(frame, case) {
                Some(f) => {
                    out.push_str("    at ");
                    out.push_str(&f);
                    if line.ends_with('\n') {
                        out.push('\n');
                    }
                }
                None => continue,
            }
            continue;
        }
        out.push_str(line);
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// A frame line (after `    at `) as `normalize` keeps it, or `None` for a harness
/// frame.
fn case_frame(frame: &str, case: &str) -> Option<String> {
    let frame = frame.trim_end_matches('\n');
    let Some((name, loc)) = frame.rsplit_once(" (") else {
        return Some(frame.to_string());
    };
    let loc = loc.strip_suffix(')')?;
    if loc == "native" {
        return Some(frame.to_string());
    }
    // `file:line:col`; the file may contain `:` (qjs's driver frames are `<cmdline>`).
    let mut parts = loc.rsplitn(3, ':');
    let (col, line, file) = (parts.next()?, parts.next()?, parts.next()?);
    if file == case || file == "<evalScript>" || file == "case" {
        Some(format!("{name} (case:{line}:{col})"))
    } else {
        None
    }
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
            image: None,
            idle_points: None,
        };
        assert_eq!(transcript(&r, None), "a\nError: x\n=== return\n{\"v\":1}\n");
        let r = RunResult {
            output: vec![],
            done: Done::Throw("TypeError: no\n    at <eval>".into()),
            image: None,
            idle_points: None,
        };
        assert_eq!(transcript(&r, None), "=== throw\nTypeError: no\n");
        let r = RunResult {
            output: vec!["".into()],
            done: Done::Completed,
            image: None,
            idle_points: None,
        };
        assert_eq!(transcript(&r, None), "\n=== completed\n");
    }

    #[test]
    fn normalizing() {
        assert_eq!(normalize("x\r\n    at y\n  at z", None), "x\n  at z\n");
        assert_eq!(normalize("", None), "");
        let qjs = "    at f (<evalScript>:3:8)\n    at map (native)\n    at run (/c/prelude.js:181:21)\n    at <anonymous> (<cmdline>:64:30)\n";
        let djs = "    at f (a/b.js:3:8)\n    at map (native)\n    at run (prelude.js:181:21)\n";
        let want = "    at f (case:3:8)\n    at map (native)\n";
        assert_eq!(normalize(qjs, Some("a/b.js")), want);
        assert_eq!(normalize(djs, Some("a/b.js")), want);
        assert_eq!(normalize(want, Some("a/b.js")), want);
    }
}
