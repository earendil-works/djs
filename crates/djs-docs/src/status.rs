//! Status numbers for `gen:status` regions (D2101): counts that follow from the
//! repository are computed here (test262 groups and ratchets, corpus cases, the base
//! snapshot, builtin tags, engine tables); numbers that need a build or a benchmark are
//! read from `tests/scores/status.tsv`, which `scripts/status.py` writes. Nothing here
//! measures, so `check` gives the same answer on every machine.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::bytecode::ops::OPS;
use djs_core::heap::cells::LAYOUTS;

/// test262's active group, from tests/test262.toml and the checkout in vendor/.
pub struct Test262 {
    pub active: usize,
    pub total: usize,
}

pub struct Status {
    /// `None` when there is no test262 checkout (the region keeps its old text).
    pub test262: Result<Test262, String>,
    /// Entries of tests/test262/expected-failures.txt (djs) and oracle-failures.txt (qjs).
    pub ratchet262: usize,
    pub oracle262: usize,
    pub corpus_cases: usize,
    pub corpus_skipped: usize,
    pub corpus_ratchet: usize,
    pub base_bytes: usize,
    pub builtins: usize,
    pub native: usize,
    pub fast: usize,
    /// `tests/scores/status.tsv`: key -> (value, measured at, date).
    pub recorded: BTreeMap<String, (String, String, String)>,
}

fn read(root: &Path, p: &str) -> String {
    std::fs::read_to_string(root.join(p)).unwrap_or_default()
}

/// Entries of a ratchet file: non-empty lines that are not comments.
fn ratchet_entries(text: &str) -> usize {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .count()
}

impl Status {
    pub fn compute(root: &Path) -> Status {
        let (corpus_cases, corpus_skipped) = corpus(&root.join("tests/corpus"));
        let (builtins, native, fast) = builtin_tags(&root.join("builtins"));
        Status {
            test262: test262(root),
            ratchet262: ratchet_entries(&read(root, "tests/test262/expected-failures.txt")),
            oracle262: ratchet_entries(&read(root, "tests/test262/oracle-failures.txt")),
            corpus_cases,
            corpus_skipped,
            corpus_ratchet: ratchet_entries(&read(root, "tests/corpus/expected-failures.txt")),
            base_bytes: djs_prelude::BASE.len(),
            builtins,
            native,
            fast,
            recorded: parse_recorded(&read(root, "tests/scores/status.tsv")),
        }
    }

    fn get(&self, key: &str) -> Option<&(String, String, String)> {
        self.recorded.get(key)
    }

    fn num(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.0.parse().ok())
    }
}

fn test262(root: &Path) -> Result<Test262, String> {
    use djs_test262::config::{Config, Kind};
    use djs_test262::suite::{self, Suite};
    let config = Config::load(&root.join("tests/test262.toml"))?;
    let dir = suite::locate_test262()?;
    if suite::checkout_commit(&dir).as_deref() != Some(suite::PINNED_COMMIT) {
        return Err(format!("{} is not at the pinned commit", dir.display()));
    }
    let suite = Suite::open(&dir)?;
    let paths = suite.discover(&[])?;
    let jobs = std::env::var("DJS_JOBS")
        .ok()
        .and_then(|j| j.parse().ok())
        .unwrap_or(4);
    let tests = suite.load(&config, paths, jobs);
    let active = tests
        .iter()
        .filter(|t| config.groups[t.group].kind == Kind::Active)
        .count();
    Ok(Test262 {
        active,
        total: tests.len(),
    })
}

/// Difftest cases (every .js/.ts below a corpus area directory) and those marked
/// `// difftest: skip-djs` in their leading comments, as djs-difftest finds them.
fn corpus(dir: &Path) -> (usize, usize) {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "js" || e == "ts") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                walk(&e.path(), &mut files);
            }
        }
    }
    let skipped = files
        .iter()
        .filter(|f| {
            let src = std::fs::read_to_string(f).unwrap_or_default();
            src.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map_while(|l| l.strip_prefix("//"))
                .any(|c| {
                    c.trim()
                        .strip_prefix("difftest:")
                        .is_some_and(|d| d.trim().starts_with("skip-djs"))
                })
        })
        .count();
    (files.len(), skipped)
}

