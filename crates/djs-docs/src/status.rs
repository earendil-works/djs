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

/// `gen:status recorded`: the recorded measurements with where they were taken (but the
/// engine comparison's, which the benchmarks page shows). Only
/// a page that may name pi lists those whose key names pi (its tests, its benchmark,
/// the pi-like workload), and it lists only those.
fn recorded(st: &Status, pi: bool) -> String {
    let names_pi = |k: &str| {
        k.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == "pi")
    };
    let mut s = String::from("| measurement | value | measured at |\n| --- | ---: | --- |\n");
    // The engine comparison's keys (a few hundred) are the benchmarks page's tables.
    let listed = |k: &str| names_pi(k) == pi && !k.starts_with("engines/");
    for (k, (v, c, d)) in st.recorded.iter().filter(|(k, _)| listed(k)) {
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
        "engines-size" | "engines-startup" | "engines-run" | "engines-memory" | "engines-dumps"
        | "engines-ops" => engines(st, what),
        "bridge-in" | "bridge-out" | "bridge-rate" | "bridge-memory" | "bridge-e2e" => {
            bridge(st, what)
        }
        _ => Err(format!(
            "gen:status {what}: expected conformance, speed, size, inventory, recorded, \
             engines-size|startup|run|memory|dumps|ops or bridge-in|out|rate|memory|e2e"
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

/// The speed table (`gen:status speed`): (workload, label).
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

/// The engines of the benchmarks page (js/bench/engines, `scripts/status.py engines`): key,
/// column label.
const ENGINES: [(&str, &str); 7] = [
    ("djs", "djs"),
    ("quickjs-wasi", "quickjs-wasi"),
    ("quickjs", "QuickJS"),
    ("quickjs-ng", "QuickJS-NG"),
    ("quickjs-emscripten", "quickjs-emscripten"),
    ("quickjs-emscripten-ng", "quickjs-emscripten (NG)"),
    ("monty", "monty"),
];
/// The codemode workload set (tests/bench/codemode), in the page's order.
const WORKLOADS: [&str; 9] = [
    "sequential",
    "fanout",
    "media",
    "inspect",
    "loop",
    "text",
    "json",
    "ts",
    "heavy",
];

/// The operations of js/bench/engines/ops.ts the benchmarks page lists: key (status.py's
/// `ops_slug` of the op's name), label.
const OPS_LISTED: [(&str, &str); 30] = [
    ("json-parse-rows-x5", "`JSON.parse` of 600 query rows"),
    (
        "json-stringify-rows-x5",
        "`JSON.stringify` of 600 query rows",
    ),
    ("split-lines-x3", "`split(\"\\n\")` of file texts"),
    ("join-lines-x3", "`join(\"\\n\")` of their lines"),
    ("includes-x2-per-line", "`includes` per line"),
    ("template-concat", "template literal appends"),
    ("trim-100k", "`trim`"),
    ("slice-100k", "`slice`"),
    ("padstart-50k", "`padStart`"),
    ("tolowercase-100k", "`toLowerCase`"),
    ("charcodeat-loop", "`charCodeAt` loop"),
    ("string-i-100k", "`String(i)`"),
    ("tofixed-50k", "`toFixed`"),
    ("math-max-per-line", "`Math.max`"),
    ("math-floor-100k", "`Math.floor`"),
    ("regex-match-function-w-g", "`match(/function \\w+/g)`"),
    ("regex-replace-s-g-x20", "`replace(/\\s+/g, \" \")`"),
    ("regex-exec-per-line", "`exec` of a line pattern"),
    ("matchall-gm", "`matchAll` with `/gm`"),
    ("new-regexp-5k", "`new RegExp`"),
    ("map-x200", "`map`"),
    ("sort-comparator-x50", "`sort` with a comparator"),
    ("sort-default-x50", "`sort` of strings"),
    ("push-300k", "`push`"),
    ("map-string-keys-x200", "`Map.set` with string keys"),
    ("set-string-keys-x200", "`Set.add` with string keys"),
    ("object-spread-x20", "object spread"),
    (
        "object-entries-fromentries-x200",
        "`Object.entries` and `fromEntries`",
    ),
    ("await-value-50k", "`await` of a value"),
    ("promise-all-20-x500", "`Promise.all` of 20"),
];

/// `gen:status engines-*`: the benchmarks page's tables.
fn engines(st: &Status, what: &str) -> Result<String, String> {
    let load_key = if what == "engines-ops" {
        "engines/ops/load"
    } else {
        "engines/load"
    };
    let mut keys: Vec<String> = vec![load_key.into()];
    let mut get = |k: String| -> Result<f64, String> {
        let v = st.num(&k).ok_or_else(|| {
            format!(
                "gen:status {what}: no {k} in tests/scores/status.tsv (scripts/status.py engines)"
            )
        })?;
        keys.push(k);
        Ok(v)
    };
    let ms = |v: f64| {
        if v < 10.0 {
            format!("{v:.2}")
        } else {
            format!("{v:.1}")
        }
    };
    let mib = |v: f64| format!("{:.1}", v / 1048576.0);
    let kib = |v: f64| format!("{:.1}", v / 1024.0);
    let header = |first: &str| {
        let mut s = format!("| {first} |");
        for (_, label) in ENGINES {
            let _ = write!(s, " {label} |");
        }
        s.push_str("\n| --- |");
        s.push_str(&" ---: |".repeat(ENGINES.len()));
        s.push('\n');
        s
    };
    let (mut s, note) = match what {
        "engines-size" => {
            let mut s = String::from(
                "| engine | wasm | gzip | brotli | JS glue, gzip |\n| --- | ---: | ---: | ---: | ---: |\n",
            );
            for (e, label) in ENGINES {
                let k = |m: &str| format!("engines/{e}/{m}");
                let glue = get(k("glue-gzip"))?;
                let _ = writeln!(
                    s,
                    "| {label} | {} | {} | {} | {} |",
                    kb(get(k("wasm-bytes"))?),
                    kb(get(k("wasm-gzip"))?),
                    kb(get(k("wasm-brotli"))?),
                    if glue == 0.0 {
                        "none".to_string()
                    } else {
                        kb(glue)
                    }
                );
            }
            (s, "Module bytes as shipped; gzip -9, brotli quality 11")
        }
        "engines-startup" => {
            let mut s = String::from(
                "| engine | compile, ms | new VM, ms | idle VM memory, MiB |\n| --- | ---: | ---: | ---: |\n",
            );
            for (e, label) in ENGINES {
                let k = |m: &str| format!("engines/{e}/{m}");
                let _ = writeln!(
                    s,
                    "| {label} | {} | {} | {} |",
                    ms(get(k("compile-ms"))?),
                    ms(get(k("startup-ms"))?),
                    mib(get(k("idle-memory"))?)
                );
            }
            (
                s,
                "Compile: the module, once per process. New VM: a new instance and VM, ready to evaluate (median). Memory: linear memory",
            )
        }
        "engines-run" | "engines-memory" => {
            let run = what == "engines-run";
            let mut s = header("workload");
            let mut logs = [0.0f64; ENGINES.len()];
            for w in WORKLOADS {
                let _ = write!(s, "| {w} |");
                let mut row = [0.0f64; ENGINES.len()];
                for (i, (e, _)) in ENGINES.iter().enumerate() {
                    row[i] = get(format!(
                        "engines/{e}/{w}/{}",
                        if run { "run-ms" } else { "memory" }
                    ))?;
                    let _ = write!(s, " {} |", if run { ms(row[i]) } else { mib(row[i]) });
                }
                for i in 0..ENGINES.len() {
                    logs[i] += (row[i] / row[0]).ln();
                }
                s.push('\n');
            }
            s.push_str("| relative to djs |");
            for l in logs {
                let _ = write!(s, " {:.2}× |", (l / WORKLOADS.len() as f64).exp());
            }
            s.push('\n');
            (
                s,
                if run {
                    "Milliseconds per run (median), in a warm process; the last row is the geometric mean of each engine's time over djs's"
                } else {
                    "Linear memory after the run, MiB; the last row is the geometric mean of each engine's over djs's"
                },
            )
        }
        "engines-ops" => {
            let mut s = String::from(
                "| operation | djs, ms | QuickJS, ms | djs / QuickJS |\n| --- | ---: | ---: | ---: |\n",
            );
            for (k, label) in OPS_LISTED {
                let d = get(format!("engines/ops/{k}/djs"))?;
                let q = get(format!("engines/ops/{k}/quickjs"))?;
                let ratio = if d < 0.05 || q < 0.05 {
                    "-".to_string()
                } else {
                    format!("{:.2}", d / q)
                };
                let _ = writeln!(s, "| {label} | {} | {} | {ratio} |", ms(d), ms(q));
            }
            (
                s,
                "Milliseconds per loop of the operation (js/bench/engines/ops.ts), the setup's time subtracted; QuickJS: the faster of the two latest releases",
            )
        }
        _ => {
            let mut s = String::from(
                "| workload | idle points | dump, KiB (max) | gzip, KiB (max) | write, ms | load, ms |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
            );
            for w in WORKLOADS {
                let k = |m: &str| format!("engines/djs/{w}/dump-{m}");
                if st.num(&k("points")).is_none() {
                    let _ = writeln!(s, "| {w} | 0 | | | | |");
                    continue;
                }
                let _ = writeln!(
                    s,
                    "| {w} | {} | {} ({}) | {} ({}) | {} | {} |",
                    get(k("points"))?,
                    kib(get(k("raw-p50"))?),
                    kib(get(k("raw-max"))?),
                    kib(get(k("gzip-p50"))?),
                    kib(get(k("gzip-max"))?),
                    ms(get(k("ms"))?),
                    ms(get(k("load-ms"))?)
                );
            }
            (
                s,
                "Medians over a workload's idle points; load: a new sandbox with the run loaded",
            )
        }
    };
    let load = st.num(load_key).unwrap_or(f64::NAN);
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    s.push('\n');
    s += &wrap(
        &format!("{note}; {}, load {load:.1}.", provenance(st, &keys)),
        "",
        "",
    );
    Ok(s)
}

/// The payloads of js/bench/engines/bridge.ts, in the page's order: key, label.
const BRIDGE_PAYLOADS: [(&str, &str); 11] = [
    ("args", "call arguments"),
    ("json-1k", "JSON, 1 KB"),
    ("json-100k", "JSON, 100 KB"),
    ("json-1m", "JSON, 1 MB"),
    ("json-10m", "JSON, 10 MB"),
    ("text-1m", "ASCII, 1 MB"),
    ("text-10m", "ASCII, 10 MB"),
    ("unicode-1m", "non-ASCII, 1 MB"),
    ("numbers", "100k numbers"),
    ("bytes-1m", "bytes, 1 MiB"),
    ("nested", "nested 100 deep"),
];
/// Its end-to-end cases: key, label.
const BRIDGE_CASES: [(&str, &str); 3] = [
    ("field-1m", "one field of 1 MB"),
    ("edit-1m", "edit a 1 MB file"),
    ("calls-1000", "1,000 small calls"),
];

/// A bridge time: three significant digits, down to microseconds.
fn bridge_ms(v: f64) -> String {
    crate::chart::short(v)
}

/// `gen:status bridge-*`: the benchmarks page's tables of passing data through the bridge.
fn bridge(st: &Status, what: &str) -> Result<String, String> {
    let mut keys: Vec<String> = vec!["engines/bridge/load".into()];
    let mut get = |k: String| -> Result<f64, String> {
        let v = st.num(&k).ok_or_else(|| {
            format!(
                "gen:status {what}: no {k} in tests/scores/status.tsv (scripts/status.py bridge)"
            )
        })?;
        keys.push(k);
        Ok(v)
    };
    let header = |first: &str, extra: &str| {
        let mut s = format!("| {first} |{extra}");
        for (_, label) in ENGINES {
            let _ = write!(s, " {label} |");
        }
        s.push_str("\n| --- |");
        if !extra.is_empty() {
            s.push_str(" ---: |");
        }
        s.push_str(&" ---: |".repeat(ENGINES.len()));
        s.push('\n');
        s
    };
    let (mut s, note) = match what {
        "bridge-in" | "bridge-out" => {
            let dir = &what["bridge-".len()..];
            let mut s = header("payload", " size |");
            let mut logs = [0.0f64; ENGINES.len()];
            let mut n = 0;
            for (p, label) in BRIDGE_PAYLOADS {
                let bytes = get(format!("engines/bridge/payload/{p}/bytes"))?;
                let size = if bytes < 1000.0 {
                    format!("{bytes} B")
                } else {
                    kb(bytes)
                };
                let _ = write!(s, "| {label} | {size} |");
                let mut row = [None; ENGINES.len()];
                for (i, (e, _)) in ENGINES.iter().enumerate() {
                    // An engine that cannot take the payload has no number.
                    row[i] = get(format!("engines/bridge/{e}/{p}/{dir}-ms")).ok();
                    match row[i] {
                        Some(v) => {
                            let _ = write!(s, " {} |", bridge_ms(v));
                        }
                        None => s.push_str(" fails |"),
                    }
                }
                if row.iter().all(Option::is_some) {
                    n += 1;
                    for i in 0..ENGINES.len() {
                        logs[i] += (row[i].unwrap() / row[0].unwrap()).ln();
                    }
                }
                s.push('\n');
            }
            s.push_str("| relative to djs | |");
            for l in logs {
                let _ = write!(s, " {:.1}× |", (l / n.max(1) as f64).exp());
            }
            s.push('\n');
            (
                s,
                if dir == "in" {
                    "Milliseconds per value put into a fresh VM (median), each engine's best path; the last row is the geometric mean of each engine's time over djs's"
                } else {
                    "Milliseconds per value read out of a VM (median), each engine's best path; the last row is the geometric mean of each engine's time over djs's"
                },
            )
        }
        "bridge-rate" => {
            let cols = [
                ("json-1m", "JSON 1 MB"),
                ("json-10m", "JSON 10 MB"),
                ("text-10m", "ASCII 10 MB"),
                ("unicode-1m", "non-ASCII 1 MB"),
                ("bytes-1m", "bytes 1 MiB"),
            ];
            let mut s = String::from("| engine |");
            for (_, l) in cols {
                let _ = write!(s, " {l}, in | out |");
            }
            s.push_str("\n| --- |");
            s.push_str(&" ---: | ---: |".repeat(cols.len()));
            s.push('\n');
            for (e, label) in ENGINES {
                let _ = write!(s, "| {label} |");
                for (p, _) in cols {
                    let bytes = get(format!("engines/bridge/payload/{p}/bytes"))?;
                    for dir in ["in", "out"] {
                        let ms = get(format!("engines/bridge/{e}/{p}/{dir}-ms"))?;
                        let _ = write!(s, " {} |", thousands((bytes / 1e3 / ms).round() as usize));
                    }
                }
                s.push('\n');
            }
            (s, "MB per second: the payload's size over the median time")
        }
        "bridge-memory" => {
            let mut s = String::from(
                "| engine | idle | JSON 10 MB in | and out | ASCII 10 MB in | and out |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
            );
            let mib = |v: f64| format!("{:.1}", v / 1048576.0);
            for (e, label) in ENGINES {
                let k = |m: &str| format!("engines/bridge/{e}/{m}");
                let _ = writeln!(
                    s,
                    "| {label} | {} | {} | {} | {} | {} |",
                    mib(get(k("idle-memory"))?),
                    mib(get(k("json-10m/in-memory"))?),
                    mib(get(k("json-10m/out-memory"))?),
                    mib(get(k("text-10m/in-memory"))?),
                    mib(get(k("text-10m/out-memory"))?)
                );
            }
            (
                s,
                "Linear memory, MiB: of a fresh VM, after putting the value in, and after reading it back out",
            )
        }
        _ => {
            let mut s = header("case", "");
            for (c, label) in BRIDGE_CASES {
                let _ = write!(s, "| {label} |");
                for (e, _) in ENGINES {
                    let _ = write!(
                        s,
                        " {} |",
                        bridge_ms(get(format!("engines/bridge/{e}/e2e/{c}-ms"))?)
                    );
                }
                s.push('\n');
            }
            (
                s,
                "Milliseconds per run in a fresh VM (median): compiling and running the script, its tool calls through each engine's host function",
            )
        }
    };
    let load = st.num("engines/bridge/load").unwrap_or(f64::NAN);
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    s.push('\n');
    s += &wrap(
        &format!("{note}; {}, load {load:.1}.", provenance(st, &keys)),
        "",
        "",
    );
    Ok(s)
}

/// Each engine's palette color in the charts (chart.rs), in `ENGINES` order: djs blue,
/// the QuickJS builds warm colors and teal, monty green.
const ENGINE_COLORS: [usize; 7] = [0, 3, 1, 2, 5, 6, 4];

/// A generated chart: its file under docs/guide, its alt text (the figure's caption) and
/// the SVG.
pub struct Chart {
    pub file: String,
    pub alt: String,
    pub svg: String,
}

/// The chart drawn above the table of `gen:status WHAT`, if it has one. Like the tables
/// it only reads status.tsv.
pub fn chart(st: &Status, what: &str) -> Result<Option<Chart>, String> {
    use crate::chart::{Meta, Panel, Series, bars, columns, ratios, short};
    let get = |k: String| -> Result<f64, String> {
        st.num(&k)
            .ok_or_else(|| format!("gen:status {what}: no {k} in tests/scores/status.tsv"))
    };
    let name = format!("chart-{what}");
    let series: Vec<Series> = ENGINES
        .iter()
        .zip(ENGINE_COLORS)
        .map(|((_, label), color)| Series { label, color })
        .collect();
    let rows: Vec<(&str, usize)> = ENGINES
        .iter()
        .zip(ENGINE_COLORS)
        .map(|((_, l), c)| (*l, c))
        .collect();
    let per_engine = |m: &str, scale: f64| -> Result<Vec<Option<f64>>, String> {
        ENGINES
            .iter()
            .map(|(e, _)| get(format!("engines/{e}/{m}")).map(|v| Some(v / scale)))
            .collect()
    };
    let (alt, svg) = match what {
        "engines-size" => {
            let alt = "Module size per engine, raw and with gzip, in KB on a logarithmic scale";
            let svg = bars(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Horizontal bars per engine: the wasm module's bytes and its gzip -9 size. The table below has the exact numbers.",
                },
                &rows,
                &[
                    Panel {
                        title: "wasm, KB",
                        values: per_engine("wasm-bytes", 1000.0)?,
                        log: true,
                        format: |v| short(v.round()),
                    },
                    Panel {
                        title: "gzip, KB",
                        values: per_engine("wasm-gzip", 1000.0)?,
                        log: true,
                        format: |v| short(v.round()),
                    },
                ],
            );
            (alt, svg)
        }
        "engines-startup" => {
            let alt = "Compiling the module, starting a VM and an idle VM's memory, per engine";
            let svg = bars(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Horizontal bars per engine: compile time in ms and idle memory in MiB on logarithmic scales, new-VM time in ms on a linear one. The table below has the exact numbers.",
                },
                &rows,
                &[
                    Panel {
                        title: "compile, ms",
                        values: per_engine("compile-ms", 1.0)?,
                        log: true,
                        format: short,
                    },
                    Panel {
                        title: "new VM, ms",
                        values: per_engine("startup-ms", 1.0)?,
                        log: false,
                        format: short,
                    },
                    Panel {
                        title: "idle memory, MiB",
                        values: per_engine("idle-memory", 1048576.0)?,
                        log: true,
                        format: |v| format!("{v:.1}"),
                    },
                ],
            );
            (alt, svg)
        }
        "engines-run" | "engines-memory" => {
            let run = what == "engines-run";
            let values = WORKLOADS
                .iter()
                .map(|w| {
                    ENGINES
                        .iter()
                        .map(|(e, _)| {
                            if run {
                                get(format!("engines/{e}/{w}/run-ms")).map(Some)
                            } else {
                                get(format!("engines/{e}/{w}/memory")).map(|v| Some(v / 1048576.0))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            let alt = if run {
                "Run time per workload and engine, in ms on a logarithmic scale"
            } else {
                "Linear memory after the run per workload and engine, in MiB on a logarithmic scale"
            };
            let svg = columns(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Grouped columns: one group per workload, one column per engine, lower is better. The table below has the exact numbers.",
                },
                if run { "ms" } else { "MiB" },
                &WORKLOADS,
                &series,
                &values,
                true,
            );
            (alt, svg)
        }
        "engines-ops" => {
            let mut rows = Vec::new();
            for (k, label) in OPS_LISTED {
                let d = get(format!("engines/ops/{k}/djs"))?;
                let q = get(format!("engines/ops/{k}/quickjs"))?;
                rows.push((label, (d >= 0.05 && q >= 0.05).then(|| d / q)));
            }
            let alt = "djs's time over the best QuickJS build's, per operation, on a logarithmic scale around parity";
            let svg = ratios(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Bars from the parity line at 1: to the left djs is faster, to the right slower. The table below has the times.",
                },
                &rows,
                "djs faster",
                "djs slower",
            );
            (alt, svg)
        }
        "engines-dumps" => {
            let rows: Vec<(&str, usize)> = WORKLOADS.iter().map(|w| (*w, 0)).collect();
            let col = |m: &str, scale: f64| -> Result<Vec<Option<f64>>, String> {
                WORKLOADS
                    .iter()
                    .map(|w| {
                        let k = format!("engines/djs/{w}/dump-{m}");
                        if st.num(&format!("engines/djs/{w}/dump-points")).is_none() {
                            Ok(None)
                        } else {
                            get(k).map(|v| Some(v / scale))
                        }
                    })
                    .collect()
            };
            let alt = "djs's continuation dumps per workload: size, and the time to write and to load one";
            let svg = bars(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Horizontal bars per workload: the median dump size in KiB on a logarithmic scale, the median write and load times in ms. A workload that never waits has no dump. The table below has the exact numbers.",
                },
                &rows,
                &[
                    Panel {
                        title: "dump, KiB",
                        values: col("raw-p50", 1024.0)?,
                        log: true,
                        format: |v| format!("{v:.1}"),
                    },
                    Panel {
                        title: "write, ms",
                        values: col("ms", 1.0)?,
                        log: false,
                        format: short,
                    },
                    Panel {
                        title: "load, ms",
                        values: col("load-ms", 1.0)?,
                        log: false,
                        format: short,
                    },
                ],
            );
            (alt, svg)
        }
        "bridge-in" | "bridge-out" => {
            let dir = &what["bridge-".len()..];
            let groups: Vec<&str> = BRIDGE_PAYLOADS.iter().map(|(_, l)| *l).collect();
            let values = BRIDGE_PAYLOADS
                .iter()
                .map(|(p, _)| {
                    ENGINES
                        .iter()
                        .map(|(e, _)| st.num(&format!("engines/bridge/{e}/{p}/{dir}-ms")))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if values.iter().flatten().all(Option::is_none) {
                return Err(format!(
                    "gen:status {what}: no engines/bridge/* in tests/scores/status.tsv"
                ));
            }
            let alt = if dir == "in" {
                "Putting a value into the VM, per payload and engine, in ms on a logarithmic scale"
            } else {
                "Reading a value out of the VM, per payload and engine, in ms on a logarithmic scale"
            };
            let svg = columns(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Grouped columns: one group per payload, one column per engine, lower is better. The table below has the exact numbers.",
                },
                "ms",
                &groups,
                &series,
                &values,
                true,
            );
            (alt, svg)
        }
        "bridge-e2e" => {
            let panels = BRIDGE_CASES
                .iter()
                .map(|(c, label)| {
                    Ok(Panel {
                        title: label,
                        values: ENGINES
                            .iter()
                            .map(|(e, _)| get(format!("engines/bridge/{e}/e2e/{c}-ms")).map(Some))
                            .collect::<Result<Vec<_>, String>>()?,
                        log: true,
                        format: short,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let alt =
                "Whole runs that pass data through a tool, per engine, in ms on logarithmic scales";
            let svg = bars(
                &Meta {
                    name: &name,
                    title: alt,
                    desc: "Horizontal bars per engine, one panel per case: a tool's 1 MB result of which the script reads one field, a 1 MB file read, edited and written back, and a thousand small calls. Lower is better. The table below has the exact numbers.",
                },
                &rows,
                &panels,
            );
            (alt, svg)
        }
        _ => return Ok(None),
    };
    Ok(Some(Chart {
        file: format!("gen/{name}.svg"),
        alt: alt.to_string(),
        svg,
    }))
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
    fn engine_tables() {
        let mut rec = String::from("engines/load\t2.5\tabc\t2026-11-06\n");
        for (e, _) in ENGINES {
            for m in [
                "wasm-bytes",
                "wasm-gzip",
                "wasm-brotli",
                "glue-gzip",
                "compile-ms",
                "startup-ms",
                "idle-memory",
            ] {
                rec += &format!("engines/{e}/{m}\t1000\tabc\t2026-11-06\n");
            }
            for w in WORKLOADS {
                rec += &format!(
                    "engines/{e}/{w}/run-ms\t2\tabc\t2026-11-06\nengines/{e}/{w}/memory\t1048576\tabc\t2026-11-06\n"
                );
            }
        }
        for w in WORKLOADS.iter().filter(|w| **w != "inspect") {
            for m in [
                "points", "raw-p50", "raw-max", "gzip-p50", "gzip-max", "ms", "load-ms",
            ] {
                rec += &format!("engines/djs/{w}/dump-{m}\t3\tabc\t2026-11-06\n");
            }
        }
        rec += "engines/bridge/load\t2.0\tabc\t2026-11-06\n";
        for (p, _) in BRIDGE_PAYLOADS {
            rec += &format!("engines/bridge/payload/{p}/bytes\t1000000\tabc\t2026-11-06\n");
            for (e, _) in ENGINES {
                for m in ["in-ms", "out-ms", "in-memory", "out-memory"] {
                    // monty cannot take the nested payload: no numbers.
                    if !(e == "monty" && p == "nested") {
                        rec += &format!("engines/bridge/{e}/{p}/{m}\t2\tabc\t2026-11-06\n");
                    }
                }
            }
        }
        for (e, _) in ENGINES {
            rec += &format!("engines/bridge/{e}/idle-memory\t1048576\tabc\t2026-11-06\n");
            for (c, _) in BRIDGE_CASES {
                rec += &format!("engines/bridge/{e}/e2e/{c}-ms\t3\tabc\t2026-11-06\n");
            }
        }
        let st = Status {
            test262: Err("no checkout".into()),
            ratchet262: 0,
            oracle262: 0,
            corpus_cases: 0,
            corpus_skipped: 0,
            corpus_ratchet: 0,
            harvest: 0,
            base_bytes: 0,
            base_tail: 0,
            builtins: 0,
            native: 0,
            fast: 0,
            recorded: parse_recorded(&rec),
        };
        let run = generate(&st, "engines-run", false).unwrap();
        assert!(run.contains("| heavy | 2.00 |"), "{run}");
        assert!(run.contains("| relative to djs | 1.00× |"), "{run}");
        assert!(
            run.contains("measured at abc, 2026-11-06, load 2.5."),
            "{run}"
        );
        assert!(
            generate(&st, "engines-dumps", false)
                .unwrap()
                .contains("| inspect | 0 |")
        );
        assert!(
            generate(&st, "engines-size", false)
                .unwrap()
                .contains("| djs | 1 KB |")
        );
        for what in [
            "engines-size",
            "engines-startup",
            "engines-run",
            "engines-memory",
            "engines-dumps",
        ] {
            let c = chart(&st, what).unwrap().unwrap();
            assert_eq!(c.file, format!("gen/chart-{what}.svg"));
            assert!(c.svg.contains("<desc id="), "{what}");
        }
        assert!(chart(&st, "inventory").unwrap().is_none());
        let t = generate(&st, "bridge-in", false).unwrap();
        assert!(t.contains("| nested 100 deep | 1,000 KB | 2 |"), "{t}");
        assert!(t.contains(" fails |"), "{t}");
        assert!(t.contains("| relative to djs | | 1.0× |"), "{t}");
        assert!(
            generate(&st, "bridge-rate", false)
                .unwrap()
                .contains("| djs | 500 |")
        );
        for what in ["bridge-out", "bridge-memory", "bridge-e2e"] {
            generate(&st, what, false).unwrap();
        }
        assert!(chart(&st, "bridge-e2e").unwrap().is_some());
        let mut st = st;
        st.recorded.remove("engines/monty/heavy/memory");
        assert!(chart(&st, "engines-memory").is_err());
        assert!(
            generate(&st, "engines-memory", false)
                .unwrap_err()
                .contains("engines/monty/heavy/memory")
        );
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
