//! Test grouping (`tests/test262.toml`): every test belongs to exactly one group.
//!
//! Groups are listed as `[[group]]` tables and checked in file order; the first group
//! with a matching rule wins. Tests that match no group are `active`. A group's `kind`
//! is `skip` (never run, excluded from the score) or `later` (run, reported
//! separately). See TESTING.md.

use std::path::Path;

use crate::glob;
use crate::scan;
use crate::toml::{self, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Active,
    Later,
    Skip,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Active => "active",
            Kind::Later => "later",
            Kind::Skip => "skip",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    /// Matches if the test has any of these flags.
    pub flags: Vec<String>,
    /// Directory prefixes (or file paths) relative to `test/`, or globs (`glob`).
    pub paths: Vec<String>,
    /// Feature globs (`*` wildcard), matched against the test's features, including
    /// those implied by its includes (`harness/features.yml`).
    pub features: Vec<String>,
    /// Source scan patterns (see `scan`), checked against the test and its includes.
    pub scan: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub kind: Kind,
    pub description: String,
    pub rules: Rules,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// The configured groups followed by the implicit `active` group (always last).
    pub groups: Vec<Group>,
}

/// What a group rule needs to know about a test.
pub struct TestFacts<'a> {
    pub path: &'a str,
    pub flags: &'a [String],
    pub features: &'a [String],
}

/// Which rule put a test into its group (for `list --why`-style diagnostics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    Flag(String),
    Path(String),
    Feature(String),
    Scan(String),
    Default,
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason::Flag(s) => write!(f, "flag {s}"),
            Reason::Path(s) => write!(f, "path {s}"),
            Reason::Feature(s) => write!(f, "feature {s}"),
            Reason::Scan(s) => write!(f, "scan `{s}`"),
            Reason::Default => write!(f, "default"),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        let tables = toml::parse(text)?;
        let mut groups = Vec::new();
        for table in &tables {
            if table.name.is_empty() {
                if let Some((k, _)) = table.entries.first() {
                    return Err(format!("unexpected top-level key `{k}`"));
                }
                continue;
            }
            if table.name != "group" || !table.is_array_item {
                return Err(format!(
                    "line {}: unknown table `{}` (expected [[group]])",
                    table.line, table.name
                ));
            }
            groups.push(parse_group(table)?);
        }
        for (i, g) in groups.iter().enumerate() {
            if g.name == "active" || groups[..i].iter().any(|h| h.name == g.name) {
                return Err(format!("group name `{}` is reserved or used twice", g.name));
            }
        }
        groups.push(Group {
            name: "active".into(),
            kind: Kind::Active,
            description: "everything not matched by another group".into(),
            rules: Rules::default(),
        });
        Ok(Config { groups })
    }

    pub fn group_index(&self, name: &str) -> Option<usize> {
        self.groups.iter().position(|g| g.name == name)
    }

    /// True if any group needs the source scan (so callers can skip reading sources).
    pub fn needs_scan(&self) -> bool {
        self.groups.iter().any(|g| !g.rules.scan.is_empty())
    }

    /// Returns the index of the test's group and the rule that matched. `sources` is
    /// called at most once, lazily, and returns the comment-stripped text of the test
    /// and its includes.
    pub fn classify(
        &self,
        facts: &TestFacts<'_>,
        sources: &mut dyn FnMut() -> Vec<String>,
    ) -> (usize, Reason) {
        let mut stripped: Option<Vec<String>> = None;
        for (i, g) in self.groups.iter().enumerate() {
            let r = &g.rules;
            if let Some(f) = r.flags.iter().find(|f| facts.flags.contains(f)) {
                return (i, Reason::Flag(f.clone()));
            }
            if let Some(p) = r
                .paths
                .iter()
                .find(|p| glob::path_filter_matches(p, facts.path))
            {
                return (i, Reason::Path(p.clone()));
            }
            for pat in &r.features {
                if let Some(f) = facts.features.iter().find(|f| glob::matches(pat, f)) {
                    return (i, Reason::Feature(f.clone()));
                }
            }
            if !r.scan.is_empty() {
                let texts = stripped.get_or_insert_with(|| {
                    sources().iter().map(|s| scan::strip_comments(s)).collect()
                });
                for pat in &r.scan {
                    if texts.iter().any(|t| scan::contains_pattern(t, pat)) {
                        return (i, Reason::Scan(pat.clone()));
                    }
                }
            }
        }
        (self.groups.len() - 1, Reason::Default)
    }
}

