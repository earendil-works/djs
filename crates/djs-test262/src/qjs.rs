//! The `qjs` engine: dquickts' `qjs` binary as a subprocess, one process per test.
//!
//! A small JS driver (`qjs_driver.js`) provides `$262`, evaluates the harness and the
//! test, and reports the outcome on stdout. See the driver for the protocol.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::engine::{Engine, ErrorInfo, Job, RunResult, Status};

pub const DRIVER: &str = include_str!("qjs_driver.js");
const MARKER: &str = "\u{1}T262 ";

pub struct Qjs {
    pub binary: PathBuf,
}

impl Qjs {
    /// `$DJS_QJS`, else `~/Development/dquickts/qjs`.
    pub fn default_binary() -> PathBuf {
        if let Some(p) = std::env::var_os("DJS_QJS") {
            return PathBuf::from(p);
        }
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home).join("Development/dquickts/qjs")
    }

    fn spec(job: &Job<'_>) -> String {
        let harness: Vec<String> = job
            .harness
            .iter()
            .map(|h| {
                format!(
                    "[{},{}]",
                    json_quote(&h.name),
                    json_quote(&absolute(&h.path).to_string_lossy())
                )
            })
            .collect();
        format!(
            "{{\"harness\":[{}],\"test\":{},\"strict\":{},\"module\":{}}}",
            harness.join(","),
            // Absolute: qjs runs in the test's directory, and imports a module test by
            // this path.
            json_quote(&absolute(job.file).to_string_lossy()),
            job.strict,
            job.module
        )
    }
}

impl Engine for Qjs {
    fn name(&self) -> &str {
        "qjs"
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        let mut cmd = Command::new(&self.binary);
        if let Some(limit) = job.memory_limit {
            cmd.arg("--memory-limit").arg(limit.to_string());
        }
        cmd.args(["-m", "-e", DRIVER]).arg(Self::spec(job));
        if let Some(dir) = job.file.parent() {
            // qjs resolves the specifiers of a Script's `import()`, and a module's that
            // lack a `./`, against the working directory.
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return RunResult::new(Status::Crash(format!(
                    "cannot run {}: {e}",
                    self.binary.display()
                )));
            }
        };
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        // stdout reaching EOF means the process is done; the timeout is measured on that,
        // so no polling is needed.
        let (done_tx, done_rx) = mpsc::channel();
        let out_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf);
            let _ = done_tx.send(());
            buf
        });
        let err_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
            buf
        });
        let timed_out = matches!(
            done_rx.recv_timeout(job.timeout),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        if timed_out {
            let _ = child.kill();
        }
        let exit = child.wait();
        let out = out_thread.join().unwrap_or_default();
        let err = err_thread.join().unwrap_or_default();
        let diagnostics = String::from_utf8_lossy(&err).into_owned();
        if timed_out {
            // The runner words the reason (the wall-clock limit).
            return RunResult::new(Status::Timeout);
        }

        let mut printed = Vec::new();
        let mut status = None;
        for line in String::from_utf8_lossy(&out).lines() {
            match line.strip_prefix(MARKER) {
                Some(rest) if status.is_none() => status = Some(parse_report(rest)),
                _ => printed.push(line.to_string()),
            }
        }
        let signal = exit.as_ref().ok().and_then(exit_signal);
        let status = match (status, signal) {
            (_, Some(sig)) => Status::Crash(format!("killed by signal {sig}")),
            (Some(s), None) => s,
            (None, None) => {
                let code = exit
                    .map(|e| e.to_string())
                    .unwrap_or_else(|e| e.to_string());
                let first = diagnostics
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .trim();
                Status::Crash(format!("no result ({code}) {first}").trim_end().to_string())
            }
        };
        RunResult {
            status,
            printed,
            diagnostics,
        }
    }
}

