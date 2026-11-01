//! The test262 checkout: locating it, discovering tests, loading the harness and
//! classifying tests into groups.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Config, Reason, TestFacts};
use crate::frontmatter::{self, Meta};
use crate::glob;
use crate::par;

/// The pinned test262 commit (D003).
pub const PINNED_COMMIT: &str = "7ab7fafa0003f73fc85c1b95d88094d33f7eb8bd";

/// The djs workspace root this binary was built from.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Finds the test262 checkout: `$DJS_TEST262`, else `vendor/test262` in the primary
/// checkout (the parent of git's common directory, shared by all worktrees).
pub fn locate_test262() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("DJS_TEST262") {
        return Ok(PathBuf::from(p));
    }
    let root = workspace_root();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(
            "cannot locate the primary checkout (git rev-parse failed); set DJS_TEST262".into(),
        );
    }
    let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    let primary = common.parent().ok_or("git common dir has no parent")?;
    Ok(primary.join("vendor/test262"))
}

/// Returns the checked-out commit of the test262 directory, if it is a git checkout.
pub fn checkout_commit(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub struct HarnessFile {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

pub struct Suite {
    pub root: PathBuf,
    /// Harness files by include name (`assert.js`, `sm/non262.js`, ...).
    pub harness: HashMap<String, HarnessFile>,
    /// Features implied by includes, from `harness/features.yml`.
    implied: HashMap<String, Vec<String>>,
}

pub struct TestCase {
    /// Path relative to `test/`, with `/` separators.
    pub path: String,
    pub meta: Meta,
    /// The test's features plus those implied by its includes.
    pub features: Vec<String>,
    /// Index into `Config::groups`.
    pub group: usize,
    pub reason: Reason,
    /// Set if the frontmatter could not be parsed or the file could not be read.
    pub error: Option<String>,
}

impl Suite {
    pub fn open(root: &Path) -> Result<Suite, String> {
        let harness_dir = root.join("harness");
        if !root.join("test").is_dir() || !harness_dir.is_dir() {
            return Err(format!(
                "{} is not a test262 checkout (run scripts/fetch-test262.sh or set DJS_TEST262)",
                root.display()
            ));
        }
        let mut harness = HashMap::new();
        for path in walk(&harness_dir, &|p| p.extension().is_some_and(|e| e == "js"))? {
            let name = rel_path(&harness_dir, &path);
            let source =
                fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            harness.insert(name.clone(), HarnessFile { name, path, source });
        }
        let mut implied = HashMap::new();
        if let Ok(text) = fs::read_to_string(harness_dir.join("features.yml")) {
            for line in text.lines() {
                // `testTypedArray.js: [TypedArray]`; some keys lack the `.js`.
                let Some((name, rest)) = line.split_once(':') else {
                    continue;
                };
                let Some(list) = rest
                    .trim()
                    .strip_prefix('[')
                    .and_then(|r| r.strip_suffix(']'))
                else {
                    continue;
                };
                let mut name = name.trim().to_string();
                if !name.ends_with(".js") {
                    name.push_str(".js");
                }
                let features = list
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect();
                implied.insert(name, features);
            }
        }
        Ok(Suite {
            root: root.to_path_buf(),
            harness,
            implied,
        })
    }

    pub fn test_dir(&self) -> PathBuf {
        self.root.join("test")
    }

    /// All test paths (relative to `test/`, sorted) that match any of `filters` (all
    /// tests if `filters` is empty). `_FIXTURE` files are module fixtures, not tests.
    pub fn discover(&self, filters: &[String]) -> Result<Vec<String>, String> {
        let test_dir = self.test_dir();
        let is_test = |p: &Path| {
            p.extension().is_some_and(|e| e == "js")
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().contains("_FIXTURE"))
        };
        let mut paths: Vec<String> = walk(&test_dir, &is_test)?
            .iter()
            .map(|p| rel_path(&test_dir, p))
            .collect();
        if !filters.is_empty() {
            let filters: Vec<String> = filters.iter().map(|f| normalize_filter(f)).collect();
            paths.retain(|p| filters.iter().any(|f| glob::path_filter_matches(f, p)));
        }
        paths.sort();
        Ok(paths)
    }

    /// Reads and classifies the given tests (in parallel, keeping the order).
    pub fn load(&self, config: &Config, paths: Vec<String>, jobs: usize) -> Vec<TestCase> {
        let test_dir = self.test_dir();
        par::map(paths, jobs, |path| {
            let file = test_dir.join(&path);
            let (meta, source, error) = match fs::read_to_string(&file) {
                Ok(source) => match frontmatter::parse_source(&source) {
                    Ok(meta) => (meta, source, None),
                    Err(e) => (
                        Meta::default(),
                        source,
                        Some(format!("bad frontmatter: {e}")),
                    ),
                },
                Err(e) => (
                    Meta::default(),
                    String::new(),
                    Some(format!("cannot read: {e}")),
                ),
            };
            let mut features = meta.features.clone();
            for inc in &meta.includes {
                for f in self.implied.get(inc).into_iter().flatten() {
                    if !features.contains(f) {
                        features.push(f.clone());
                    }
                }
            }
            let facts = TestFacts {
                path: &path,
                flags: &meta.flags,
                features: &features,
            };
            let (group, reason) = config.classify(&facts, &mut || {
                let mut texts = vec![source.clone()];
                texts.extend(
                    meta.includes
                        .iter()
                        .filter_map(|i| self.harness.get(i))
                        .map(|h| h.source.clone()),
                );
                texts
            });
            TestCase {
                path,
                meta,
                features,
                group,
                reason,
                error,
            }
        })
    }

    /// The harness files to evaluate before a (non-raw) test, in order: `assert.js`,
    /// `sta.js`, `doneprintHandle.js` for async tests, then the includes. Duplicates
    /// are dropped. Returns the name of the first missing file as an error.
    pub fn harness_for(&self, meta: &Meta) -> Result<Vec<&HarnessFile>, String> {
        let mut names: Vec<&str> = vec!["assert.js", "sta.js"];
        if meta.has_flag("async") {
            names.push("doneprintHandle.js");
        }
        for inc in &meta.includes {
            if !names.contains(&inc.as_str()) {
                names.push(inc);
            }
        }
        names
            .iter()
            .map(|n| {
                self.harness
                    .get(*n)
                    .ok_or_else(|| format!("missing harness file {n}"))
            })
            .collect()
    }
}

/// Accepts filters written relative to `test/`, relative to the checkout (`test/...`)
/// or as paths into the checkout (`.../test262/test/...`).
fn normalize_filter(f: &str) -> String {
    let f = f.replace('\\', "/");
    let f = match f.find("/test262/test/") {
        Some(i) => f[i + "/test262/test/".len()..].to_string(),
        None => f,
    };
    f.strip_prefix("test/")
        .unwrap_or(&f)
        .trim_start_matches("./")
        .to_string()
}

fn rel_path(base: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Recursively lists files under `dir` accepted by `keep`.
fn walk(dir: &Path, keep: &dyn Fn(&Path) -> bool) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", d.display()))?;
            let path = entry.path();
            let ty = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if ty.is_dir() {
                stack.push(path);
            } else if keep(&path) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_are_normalized() {
        assert_eq!(normalize_filter("built-ins/Array"), "built-ins/Array");
        assert_eq!(normalize_filter("test/built-ins/Array"), "built-ins/Array");
        assert_eq!(normalize_filter("./built-ins/Array"), "built-ins/Array");
        assert_eq!(
            normalize_filter("/x/vendor/test262/test/language/a.js"),
            "language/a.js"
        );
    }
}
