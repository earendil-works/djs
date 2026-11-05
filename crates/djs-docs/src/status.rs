//! Status numbers for `gen:status` regions (D2101): counts that follow from the
//! repository are computed here (test262 groups and ratchets, corpus cases, the base
//! snapshot, builtin tags, engine tables); numbers that need a build or a benchmark are
//! read from `tests/scores/status.tsv`, which `scripts/status.py` writes. Nothing here
//! measures, so `check` gives the same answer on every machine. Besides the regions,
//! prose can carry single numbers inline (`<!--n KEY FMT-->...<!--/n-->`, [`value`]).

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use djs_core::bytecode::intrinsics::{INTRINSICS, IntrinsicKind};
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
    /// Scripts of tests/corpus/codemode-harvest: codemode scripts models wrote.
    pub harvest: usize,
    pub base_bytes: usize,
    /// The base's tail: the cells past the builtin prefix, which every VM writes.
    pub base_tail: usize,
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
            harvest: std::fs::read_dir(root.join("tests/corpus/codemode-harvest"))
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.path().extension().is_some_and(|x| x == "js"))
                        .count()
                })
                .unwrap_or(0),
            base_bytes: djs_prelude::BASE.len(),
            base_tail: djs_prelude::new_vm(Default::default()).map_or(0, |vm| {
                djs_prelude::BASE.len() - vm.heap().base_end() as usize
            }),
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

/// Why an inline number has no value.
pub enum Missing {
    /// It is a test262 count and there is no checkout at the pinned commit.
    Test262(String),
    Key(String),
}

fn intrinsics_of(kind: IntrinsicKind) -> usize {
    INTRINSICS.iter().filter(|i| i.kind == kind).count()
}

/// The numbers computed from the repository, by inline key.
fn computed(st: &Status, key: &str) -> Option<Result<f64, Missing>> {
    let t = || st.test262.as_ref().map_err(|e| Missing::Test262(e.clone()));
    let run = st.corpus_cases - st.corpus_skipped;
    let n = |v: usize| Some(Ok(v as f64));
    match key {
        "test262/active" => Some(t().map(|t| t.active as f64)),
        "test262/total" => Some(t().map(|t| t.total as f64)),
        "test262/passed" => Some(t().map(|t| t.active.saturating_sub(st.ratchet262) as f64)),
        "test262/known-failures" => n(st.ratchet262),
        "test262/oracle-failures" => n(st.oracle262),
        "corpus/cases" => n(st.corpus_cases),
        "corpus/passed" => n(run.saturating_sub(st.corpus_ratchet)),
        "corpus/known-failures" => n(st.corpus_ratchet),
        "corpus/skipped" => n(st.corpus_skipped),
        "corpus/harvest" => n(st.harvest),
        "base/bytes" => n(st.base_bytes),
        "base/tail" => n(st.base_tail),
        "builtins/all" => n(st.builtins),
        "builtins/native" => n(st.native),
        "builtins/fast" => n(st.fast),
        "intrinsics/all" => n(INTRINSICS.len()),
        "intrinsics/leaf" => n(intrinsics_of(IntrinsicKind::Leaf)),
        "intrinsics/reentrant" => n(intrinsics_of(IntrinsicKind::Reentrant)),
        "opcodes" => n(OPS.len()),
        "cell-kinds" => n(LAYOUTS.len()),
        _ => None,
    }
}