fn absolute(p: &std::path::Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Parses `<status> "<file>" "<name>" "<message>"` from the driver.
fn parse_report(rest: &str) -> Status {
    let Some((status, mut rest)) = rest.split_once(' ') else {
        return Status::Crash(format!("bad driver report: {rest}"));
    };
    let mut fields = Vec::new();
    while fields.len() < 3 {
        match json_unquote(rest.trim_start()) {
            Some((s, tail)) => {
                fields.push(s);
                rest = tail;
            }
            None => return Status::Crash(format!("bad driver report: {rest}")),
        }
    }
    let message = fields.pop().unwrap();
    let name = fields.pop().unwrap();
    let file = fields.pop().unwrap();
    let error = ErrorInfo { name, message };
    match status {
        "completed" => Status::Completed,
        _ if error.name == "InternalError" && error.message == "out of memory" => {
            Status::OutOfMemory
        }
        "harness" => Status::HarnessError { file, error },
        "parse" => Status::ParseError(error),
        "resolution" => Status::ResolutionError(error),
        "runtime" => Status::RuntimeError(error),
        other => Status::Crash(format!("bad driver status {other}")),
    }
}

fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Decodes a JSON string literal at the start of `s`; returns it and the remainder.
/// Lone surrogates become U+FFFD.
fn json_unquote(s: &str) -> Option<(String, &str)> {
    let body = s.strip_prefix('"')?;
    let mut out = String::new();
    let mut units: Vec<u16> = Vec::new();
    let flush = |units: &mut Vec<u16>, out: &mut String| {
        out.extend(char::decode_utf16(units.drain(..)).map(|r| r.unwrap_or('\u{FFFD}')));
    };
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                flush(&mut units, &mut out);
                return Some((out, &body[i + 1..]));
            }
            '\\' => {
                let (_, e) = chars.next()?;
                let simple = match e {
                    'n' => Some('\n'),
                    't' => Some('\t'),
                    'r' => Some('\r'),
                    'b' => Some('\u{8}'),
                    'f' => Some('\u{c}'),
                    '/' => Some('/'),
                    '"' => Some('"'),
                    '\\' => Some('\\'),
                    'u' => None,
                    _ => return None,
                };
                if let Some(c) = simple {
                    flush(&mut units, &mut out);
                    out.push(c);
                } else {
                    let hex: String = (0..4)
                        .filter_map(|_| chars.next().map(|(_, c)| c))
                        .collect();
                    units.push(u16::from_str_radix(&hex, 16).ok()?);
                }
            }
            c => {
                flush(&mut units, &mut out);
                out.push(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_round_trip() {
        let s = "a\"b\\c\nd\u{1}é😀";
        let q = json_quote(s);
        assert_eq!(json_unquote(&q), Some((s.to_string(), "")));
        assert_eq!(
            json_unquote("\"\\ud83d\\ude00x\" rest"),
            Some(("😀x".to_string(), " rest"))
        );
        assert_eq!(
            json_unquote("\"\\ud83d\""),
            Some(("\u{FFFD}".to_string(), ""))
        );
        assert_eq!(json_unquote("\"open"), None);
    }

    #[test]
    fn reports() {
        assert_eq!(parse_report("completed \"\" \"\" \"\""), Status::Completed);
        assert_eq!(
            parse_report("parse \"\" \"SyntaxError\" \"unexpected token\""),
            Status::ParseError(ErrorInfo {
                name: "SyntaxError".into(),
                message: "unexpected token".into()
            })
        );
        assert_eq!(
            parse_report("harness \"assert.js\" \"TypeError\" \"x\""),
            Status::HarnessError {
                file: "assert.js".into(),
                error: ErrorInfo {
                    name: "TypeError".into(),
                    message: "x".into()
                }
            }
        );
        assert_eq!(
            parse_report("runtime \"\" \"InternalError\" \"out of memory\""),
            Status::OutOfMemory
        );
        assert!(matches!(parse_report("runtime \"\" \"x"), Status::Crash(_)));
    }
}
