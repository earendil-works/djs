//! Corpus cases: discovery and the per-case metadata in leading comments.
//!
//! A case is `tests/corpus/<area>/.../<name>.{js,ts}` with `<name>.expected` next to it.
//! Files directly in `tests/corpus/` (the prelude, the failures file) are not cases.
//! Metadata lives in the leading `//` comment lines, one directive per line:
//!
//! ```text
//! // difftest: skip-djs <reason>   do not run with djs (a non-goal such as eval)
//! // difftest: ts                  parse a .js file as TypeScript
//! // difftest: fuel <n>            djs fuel limit for this case
//! // difftest: timeout <secs>      wall-clock limit for this case
//! // difftest: djs-expect <a> => <b>  an expected difference: with djs, `.expected` lines
//!                                     equal to `a` must read `b` (D013: stack overflow)
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use djs_test262::glob;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub skip_djs: Option<String>,
    pub ts: bool,
    pub fuel: Option<u64>,
    pub timeout: Option<Duration>,
    /// `djs-expect` directives: transcript lines that differ from the oracle on purpose.
    pub djs_expect: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Case {
    /// Path relative to the corpus root, with `/` separators (`console/basic.js`).
    pub name: String,
    pub file: PathBuf,
    pub expected_file: PathBuf,
    pub meta: Meta,
}

impl Case {
    /// The area: the first directory below the corpus root.
    pub fn area(&self) -> &str {
        self.name.split('/').next().unwrap_or("")
    }
}

/// Parses the `// difftest:` directives from the leading comment lines of a case.
pub fn parse_meta(source: &str, is_ts_file: bool) -> Result<Meta, String> {
    let mut meta = Meta {
        ts: is_ts_file,
        ..Meta::default()
    };
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix("//") else {
            break;
        };
        let Some(directive) = comment.trim().strip_prefix("difftest:") else {
            continue;
        };
        let directive = directive.trim();
        let (word, rest) = directive
            .split_once(char::is_whitespace)
            .unwrap_or((directive, ""));
        let rest = rest.trim();
        match word {
            "skip-djs" if !rest.is_empty() => meta.skip_djs = Some(rest.to_string()),
            "skip-djs" => return Err("`skip-djs` needs a reason".into()),
            "ts" if rest.is_empty() => meta.ts = true,
            "fuel" => meta.fuel = Some(rest.parse().map_err(|_| format!("bad fuel `{rest}`"))?),
            "timeout" => {
                let secs: f64 = rest.parse().map_err(|_| format!("bad timeout `{rest}`"))?;
                meta.timeout = Some(Duration::from_secs_f64(secs));
            }
            "djs-expect" => {
                let (a, b) = rest.split_once(" => ").ok_or_else(|| {
                    format!("`djs-expect` needs `<oracle line> => <djs line>`: `{rest}`")
                })?;
                meta.djs_expect
                    .push((a.trim().to_string(), b.trim().to_string()));
            }
            _ => return Err(format!("unknown directive `difftest: {directive}`")),
        }
    }
    Ok(meta)
}

/// All cases under `root` matching any of `filters` (all if empty), sorted by name.
pub fn discover(root: &Path, filters: &[String]) -> Result<Vec<Case>, String> {
    let mut files = Vec::new();
    let entries = fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            walk(&path, &mut files)?;
        }
    }
    let mut cases = Vec::new();
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(&file);
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if !filters.is_empty()
            && !filters
                .iter()
                .any(|f| glob::path_filter_matches(f.trim_end_matches('/'), &name))
        {
            continue;
        }
        let source = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let is_ts = name.ends_with(".ts");
        let meta = parse_meta(&source, is_ts).map_err(|e| format!("{name}: {e}"))?;
        let expected_file = file.with_extension("expected");
        cases.push(Case {
            name,
            file,
            expected_file,
            meta,
        });
    }
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    let mut seen = std::collections::HashMap::new();
    for c in &cases {
        if let Some(other) = seen.insert(&c.expected_file, &c.name) {
            return Err(format!(
                "{other} and {} would share one .expected file",
                c.name
            ));
        }
    }
    Ok(cases)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "js" || e == "ts") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives() {
        let src = "// Some description.\n// difftest: skip-djs needs eval\n//difftest: fuel 1000\n\n// difftest: timeout 2.5\nlet x = 1;\n// difftest: ts\n";
        let m = parse_meta(src, false).unwrap();
        assert_eq!(m.skip_djs.as_deref(), Some("needs eval"));
        assert_eq!(m.fuel, Some(1000));
        assert_eq!(m.timeout, Some(Duration::from_millis(2500)));
        // Directives after the first code line are ignored.
        assert!(!m.ts);
        assert!(parse_meta("// difftest: ts\n", false).unwrap().ts);
        assert!(parse_meta("", true).unwrap().ts);
        let m = parse_meta("// difftest: djs-expect A: x => B: y\n", false).unwrap();
        assert_eq!(m.djs_expect, [("A: x".to_string(), "B: y".to_string())]);
    }

    #[test]
    fn bad_directives() {
        assert!(parse_meta("// difftest: skip-djs\n", false).is_err());
        assert!(parse_meta("// difftest: frobnicate\n", false).is_err());
        assert!(parse_meta("// difftest: fuel lots\n", false).is_err());
        assert!(parse_meta("// difftest: djs-expect no arrow\n", false).is_err());
    }
}
