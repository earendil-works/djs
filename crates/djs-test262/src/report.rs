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

fn pct(pass: usize, ran: usize) -> String {
    if ran == 0 {
        String::new()
    } else {
        format!("{:.1}%", pass as f64 * 100.0 / ran as f64)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_keys() {
        assert_eq!(dir_key("built-ins/Array/from/x.js", 1), "built-ins");
        assert_eq!(dir_key("built-ins/Array/from/x.js", 2), "built-ins/Array");
        assert_eq!(dir_key("built-ins/Array/x.js", 5), "built-ins/Array");
        assert_eq!(dir_key("harness/x.js", 2), "harness");
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
