//! `tests/scores/bench.tsv`: one line per run and benchmark, appended by `--record`,
//! read by `--compare`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runner::{Measurement, Unit};

pub const HEADER: &str = "date\tcommit\tmachine\tbenchmark\tvalue\tunit";

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub date: String,
    pub commit: String,
    pub machine: String,
    pub benchmark: String,
    pub value: f64,
    pub unit: Unit,
}

pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.is_empty() || line == HEADER || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let bad = || format!("bench.tsv line {}: expected 6 tab-separated fields", i + 1);
        if f.len() != 6 {
            return Err(bad());
        }
        rows.push(Row {
            date: f[0].into(),
            commit: f[1].into(),
            machine: f[2].into(),
            benchmark: f[3].into(),
            value: f[4].parse().map_err(|_| bad())?,
            unit: Unit::parse(f[5]).ok_or_else(bad)?,
        });
    }
    Ok(rows)
}

pub fn load(path: &Path) -> Result<Vec<Row>, String> {
    match std::fs::read_to_string(path) {
        Ok(t) => parse(&t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Appends one run (creating the file with its header).
pub fn append(
    path: &Path,
    date: &str,
    commit: &str,
    machine: &str,
    results: &[Measurement],
) -> Result<(), String> {
    let mut text = std::fs::read_to_string(path).unwrap_or_default();
    if text.is_empty() {
        text.push_str(HEADER);
        text.push('\n');
    }
    for m in results {
        writeln!(
            text,
            "{date}\t{commit}\t{machine}\t{}\t{}\t{}",
            m.name,
            format_value(m.value),
            m.unit.as_str()
        )
        .unwrap();
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Four significant digits (enough for noisy timings; sizes stay exact).
pub fn format_value(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{v:.0}");
    }
    let digits = if v.abs() >= 1000.0 {
        0
    } else if v.abs() >= 100.0 {
        1
    } else if v.abs() >= 10.0 {
        2
    } else {
        3
    };
    format!("{v:.digits$}")
}

/// The rows to compare against: those of the commit `rev` (a prefix), or else the most
/// recent run on `machine`. Prefers rows from `machine`.
pub fn baseline<'a>(rows: &'a [Row], machine: &str, rev: Option<&str>) -> Vec<&'a Row> {
    let same_machine: Vec<&Row> = rows.iter().filter(|r| r.machine == machine).collect();
    let pool = if same_machine.is_empty() {
        rows.iter().collect()
    } else {
        same_machine
    };
    let key = match rev {
        Some(rev) => match pool.iter().rev().find(|r| r.commit.starts_with(rev)) {
            Some(r) => (r.date.clone(), r.commit.clone()),
            None => return Vec::new(),
        },
        None => match pool.last() {
            Some(r) => (r.date.clone(), r.commit.clone()),
            None => return Vec::new(),
        },
    };
    pool.into_iter()
        .filter(|r| r.date == key.0 && r.commit == key.1)
        .collect()
}

/// The names of the results that are worse than the baseline beyond the threshold.
pub fn compare_rows(
    base: &[&Row],
    now: &[Measurement],
    threshold_pct: f64,
) -> (usize, Vec<String>) {
    let mut names = Vec::new();
    for m in now {
        let Some(b) = base.iter().find(|r| r.benchmark == m.name) else {
            continue;
        };
        if is_worse(b.value, m, threshold_pct) {
            names.push(m.name.clone());
        }
    }
    (names.len(), names)
}

fn limit(unit: Unit, threshold_pct: f64) -> f64 {
    // Sizes are deterministic: any real change matters.
    if unit == Unit::Bytes {
        0.5
    } else {
        threshold_pct
    }
}

fn change_pct(base: f64, now: f64) -> f64 {
    if base == 0.0 {
        0.0
    } else {
        (now - base) / base * 100.0
    }
}

fn is_worse(base: f64, m: &Measurement, threshold_pct: f64) -> bool {
    let pct = change_pct(base, m.value);
    let better = if m.unit.higher_is_better() {
        pct > 0.0
    } else {
        pct < 0.0
    };
    pct.abs() > limit(m.unit, threshold_pct) && !better
}

/// A table of baseline vs now with the change in percent; changes beyond the noise
/// threshold are flagged better/worse. Returns the table and the number of regressions.
pub fn compare(base: &[&Row], now: &[Measurement], threshold_pct: f64) -> (String, usize) {
    let by_name: BTreeMap<&str, &Row> = base.iter().map(|r| (r.benchmark.as_str(), *r)).collect();
    let mut rows = Vec::new();
    let mut worse = 0;
    for m in now {
        let Some(b) = by_name.get(m.name.as_str()) else {
            rows.push(vec![
                m.name.clone(),
                "-".into(),
                format_value(m.value),
                m.unit.as_str().into(),
                "".into(),
                "new".into(),
            ]);
            continue;
        };
        let pct = change_pct(b.value, m.value);
        let flag = if is_worse(b.value, m, threshold_pct) {
            worse += 1;
            "WORSE"
        } else if pct.abs() > limit(m.unit, threshold_pct) {
            "better"
        } else {
            ""
        };
        rows.push(vec![
            m.name.clone(),
            format_value(b.value),
            format_value(m.value),
            m.unit.as_str().into(),
            format!("{pct:+.1}%"),
            flag.into(),
        ]);
    }
    let table = table(&["benchmark", "base", "now", "unit", "±", ""], &rows);
    (table, worse)
}

/// A plain aligned table (Markdown-style, like the other runners).
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut width: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            width[i] = width[i].max(c.chars().count());
        }
    }
    let line = |cells: Vec<String>| {
        let mut s = String::from("|");
        for (i, c) in cells.iter().enumerate() {
            let pad = width[i] - c.chars().count();
            // Numbers right-aligned, text left-aligned.
            if i > 0
                && c.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_digit() || ch == '+' || ch == '-')
            {
                write!(s, " {}{c} |", " ".repeat(pad)).unwrap();
            } else {
                write!(s, " {c}{} |", " ".repeat(pad)).unwrap();
            }
        }
        s + "\n"
    };
    let mut out = line(headers.iter().map(|h| h.to_string()).collect());
    out.push('|');
    for w in &width {
        out.push_str(&format!("-{}-|", "-".repeat(*w)));
    }
    out.push('\n');
    for r in rows {
        out.push_str(&line(r.clone()));
    }
    out
}

