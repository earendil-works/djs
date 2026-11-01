//! The `qjs` engine: dquickts' `qjs` as a subprocess, one process per case, driven by
//! `qjs_driver.js` (see there for the protocol).

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::engine::{Done, Engine, Job, RunResult};

const DRIVER: &str = include_str!("qjs_driver.js");
const OUT: &str = "\u{1}OUT ";
const DONE: &str = "\u{1}DONE ";

pub struct Qjs {
    pub binary: PathBuf,
}

impl Qjs {
    /// `$DJS_QJS`, else `~/Development/dquickts/qjs`.
    pub fn default_binary() -> PathBuf {
        djs_test262::qjs::Qjs::default_binary()
    }
}

impl Engine for Qjs {
    fn name(&self) -> &str {
        "qjs"
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        let spec = format!(
            "{{\"prelude\":{},\"case\":{},\"ts\":{}}}",
            json_quote(&job.prelude_file.to_string_lossy()),
            json_quote(&job.file.to_string_lossy()),
            job.ts
        );
        let crash = |msg: String| RunResult {
            output: Vec::new(),
            done: Done::Crash(msg),
        };
        let mut cmd = Command::new(&self.binary);
        if let Some(limit) = job.memory_limit {
            cmd.arg("--memory-limit").arg(limit.to_string());
        }
        cmd.args(["-m", "-e", DRIVER]).arg(spec);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return crash(format!("cannot run {}: {e}", self.binary.display())),
        };
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        // stdout reaching EOF means the process is done; the timeout is measured on that.
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
        parse_output(
            &String::from_utf8_lossy(&out),
            &String::from_utf8_lossy(&err),
            timed_out,
            exit.ok(),
        )
    }
}

fn parse_output(
    out: &str,
    err: &str,
    timed_out: bool,
    exit: Option<std::process::ExitStatus>,
) -> RunResult {
    let mut output = Vec::new();
    let mut done = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix(OUT) {
            match json_unquote(rest) {
                Some((s, "")) => output.push(s),
                _ => done = done.or(Some(Done::Crash(format!("bad driver line: {line}")))),
            }
        } else if let Some(rest) = line.strip_prefix(DONE) {
            let parsed = rest
                .split_once(' ')
                .and_then(|(kind, json)| match json_unquote(json) {
                    Some((text, "")) => match kind {
                        "completed" => Some(Done::Completed),
                        "return" => Some(Done::Return(text)),
                        "throw" => Some(Done::Throw(text)),
                        _ => None,
                    },
                    _ => None,
                });
            done =
                done.or(Some(parsed.unwrap_or_else(|| {
                    Done::Crash(format!("bad driver line: {line}"))
                })));
        } else {
            done = done.or(Some(Done::Crash(format!("unexpected stdout: {line}"))));
        }
    }
    let done = if timed_out {
        Done::Timeout
    } else if let Some(d) = done {
        d
    } else {
        let code = exit
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown exit".into());
        let first = err
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim();
        Done::Crash(format!("no result ({code}) {first}").trim_end().to_string())
    };
    RunResult { output, done }
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
    fn protocol() {
        let out =
            "\u{1}OUT \"a\\nb\"\n\u{1}OUT \"\\ud83d\\ude00\"\n\u{1}DONE return \"{\\\"v\\\":1}\"\n";
        let r = parse_output(out, "", false, None);
        assert_eq!(r.output, ["a\nb", "😀"]);
        assert_eq!(r.done, Done::Return("{\"v\":1}".into()));
        let r = parse_output("\u{1}DONE completed \"\"\n", "", false, None);
        assert_eq!(r.done, Done::Completed);
        assert!(
            matches!(parse_output("", "boom\n", false, None).done, Done::Crash(m) if m.contains("boom"))
        );
        assert!(matches!(
            parse_output("stray\n", "", false, None).done,
            Done::Crash(_)
        ));
        assert_eq!(
            parse_output("\u{1}OUT \"x\"\n", "", true, None).done,
            Done::Timeout
        );
        assert!(matches!(
            parse_output("\u{1}DONE weird \"\"\n", "", false, None).done,
            Done::Crash(_)
        ));
    }

    #[test]
    fn quoting() {
        let s = "a\"b\\c\nd\u{1}é";
        assert_eq!(json_unquote(&json_quote(s)), Some((s.to_string(), "")));
    }
}
