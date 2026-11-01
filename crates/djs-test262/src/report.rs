//! Summary tables (Markdown, so they paste straight into the board).

use std::collections::BTreeMap;

use crate::config::{Config, Kind};
use crate::engine::Verdict;
use crate::runner::Outcome;
use crate::suite::TestCase;

/// Counts for one row: tests per kind, and how many of them ran and passed.
#[derive(Default, Clone)]
struct Tally {
    total: [usize; 3],
    ran: [usize; 3],
    pass: [usize; 3],
}

fn kind_index(k: Kind) -> usize {
    match k {
        Kind::Active => 0,
        Kind::Later => 1,
        Kind::Skip => 2,
    }
}

impl Tally {
    fn add(&mut self, kind: Kind, outcome: Option<&Outcome>) {
        let k = kind_index(kind);
        self.total[k] += 1;
        if let Some(o) = outcome {
            self.ran[k] += 1;
            if o.verdict == Verdict::Pass {
                self.pass[k] += 1;
            }
        }
    }
}

/// Pass rate rounded down to tenths, so 100.0% means that everything passed.
fn pct(pass: usize, ran: usize) -> String {
    if ran == 0 {
        return String::new();
    }
    let tenths = pass * 1000 / ran;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// Renders a Markdown table; numeric-looking columns are right-aligned.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let ncol = headers.len();
    let mut width: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            width[i] = width[i].max(cell.chars().count());
        }
    }
    let right: Vec<bool> = (0..ncol)
        .map(|i| {
            i > 0
                && rows.iter().all(|r| {
                    r[i].is_empty()
                        || r[i]
                            .chars()
                            .all(|c| c.is_ascii_digit() || c == '.' || c == '%')
                })
        })
        .collect();
    let fmt_row = |cells: &[String]| {
        let mut s = String::from("|");
        for (i, cell) in cells.iter().enumerate() {
            let pad = width[i] - cell.chars().count();
            if right[i] {
                s.push_str(&format!(" {}{} |", " ".repeat(pad), cell));
            } else {
                s.push_str(&format!(" {}{} |", cell, " ".repeat(pad)));
            }
        }
        s.push('\n');
        s
    };
    let mut out = fmt_row(&headers.iter().map(|h| h.to_string()).collect::<Vec<_>>());
    out.push('|');
    for i in 0..ncol {
        let dashes = "-".repeat(width[i]);
        out.push_str(&if right[i] {
            format!("-{dashes}:|")
        } else {
            format!("-{dashes}-|")
        });
    }
    out.push('\n');
    for row in rows {
        out.push_str(&fmt_row(row));
    }
    out
}

/// `path` truncated to its first `depth` components (directories only).
fn dir_key(path: &str, depth: usize) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let n = depth.min(parts.len().saturating_sub(1)).max(1);
    parts[..n.min(parts.len())].join("/")
}

pub struct ReportOptions {
    pub depth: usize,
    pub features: bool,
}

/// The group table, the per-directory table and optionally the per-feature table.
/// `outcomes` is parallel to `tests` (None for tests that were not run).
pub fn summary(
    config: &Config,
    tests: &[TestCase],
    outcomes: &[Option<&Outcome>],
    opts: &ReportOptions,
) -> String {
    let ran_any = outcomes.iter().any(|o| o.is_some());
    let mut by_group = vec![Tally::default(); config.groups.len()];
    let mut by_dir: BTreeMap<String, Tally> = BTreeMap::new();
    let mut by_feature: BTreeMap<String, Tally> = BTreeMap::new();
    for (t, o) in tests.iter().zip(outcomes) {
        let kind = config.groups[t.group].kind;
        by_group[t.group].add(kind, *o);
        by_dir
            .entry(dir_key(&t.path, opts.depth))
            .or_default()
            .add(kind, *o);
        if opts.features {
            for f in &t.features {
                by_feature.entry(f.clone()).or_default().add(kind, *o);
            }
        }
    }

    let mut out = String::new();
    let mut rows = Vec::new();
    for (g, tally) in config.groups.iter().zip(&by_group) {
        let k = kind_index(g.kind);
        let mut row = vec![
            g.name.clone(),
            g.kind.as_str().to_string(),
            tally.total[k].to_string(),
        ];
        if ran_any {
            let (ran, pass) = (tally.ran[k], tally.pass[k]);
            if ran == 0 {
                row.extend(["".into(), "".into(), "".into(), "".into()]);
            } else {
                row.extend([
                    ran.to_string(),
                    pass.to_string(),
                    (ran - pass).to_string(),
                    pct(pass, ran),
                ]);
            }
        }
        rows.push(row);
    }
    let headers: &[&str] = if ran_any {
        &["group", "kind", "tests", "ran", "pass", "fail", "pass %"]
    } else {
        &["group", "kind", "tests"]
    };
    out.push_str(&table(headers, &rows));

    for (title, map) in [("dir", &by_dir), ("feature", &by_feature)] {
        if map.is_empty() {
            continue;
        }
        out.push('\n');
        let mut rows = Vec::new();
        for (key, t) in map {
            let mut row = vec![key.clone()];
            for k in [0, 1] {
                row.push(t.total[k].to_string());
                if ran_any {
                    row.push(if t.ran[k] == 0 {
                        String::new()
                    } else {
                        t.pass[k].to_string()
                    });
                    row.push(pct(t.pass[k], t.ran[k]));
                }
            }
            row.push(t.total[2].to_string());
            rows.push(row);
        }
        let headers: Vec<&str> = if ran_any {
            vec![title, "active", "pass", "%", "later", "pass", "%", "skip"]
        } else {
            vec![title, "active", "later", "skip"]
        };
        out.push_str(&table(&headers, &rows));
    }
    out
}