/// The text of an inline number in prose, `<!--n KEY FMT-->TEXT<!--/n-->`: KEY is a
/// count computed from the repository ([`computed`]) or a key of
/// `tests/scores/status.tsv`. FMT: none (an integer with thousands separators, else
/// the recorded text), `.N` (N decimals), `~K` (rounded to a multiple of K), `kb`, `mb`
/// (bytes in KB or MB, 1000-based), `kib`, `mib` (1024-based): `kb.N` and so on with N
/// decimals; the MB units have one by default.
pub fn value(st: &Status, key: &str, fmt: &str) -> Result<String, Missing> {
    let (v, text) = match computed(st, key) {
        Some(v) => {
            let v = v?;
            (v, v.to_string())
        }
        None => {
            let (text, _, _) = st.get(key).ok_or_else(|| {
                Missing::Key(format!(
                    "inline number {key}: no such count and no {key} in tests/scores/status.tsv (scripts/status.py)"
                ))
            })?;
            let v = text.parse::<f64>().map_err(|_| {
                Missing::Key(format!("inline number {key}: {text:?} is not a number"))
            })?;
            (v, text.clone())
        }
    };
    let bad = || Missing::Key(format!("inline number {key}: unknown format {fmt:?}"));
    let decimals = |d: &str, default: usize| -> Result<usize, Missing> {
        match d {
            "" => Ok(default),
            _ => d
                .strip_prefix('.')
                .and_then(|d| d.parse().ok())
                .ok_or_else(bad),
        }
    };
    let fixed = |v: f64, d: usize| {
        if d == 0 {
            thousands(v.round() as usize)
        } else {
            let s = format!("{v:.d$}");
            let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
            format!("{}.{frac}", thousands(int.parse().unwrap_or(0)))
        }
    };
    Ok(if fmt.is_empty() {
        if v.fract() == 0.0 && v >= 0.0 {
            thousands(v as usize)
        } else {
            text
        }
    } else if let Some(d) = fmt.strip_prefix("kib") {
        fixed(v / 1024.0, decimals(d, 0)?) + " KiB"
    } else if let Some(d) = fmt.strip_prefix("mib") {
        fixed(v / 1048576.0, decimals(d, 1)?) + " MiB"
    } else if let Some(d) = fmt.strip_prefix("kb") {
        fixed(v / 1000.0, decimals(d, 0)?) + " KB"
    } else if let Some(d) = fmt.strip_prefix("mb") {
        fixed(v / 1e6, decimals(d, 1)?) + " MB"
    } else if let Some(k) = fmt.strip_prefix('~') {
        let k: f64 = k.parse().map_err(|_| bad())?;
        thousands(((v / k).round() * k) as usize)
    } else {
        fixed(v, decimals(fmt, 0)?)
    })
}

/// `gen:status recorded`: the recorded measurements with where they were taken. Only
/// a page that may name pi lists those whose key names pi (its tests, its benchmark,
/// the pi-like workload), and it lists only those.
fn recorded(st: &Status, pi: bool) -> String {
    let names_pi = |k: &str| k.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == "pi");
    let mut s = String::from("| measurement | value | measured at |\n| --- | ---: | --- |\n");
    for (k, (v, c, d)) in st.recorded.iter().filter(|(k, _)| names_pi(k) == pi) {
        let at = if c.len() == 4 && c.bytes().all(|b| b.is_ascii_digit()) {
            d.clone()
        } else {
            format!("{c}, {d}")
        };
        let _ = writeln!(s, "| `{k}` | {v} | {at} |");
    }
    s
}

/// The text of `gen:status WHAT`, or an error.
/// `pi`: the region may name pi (README.md and the guide's pi page): its test suite, its
/// workload by name. Elsewhere the guide names no host.
pub fn generate(st: &Status, what: &str, pi: bool) -> Result<String, String> {
    match what {
        "conformance" => conformance(st, pi),
        "speed" => speed(st, pi),
        "size" => size(st),
        "inventory" => Ok(inventory(st)),
        "recorded" => Ok(recorded(st, pi)),
        _ => Err(format!(
            "gen:status {what}: expected conformance, speed, size, inventory or recorded"
        )),
    }
}

