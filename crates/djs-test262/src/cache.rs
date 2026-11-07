//! A result cache for the qjs engine (D1531).
//!
//! qjs is the oracle: for a given qjs binary, test262 commit and driver, a test always
//! gives the same result, and re-running ~30k subprocesses per check costs ~120 CPU-s.
//! The cache keeps each test's `RunResult` in one file per (qjs binary hash, test262
//! commit, driver hash, memory limit), next to the test262 checkout, which all worktrees
//! share. Timeouts and crashes are not cached: they can depend on machine load.
//!
//! The file is a text file with one entry per line: the job key, the status and the
//! printed lines, tab-separated and escaped. Concurrent writers lose entries, never
//! correctness: whoever writes last replaces the file (atomically, via a rename).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::engine::{Engine, ErrorInfo, Job, RunResult, Status};

/// The format version: part of the file name, so a format change starts a new file.
const VERSION: u32 = 1;

pub struct QjsCache {
    file: PathBuf,
    entries: Mutex<HashMap<String, RunResult>>,
    hits: AtomicUsize,
    added: AtomicUsize,
}

impl QjsCache {
    /// Opens (or starts) the cache file in `dir` for this configuration. `read` false
    /// ignores the existing entries (they are replaced as tests run).
    pub fn open(
        dir: &Path,
        qjs_binary: &Path,
        test262_commit: &str,
        driver: &str,
        memory_limit: Option<u64>,
        read: bool,
    ) -> Result<QjsCache, String> {
        let binary =
            std::fs::read(qjs_binary).map_err(|e| format!("{}: {e}", qjs_binary.display()))?;
        let mut h = fnv(FNV_INIT, &binary);
        h = fnv(h, test262_commit.as_bytes());
        h = fnv(h, driver.as_bytes());
        h = fnv(h, &memory_limit.unwrap_or(0).to_le_bytes());
        let file = dir.join(format!("test262-v{VERSION}-{h:016x}.tsv"));
        let mut entries = HashMap::new();
        if read && let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines().filter(|l| !l.starts_with('#')) {
                if let Some((key, result)) = decode_line(line) {
                    entries.insert(key, result);
                }
            }
        }
        Ok(QjsCache {
            file,
            entries: Mutex::new(entries),
            hits: AtomicUsize::new(0),
            added: AtomicUsize::new(0),
        })
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }

    /// Writes the file if this run added entries.
    pub fn save(&self) -> Result<(), String> {
        if self.added.load(Ordering::Relaxed) == 0 {
            return Ok(());
        }
        let entries = self.entries.lock().unwrap();
        let mut keys: Vec<&String> = entries.keys().collect();
        keys.sort();
        let mut text = String::from(
            "# djs-test262 qjs result cache (D1531): key, status, printed lines. Safe to delete.\n",
        );
        for k in keys {
            text.push_str(&encode_line(k, &entries[k]));
            text.push('\n');
        }
        let dir = self.file.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = self
            .file
            .with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.file).map_err(|e| format!("{}: {e}", self.file.display()))
    }
}

/// An engine that answers from the cache and runs `inner` (qjs) on a miss.
pub struct Cached<'a> {
    pub inner: &'a dyn Engine,
    pub cache: &'a QjsCache,
}