/// `git rev-parse --short=10 HEAD`, with `-dirty` if tracked files changed.
pub fn commit() -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(env!("CARGO_MANIFEST_DIR"))
            .args(args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let hash = git(&["rev-parse", "--short=10", "HEAD"])
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let dirty =
        git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    if dirty { format!("{hash}-dirty") } else { hash }
}

/// `$DJS_BENCH_MACHINE`, else the hardware model (macOS) or host name, the architecture
/// and the number of CPUs, e.g. `Mac16,7-aarch64-18cpu`.
pub fn machine() -> String {
    if let Ok(m) = std::env::var("DJS_BENCH_MACHINE") {
        return m;
    }
    let cmd = |c: &str, args: &[&str]| {
        Command::new(c)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let model = cmd("sysctl", &["-n", "hw.model"])
        .or_else(|| cmd("uname", &["-n"]))
        .unwrap_or_else(|| "unknown".into());
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    format!("{model}-{}-{cpus}cpu", std::env::consts::ARCH).replace(['\t', ' '], "_")
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86400, secs % 86400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str, value: f64, unit: Unit) -> Measurement {
        Measurement {
            name: name.into(),
            unit,
            value,
            best: value,
            samples: 5,
        }
    }

    #[test]
    fn round_trip_and_baseline() {
        let dir = std::env::temp_dir().join(format!("djs-bench-scores-{}", std::process::id()));
        let path = dir.join("bench.tsv");
        let _ = std::fs::remove_dir_all(&dir);
        append(
            &path,
            "2026-01-01T00:00:00Z",
            "aaa",
            "box",
            &[m("x", 100.0, Unit::MBps), m("y", 2.5, Unit::Ms)],
        )
        .unwrap();
        append(
            &path,
            "2026-01-02T00:00:00Z",
            "bbb",
            "box",
            &[m("x", 110.0, Unit::MBps)],
        )
        .unwrap();
        append(
            &path,
            "2026-01-03T00:00:00Z",
            "ccc",
            "other",
            &[m("x", 1.0, Unit::MBps)],
        )
        .unwrap();
        let rows = load(&path).unwrap();
        assert_eq!(rows.len(), 4);
        assert!(std::fs::read_to_string(&path).unwrap().starts_with(HEADER));
        let last = baseline(&rows, "box", None);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].commit, "bbb");
        let first = baseline(&rows, "box", Some("aa"));
        assert_eq!(first.len(), 2);
        assert!(baseline(&rows, "box", Some("zzz")).is_empty());
        // An unknown machine falls back to all rows.
        assert_eq!(baseline(&rows, "new", None)[0].commit, "ccc");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn comparison_flags() {
        let rows = parse(&format!(
            "{HEADER}\nd\tc\tm\tfast\t100\tMB/s\nd\tc\tm\tslow\t10\tms\nd\tc\tm\tsize\t1000\tbytes\nd\tc\tm\tsame\t5\tms\n"
        ))
        .unwrap();
        let base: Vec<&Row> = rows.iter().collect();
        let now = [
            m("fast", 80.0, Unit::MBps),    // throughput down 20%: worse
            m("slow", 8.0, Unit::Ms),       // time down 20%: better
            m("size", 1010.0, Unit::Bytes), // +1% size: worse (0.5% threshold)
            m("same", 5.2, Unit::Ms),       // +4%: noise
            m("added", 1.0, Unit::Ms),
        ];
        let (t, worse) = compare(&base, &now, 10.0);
        assert_eq!(worse, 2, "{t}");
        assert!(
            t.contains("| fast      |  100 |    80 | MB/s  | -20.0% | WORSE  |"),
            "{t}"
        );
        assert!(t.contains("better"), "{t}");
        assert!(t.contains("new"), "{t}");
        assert!(parse("a\tb\n").is_err());
        let (n, names) = compare_rows(&base, &now, 10.0);
        assert_eq!(
            (n, names),
            (2, vec!["fast".to_string(), "size".to_string()])
        );
    }

    #[test]
    fn formatting_and_dates() {
        assert_eq!(format_value(222858.0), "222858");
        assert_eq!(format_value(1234.567), "1235");
        assert_eq!(format_value(12.3456), "12.35");
        assert_eq!(format_value(0.12345), "0.123");
        let d = now_utc();
        assert_eq!(d.len(), 20);
        assert!(d.starts_with("20") && d.ends_with('Z'));
    }
}