fn conformance(st: &Status, pi: bool) -> Result<String, String> {
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
    if let (true, Some(p), Some(n)) = (pi, st.num("pi/passed"), st.num("pi/total")) {
        // The adapter's next mode, and that with a dump and a resume at every idle point.
        let modes = [
            ("pi/next", "in the next mode"),
            ("pi/dump", "with a dump at every idle point"),
        ];
        let mut keys = vec!["pi/passed".to_string(), "pi/total".to_string()];
        let mut same = vec!["in the default mode"];
        let mut other = Vec::new();
        for (k, what) in modes {
            let (pk, nk) = (format!("{k}/passed"), format!("{k}/total"));
            if let (Some(mp), Some(mn)) = (st.num(&pk), st.num(&nk)) {
                if (mp, mn) == (p, n) {
                    same.push(what);
                } else {
                    other.push(format!("{mp} of {mn} {what}"));
                }
                keys.extend([pk, nk]);
            }
        }
        let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
        let same = match same.as_slice() {
            [one] => one.to_string(),
            [init @ .., last] => format!("{} and {last}", init.join(", ")),
            [] => String::new(),
        };
        let other = if other.is_empty() {
            String::new()
        } else {
            format!("; {}", other.join("; "))
        };
        s += &wrap(
            &format!(
                "pi's codemode test suite on djs: {p} of {n} tests pass {same}{other} ({}).",
                provenance(st, &keys)
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

fn speed(st: &Status, pi: bool) -> Result<String, String> {
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
        let label = if pi {
            label.to_string()
        } else {
            label.replace("a pi-like", "an agent-like")
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
        "wasm/base-coded",
        "slim/bytes",
        "slim/gzip",
        "dump/bytes",
        "dump/gzip",
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
            "`djs_wasm.wasm` is {} ({} gzip, {} brotli): {} of code, {} of data, and the base \
             snapshot (the initialized realm with all builtins, {}) coded to {} in a custom \
             section. The slim build is {} ({} gzip); the dump module, the shipped build with \
             the dumper, {} ({} gzip). Module sizes {}.",
            kb(v[0]),
            kb(v[1]),
            kb(v[2]),
            kb(v[3]),
            kb(v[4]),
            kb(st.base_bytes as f64),
            kb(v[5]),
            kb(v[6]),
            kb(v[7]),
            kb(v[8]),
            kb(v[9]),
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

    #[test]
    fn inline_values() {
        let recorded = parse_recorded(
            "a/bytes\t1033538\tabc\t2026-11-05\nb/ratio\t1.456\tabc\t2026-11-05\nc/n\t38109\tabc\t2026-11-05\n",
        );
        let st = Status {
            test262: Err("no checkout".into()),
            ratchet262: 40,
            oracle262: 4,
            corpus_cases: 10,
            corpus_skipped: 1,
            corpus_ratchet: 2,
            harvest: 0,
            base_bytes: 186_376,
            base_tail: 0,
            builtins: 0,
            native: 0,
            fast: 0,
            recorded,
        };
        let v = |k: &str, f: &str| value(&st, k, f).ok().unwrap();
        assert_eq!(v("a/bytes", ""), "1,033,538");
        assert_eq!(v("a/bytes", "kb"), "1,034 KB");
        assert_eq!(v("a/bytes", "mb"), "1.0 MB");
        assert_eq!(v("a/bytes", "mb.2"), "1.03 MB");
        assert_eq!(v("a/bytes", "kib"), "1,009 KiB");
        assert_eq!(v("a/bytes", "mib"), "1.0 MiB");
        assert_eq!(v("b/ratio", ""), "1.456");
        assert_eq!(v("b/ratio", ".1"), "1.5");
        assert_eq!(v("c/n", "~1000"), "38,000");
        assert_eq!(v("c/n", ".1"), "38,109.0");
        assert_eq!(v("corpus/passed", ""), "7");
        assert_eq!(v("base/bytes", "kb"), "186 KB");
        assert!(matches!(
            value(&st, "test262/active", ""),
            Err(Missing::Test262(_))
        ));
        assert!(matches!(value(&st, "nope", ""), Err(Missing::Key(_))));
        assert!(matches!(value(&st, "a/bytes", "zz"), Err(Missing::Key(_))));
    }
}