/// Builtins (`@builtin`, `@constructor`, `@getter`, ... tags all start a builtin with
/// `@builtin`), and those tagged `@native` and `@fast`, in builtins/*.ts.
fn builtin_tags(dir: &Path) -> (usize, usize, usize) {
    let (mut b, mut n, mut f) = (0, 0, 0);
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (0, 0, 0);
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|e| e != "ts") || p.to_string_lossy().ends_with(".d.ts") {
            continue;
        }
        for l in std::fs::read_to_string(&p).unwrap_or_default().lines() {
            let Some(tag) = l.trim().strip_prefix('*').map(str::trim) else {
                continue;
            };
            match tag.split_whitespace().next() {
                Some("@builtin") => b += 1,
                Some("@native") => n += 1,
                Some("@fast") => f += 1,
                _ => {}
            }
        }
    }
    (b, n, f)
}

fn parse_recorded(text: &str) -> BTreeMap<String, (String, String, String)> {
    let mut out = BTreeMap::new();
    for l in text.lines() {
        if l.is_empty() || l.starts_with('#') || l.starts_with("key\t") {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if let [k, v, c, d] = f[..] {
            out.insert(k.to_string(), (v.to_string(), c.to_string(), d.to_string()));
        }
    }
    out
}

/// `1 known failure`, `40 known failures`.
fn plural(n: usize, what: &str) -> String {
    format!("{} {what}{}", thousands(n), if n == 1 { "" } else { "s" })
}

/// `1234567` -> `1,234,567`.
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Bytes as KB (1000), as the size tables have always used.
fn kb(bytes: f64) -> String {
    thousands((bytes / 1000.0).round() as usize) + " KB"
}

/// Wraps prose at 88 columns; continuation lines get `indent`.
fn wrap(text: &str, first: &str, indent: &str) -> String {
    let mut out = String::new();
    let mut line = first.to_string();
    let mut empty = true;
    for w in text.split_whitespace() {
        if !empty && line.len() + 1 + w.len() > 88 {
            out.push_str(line.trim_end());
            out.push('\n');
            line = indent.to_string();
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(w);
        empty = false;
    }
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

/// Where a recorded number comes from: `(measured at d780463b0d, 2026-11-01)`.
fn provenance(st: &Status, keys: &[&str]) -> String {
    let mut at: Vec<String> = keys
        .iter()
        .filter_map(|k| st.get(k))
        // An issue number (`set --at 0066`) means nothing to a reader of the guide (D2102):
        // only the date is shown then.
        .map(|(_, c, d)| {
            if c.len() == 4 && c.bytes().all(|b| b.is_ascii_digit()) {
                d.clone()
            } else {
                format!("{c}, {d}")
            }
        })
        .collect();
    at.sort();
    at.dedup();
    format!("measured at {}", at.join("; "))
}

/// The text of `gen:status WHAT`, or an error.
pub fn generate(st: &Status, what: &str) -> Result<String, String> {
    match what {
        "conformance" => conformance(st),
        "speed" => speed(st),
        "size" => size(st),
        "inventory" => Ok(inventory(st)),
        _ => Err(format!(
            "gen:status {what}: expected conformance, speed, size or inventory"
        )),
    }
}

fn conformance(st: &Status) -> Result<String, String> {
    let t = st.test262.as_ref().map_err(Clone::clone)?;
    let pass = t.active.saturating_sub(st.ratchet262);
    let pct = 100.0 * pass as f64 / t.active.max(1) as f64;
    let mut s = wrap(
        &format!(
            "test262 (at dquickts' pinned commit): djs passes **{} of the {}** tests in the \
             active group ({pct:.1}%; {} tests in all, the rest skipped by design). The {} \
             are listed with reasons in `tests/test262/expected-failures.txt`; dquickts' qjs \
             fails {} under the same runner.",
            thousands(pass),
            thousands(t.active),
            thousands(t.total),
            plural(st.ratchet262, "known failure"),
            plural(st.oracle262, "active test")
        ),
        "- ",
        "  ",
    );
    let run = st.corpus_cases - st.corpus_skipped;
    s += &wrap(
        &format!(
            "The differential corpus (codemode scripts, console and result formatting, error \
             messages, TypeScript, dquickts' own tests): **{} of {}** cases match qjs; {} \
             (`tests/corpus/expected-failures.txt`) and {} skipped as non-goals.",
            run.saturating_sub(st.corpus_ratchet),
            st.corpus_cases,
            plural(st.corpus_ratchet, "known failure"),
            st.corpus_skipped
        ),
        "- ",
        "  ",
    );
    if let (Some(p), Some(n)) = (st.num("pi/passed"), st.num("pi/total")) {
        s += &wrap(
            &format!(
                "pi's codemode test suite on djs: {p} of {n} tests pass ({}).",
                provenance(st, &["pi/passed", "pi/total"])
            ),
            "- ",
            "  ",
        );
    }
    Ok(s)
}

/// README's speed table: (workload, label).
const SPEED: &[(&str, &str)] = &[
    ("tight", "tight loop"),
    ("prop1", "property access (prop1)"),
    ("strings", "strings (split, slice, join, ...)"),
    ("arrays", "arrays"),
    ("json", "JSON round trips"),
    ("json-date", "JSON with Dates"),
    ("forof", "for-of"),
    ("calls", "calls"),
    ("push-pop", "push/pop and string methods"),
    ("pi-like", "a pi-like script"),
    ("foreach", "forEach"),
    ("higher-order", "map/filter/reduce (higher-order)"),
];

fn speed(st: &Status) -> Result<String, String> {
    let mut s = String::from(
        "| workload | djs-wasm | quickjs-wasi | ratio |\n| --- | ---: | ---: | ---: |\n",
    );
    let mut keys = Vec::new();
    for (w, label) in SPEED {
        let dk = format!("speed/{w}/djs-wasm");
        let qk = format!("speed/{w}/qjs-wasi");
        let (Some(d), Some(q)) = (st.num(&dk), st.num(&qk)) else {
            return Err(format!(
                "gen:status speed: no {dk} or {qk} in tests/scores/status.tsv (scripts/status.py speed)"
            ));
        };
        let _ = writeln!(
            s,
            "| {label} | {:.1}G | {:.1}G | {:.2} |",
            d / 1000.0,
            q / 1000.0,
            d / q
        );
        keys.push(dk);
    }
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    s.push('\n');
    s += &wrap(
        &format!(
            "Machine instructions per workload (`scripts/perf/w/`), {}; lower is better.",
            provenance(st, &keys)
        ),
        "",
        "",
    );
    Ok(s)
}

fn size(st: &Status) -> Result<String, String> {
    let keys = [
        "wasm/bytes",
        "wasm/gzip",
        "wasm/brotli",
        "wasm/code",
        "wasm/data",
    ];
    let v: Vec<f64> = keys
        .iter()
        .map(|k| {
            st.num(k).ok_or_else(|| {
                format!(
                    "gen:status size: no {k} in tests/scores/status.tsv (scripts/status.py size)"
                )
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(wrap(
        &format!(
            "`djs_wasm.wasm` is {} ({} gzip, {} brotli): {} of code and {} of data, of which \
             {} is the base snapshot (the initialized realm with all builtins). Module sizes \
             {}.",
            kb(v[0]),
            kb(v[1]),
            kb(v[2]),
            kb(v[3]),
            kb(v[4]),
            kb(st.base_bytes as f64),
            provenance(st, &keys)
        ),
        "",
        "",
    ))
}

fn inventory(st: &Status) -> String {
    let mut s = String::from("| | count |\n| --- | ---: |\n");
    let rows = [
        ("opcodes ([reference](ref-opcodes.md))", OPS.len()),
        ("cell kinds ([reference](ref-cells.md))", LAYOUTS.len()),
        (
            "intrinsics ([reference](ref-intrinsics.md))",
            INTRINSICS.len(),
        ),
        ("builtins in `builtins/*.ts`", st.builtins),
        ("of them `@native` (also compiled to Rust)", st.native),
        ("of them `@fast` (a Rust leaf entry)", st.fast),
        ("difftest corpus cases", st.corpus_cases),
        ("base snapshot, bytes", st.base_bytes),
    ];
    for (k, n) in rows {
        let _ = writeln!(s, "| {k} | {} |", thousands(n));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(thousands(38109), "38,109");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000000), "1,000,000");
        assert_eq!(kb(1_471_134.0), "1,471 KB");
        let w = wrap(&"word ".repeat(30), "- ", "  ");
        assert!(w.lines().all(|l| l.len() <= 88));
        assert!(w.lines().skip(1).all(|l| l.starts_with("  word")));
    }

    #[test]
    fn ratchets_and_records() {
        assert_eq!(ratchet_entries("# c\n\na: x\nb: y\n"), 2);
        let r = parse_recorded("# c\nkey\tvalue\tcommit\tdate\nwasm/bytes\t12\tabc\t2026-11-01\n");
        assert_eq!(r["wasm/bytes"].0, "12");
    }
}