impl Engine for Cached<'_> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn run(&self, job: &Job<'_>) -> RunResult {
        let key = job_key(job);
        if let Some(r) = self.cache.entries.lock().unwrap().get(&key) {
            self.cache.hits.fetch_add(1, Ordering::Relaxed);
            return r.clone();
        }
        let result = self.inner.run(job);
        if !matches!(result.status, Status::Timeout | Status::Crash(_)) {
            self.cache
                .entries
                .lock()
                .unwrap()
                .insert(key, result.clone());
            self.cache.added.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}

/// What determines a job's result besides the cache file's configuration: the test,
/// strictness and the harness files in order.
fn job_key(job: &Job<'_>) -> String {
    let harness: Vec<&str> = job.harness.iter().map(|h| h.name.as_str()).collect();
    let mode = match (job.module, job.strict) {
        (true, _) => "module",
        (false, true) => "strict",
        (false, false) => "sloppy",
    };
    format!("{}|{mode}|{}", job.path, harness.join(","))
}

const FNV_INIT: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// One line: key, status, printed lines, diagnostics, separated by tabs. The status
/// parts are separated by U+001F and every printed line ends in U+001E; all pieces are
/// escaped, so the separators only occur as separators.
fn encode_line(key: &str, r: &RunResult) -> String {
    let status: Vec<&str> = match &r.status {
        Status::Completed => vec!["completed"],
        Status::OutOfMemory => vec!["oom"],
        Status::HarnessError { file, error } => vec!["harness", file, &error.name, &error.message],
        Status::ParseError(e) => vec!["parse", &e.name, &e.message],
        Status::ResolutionError(e) => vec!["resolution", &e.name, &e.message],
        Status::RuntimeError(e) => vec!["runtime", &e.name, &e.message],
        // Never stored (see `Cached::run`); decoding rejects them.
        Status::Timeout => vec!["timeout"],
        Status::Crash(m) => vec!["crash", m],
    };
    let status: Vec<String> = status.into_iter().map(escape).collect();
    let printed: String = r.printed.iter().map(|l| escape(l) + "\u{1e}").collect();
    format!(
        "{}\t{}\t{}\t{}",
        escape(key),
        status.join("\u{1f}"),
        printed,
        escape(&r.diagnostics)
    )
}

fn decode_line(line: &str) -> Option<(String, RunResult)> {
    let mut fields = line.split('\t');
    let (key, status, printed, diagnostics) = (
        fields.next()?,
        fields.next()?,
        fields.next()?,
        fields.next()?,
    );
    let parts: Vec<String> = status.split('\u{1f}').map(unescape).collect();
    let part = |i: usize| parts.get(i).cloned();
    let err = |i: usize| -> Option<ErrorInfo> {
        Some(ErrorInfo {
            name: part(i)?,
            message: part(i + 1)?,
        })
    };
    let status = match parts[0].as_str() {
        "completed" => Status::Completed,
        "oom" => Status::OutOfMemory,
        "harness" => Status::HarnessError {
            file: part(1)?,
            error: err(2)?,
        },
        "parse" => Status::ParseError(err(1)?),
        "resolution" => Status::ResolutionError(err(1)?),
        "runtime" => Status::RuntimeError(err(1)?),
        _ => return None,
    };
    let printed = printed.split_terminator('\u{1e}').map(unescape).collect();
    Some((
        unescape(key),
        RunResult {
            status,
            printed,
            diagnostics: unescape(diagnostics),
        },
    ))
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{1e}' => out.push_str("\\R"),
            '\u{1f}' => out.push_str("\\U"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('R') => out.push('\u{1e}'),
            Some('U') => out.push('\u{1f}'),
            Some(c) => out.push(c),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(status: Status, printed: &[&str], diagnostics: &str) -> RunResult {
        RunResult {
            status,
            printed: printed.iter().map(|s| s.to_string()).collect(),
            diagnostics: diagnostics.into(),
        }
    }

    #[test]
    fn lines_round_trip() {
        let e = |n: &str, m: &str| ErrorInfo {
            name: n.into(),
            message: m.into(),
        };
        let cases = [
            result(Status::Completed, &[], ""),
            result(
                Status::Completed,
                &["a\tb", "", "c\\n\u{1e}"],
                "stderr\nline 2\r",
            ),
            result(Status::Completed, &[""], ""),
            result(Status::OutOfMemory, &[], ""),
            result(
                Status::HarnessError {
                    file: "assert.js".into(),
                    error: e("SyntaxError", "x"),
                },
                &[],
                "",
            ),
            result(Status::ParseError(e("SyntaxError", "")), &[], ""),
            result(Status::ResolutionError(e("SyntaxError", "m")), &[], ""),
            result(
                Status::RuntimeError(e("Test262Error", "1\n2\u{1f}")),
                &["p"],
                "",
            ),
        ];
        for r in cases {
            let line = encode_line("built-ins/x.js|strict|assert.js,sta.js", &r);
            assert!(!line.contains('\n'));
            let (k, back) = decode_line(&line).unwrap();
            assert_eq!(k, "built-ins/x.js|strict|assert.js,sta.js");
            assert_eq!(back, r);
        }
        assert!(decode_line("k\tbogus\t\t").is_none());
        assert!(decode_line("k\tcompleted").is_none());
    }
}