/// A cluster key for a failure reason: the values in it (`«...»` and numbers) replaced,
/// so that the same failure in different tests groups together.
pub fn cluster_key(reason: &str) -> String {
    let mut out = String::new();
    let mut in_value = false;
    let mut chars = reason.chars().peekable();
    let mut prev: Option<char> = None;
    while let Some(c) = chars.next() {
        match c {
            '«' => {
                in_value = true;
                out.push_str("«…»");
            }
            '»' => in_value = false,
            _ if in_value => {}
            // A number on its own (not part of a word like `Test262Error`).
            c if c.is_ascii_digit() && !prev.is_some_and(|p| p.is_alphanumeric() || p == '_') => {
                while chars
                    .peek()
                    .is_some_and(|d| d.is_ascii_digit() || *d == '.')
                {
                    chars.next();
                }
                out.push('N');
            }
            c => out.push(c),
        }
        prev = Some(c);
    }
    out
}

/// The `top` largest clusters of `(test, reason)` failures as a table with examples.
pub fn clusters(failures: &[(&str, &str)], top: usize) -> String {
    let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (test, reason) in failures {
        groups.entry(cluster_key(reason)).or_default().push(test);
    }
    let mut groups: Vec<(String, Vec<&str>)> = groups.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    let rows: Vec<Vec<String>> = groups
        .iter()
        .take(top)
        .map(|(k, tests)| {
            let mut ex = tests.iter().take(2).copied().collect::<Vec<_>>().join(", ");
            if tests.len() > 2 {
                ex.push_str(", ...");
            }
            vec![tests.len().to_string(), k.clone(), ex]
        })
        .collect();
    let mut out = table(&["tests", "cluster", "examples"], &rows);
    if groups.len() > top {
        out.push_str(&format!(
            "({} smaller clusters not shown)\n",
            groups.len() - top
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_keys() {
        assert_eq!(
            cluster_key("Test262Error: Expected SameValue(«1», «\"a\"») to be true"),
            "Test262Error: Expected SameValue(«…», «…») to be true"
        );
        assert_eq!(cluster_key("expected 10 got 200"), "expected N got N");
        let t = clusters(&[("a.js", "x 1"), ("b.js", "x 2"), ("c.js", "y")], 1);
        assert!(t.contains("| 2     | x N     | a.js, b.js |"), "{t}");
        assert!(t.contains("(1 smaller clusters not shown)"));
    }

    #[test]
    fn dir_keys() {
        assert_eq!(dir_key("built-ins/Array/from/x.js", 1), "built-ins");
        assert_eq!(dir_key("built-ins/Array/from/x.js", 2), "built-ins/Array");
        assert_eq!(dir_key("built-ins/Array/x.js", 5), "built-ins/Array");
        assert_eq!(dir_key("harness/x.js", 2), "harness");
    }

    #[test]
    fn percentages() {
        assert_eq!(pct(2679, 2680), "99.9%");
        assert_eq!(pct(5, 5), "100.0%");
        assert_eq!(pct(0, 3), "0.0%");
        assert_eq!(pct(0, 0), "");
    }

    #[test]
    fn markdown_table() {
        let t = table(
            &["dir", "n"],
            &[
                vec!["a".into(), "10".into()],
                vec!["bbb".into(), "2".into()],
            ],
        );
        assert_eq!(
            t,
            "| dir |  n |\n|-----|---:|\n| a   | 10 |\n| bbb |  2 |\n"
        );
    }
}