fn parse_group(table: &toml::Table) -> Result<Group, String> {
    let at = |msg: String| format!("[[group]] at line {}: {msg}", table.line);
    let mut name = None;
    let mut kind = None;
    let mut description = None;
    let mut rules = Rules::default();
    for (key, value) in &table.entries {
        match (key.as_str(), value) {
            ("name", Value::Str(s)) => name = Some(s.clone()),
            ("description", Value::Str(s)) => description = Some(s.clone()),
            ("kind", Value::Str(s)) => {
                kind = Some(match s.as_str() {
                    "skip" => Kind::Skip,
                    "later" => Kind::Later,
                    other => {
                        return Err(at(format!(
                            "kind must be \"skip\" or \"later\", not \"{other}\""
                        )));
                    }
                })
            }
            ("flags", Value::Array(a)) => rules.flags = a.clone(),
            ("paths", Value::Array(a)) => {
                rules.paths = a
                    .iter()
                    .map(|p| p.trim_end_matches('/').to_string())
                    .collect()
            }
            ("features", Value::Array(a)) => rules.features = a.clone(),
            ("scan", Value::Array(a)) => rules.scan = a.clone(),
            _ => return Err(at(format!("unknown key or wrong type: `{key}`"))),
        }
    }
    let name = name.ok_or_else(|| at("missing `name`".into()))?;
    Ok(Group {
        kind: kind.ok_or_else(|| at(format!("group `{name}` is missing `kind`")))?,
        description: description
            .ok_or_else(|| at(format!("group `{name}` is missing `description`")))?,
        name,
        rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = r#"
[[group]]
name = "skip"
kind = "skip"
description = "never"
flags = ["noStrict"]
paths = ["intl402", "built-ins/Temporal/", "built-ins/Date/*.js"]
features = ["Intl-*"]

[[group]]
name = "scan-eval"
kind = "skip"
description = "dynamic code"
scan = ["eval("]

[[group]]
name = "later"
kind = "later"
description = "not yet"
features = ["generators"]
"#;

    fn classify(
        cfg: &Config,
        path: &str,
        flags: &[&str],
        features: &[&str],
        src: &str,
    ) -> (String, Reason) {
        let flags: Vec<String> = flags.iter().map(|s| s.to_string()).collect();
        let features: Vec<String> = features.iter().map(|s| s.to_string()).collect();
        let facts = TestFacts {
            path,
            flags: &flags,
            features: &features,
        };
        let (i, why) = cfg.classify(&facts, &mut || vec![src.to_string()]);
        (cfg.groups[i].name.clone(), why)
    }

    #[test]
    fn first_match_wins() {
        let cfg = Config::parse(CFG).unwrap();
        assert_eq!(cfg.groups.len(), 4);
        assert!(cfg.needs_scan());
        let g = |p, fl: &[&str], fe: &[&str], s| classify(&cfg, p, fl, fe, s).0;
        assert_eq!(g("intl402/x.js", &[], &[], ""), "skip");
        assert_eq!(g("built-ins/Temporal/x.js", &[], &[], ""), "skip");
        assert_eq!(g("built-ins/Array/x.js", &["noStrict"], &[], ""), "skip");
        assert_eq!(g("built-ins/Date/x.js", &[], &[], ""), "skip");
        assert_eq!(g("built-ins/Date/now/x.js", &[], &[], ""), "active");
        assert_eq!(
            g("built-ins/Array/x.js", &[], &["Intl-enumeration"], ""),
            "skip"
        );
        assert_eq!(
            g("built-ins/Array/x.js", &[], &["generators"], "eval(1)"),
            "scan-eval"
        );
        assert_eq!(
            g("built-ins/Array/x.js", &[], &["generators"], "// eval(1)"),
            "later"
        );
        assert_eq!(g("built-ins/Array/x.js", &[], &[], "1"), "active");
        assert_eq!(
            classify(&cfg, "a.js", &[], &[], "eval(1)").1,
            Reason::Scan("eval(".into())
        );
    }

    #[test]
    fn rejects_bad_config() {
        assert!(Config::parse("[[group]]\nname = \"a\"\nkind = \"skip\"\n").is_err());
        assert!(
            Config::parse("[[group]]\nname = \"a\"\nkind = \"maybe\"\ndescription = \"\"\n")
                .is_err()
        );
        assert!(
            Config::parse("[[group]]\nname = \"active\"\nkind = \"skip\"\ndescription = \"\"\n")
                .is_err()
        );
        assert!(
            Config::parse(
                "[[group]]\nname = \"a\"\nkind = \"skip\"\ndescription = \"\"\ndirs = []\n"
            )
            .is_err()
        );
        assert!(Config::parse("[grp]\n").is_err());
    }
}
