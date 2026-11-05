//! Repository paths in the guide's text (D2104): every mention of a file or directory of
//! the repository is a link to it on GitHub. The renderer links inline code that names
//! a tracked path (`crates/djs-core/src/heap/gc.rs`, `heap/cells.rs`, `js/src/`,
//! `scripts/gate.sh:40`) and paths in plain text; `check` fails on a mention that names a
//! path that does not exist or more than one, and on any path left unlinked in the
//! rendered page.

use std::collections::BTreeSet;
use std::path::Path;

/// The repository's tracked files and their directories: what GitHub shows.
pub struct RepoIndex {
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
    /// First path segments (`crates`, `js`, `scripts`, ...): a mention that starts with
    /// one of them means the repository, so it has to exist.
    top: BTreeSet<String>,
}

/// What a mention names.
#[derive(Debug, PartialEq)]
pub enum Resolved {
    /// A tracked path, whether it is a directory, and the fragment (`#L40`).
    Path(String, bool, String),
    /// Looks like a repository path but names nothing tracked.
    Missing,
    /// A partial path (`cells.rs`) that more than one tracked path ends with.
    Ambiguous(Vec<String>),
    /// Not a repository path (`@earendil-works/djs`, `built-ins/RegExp`, `x / y`).
    No,
}

impl RepoIndex {
    /// The tracked files (`git ls-files`); without git, the files on disk outside the
    /// ignored build and checkout directories.
    pub fn load(repo: &Path) -> RepoIndex {
        let listed = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["ls-files", "-z"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
        let files = listed.unwrap_or_else(|| {
            let mut v = Vec::new();
            walk(repo, "", &mut v);
            v
        });
        RepoIndex::new(files)
    }

    pub fn new(files: impl IntoIterator<Item = String>) -> RepoIndex {
        let mut ix = RepoIndex {
            files: BTreeSet::new(),
            dirs: BTreeSet::new(),
            top: BTreeSet::new(),
        };
        for f in files {
            // Work tracking is not part of what the guide may point at (D2102).
            if f.starts_with("work/") {
                continue;
            }
            let mut at = 0;
            while let Some(i) = f[at..].find('/') {
                ix.dirs.insert(f[..at + i].to_string());
                at += i + 1;
            }
            ix.top
                .insert(f.split('/').next().unwrap_or_default().to_string());
            ix.files.insert(f);
        }
        ix
    }

    /// Resolves inline code. A full path must exist; a partial one (`heap/cells.rs`,
    /// `cells.rs`, `heap/`) must be the end of exactly one tracked path. Markdown files
    /// are left to the self-containment rule (`check_prose`).
    pub fn code(&self, t: &str) -> Resolved {
        let (p, frag) = split_line(t.trim());
        if !p
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c))
            || p.starts_with(['/', '.'])
            || p.ends_with(".md")
        {
            return Resolved::No;
        }
        let dir = p.ends_with('/');
        let p = p.trim_end_matches('/');
        let file_like = p.contains('/') || dir || has_extension(p);
        if !file_like || p.is_empty() {
            return Resolved::No;
        }
        self.find(p, dir, frag)
    }

    /// An exact path, else the one tracked path that ends with `p`.
    fn find(&self, p: &str, dir: bool, frag: String) -> Resolved {
        if let Some(r) = self.exact(p, &frag) {
            return r;
        }
        let suffix = format!("/{p}");
        let mut hits: Vec<String> = self
            .files
            .iter()
            .filter(|f| !dir && f.ends_with(&suffix))
            .chain(self.dirs.iter().filter(|d| d.ends_with(&suffix)))
            .cloned()
            .collect();
        hits.sort();
        hits.dedup();
        match hits.len() {
            // A path from the repository's root that is not there.
            0 if p.contains('/') && self.top.contains(p.split('/').next().unwrap_or_default()) => {
                Resolved::Missing
            }
            0 => Resolved::No,
            1 => {
                let h = hits.pop().unwrap_or_default();
                let is_dir = self.dirs.contains(&h);
                Resolved::Path(h, is_dir, frag)
            }
            _ => Resolved::Ambiguous(hits),
        }
    }

    /// Resolves a word of plain text: only paths that start at a top-level directory count
    /// there (`crates/x/y.rs`, `builtins/fast.rs` for the one file it can be), as prose
    /// has slashes of its own (`and/or`).
    pub fn text(&self, w: &str) -> Resolved {
        let w = w.trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
        // A glob (`builtins/*.ts`) names files of its directory.
        if let Some(star) = w.find('*') {
            let dir = w[..star]
                .trim_end_matches(|c| c != '/')
                .trim_end_matches('/');
            return if self.dirs.contains(dir) {
                Resolved::Path(dir.to_string(), true, String::new())
            } else {
                Resolved::No
            };
        }
        let (p, frag) = split_line(w);
        let p = p.trim_end_matches('/');
        if !p.contains('/')
            || p.ends_with(".md")
            || !self.top.contains(p.split('/').next().unwrap_or_default())
            || !p
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c))
        {
            return Resolved::No;
        }
        self.find(p, false, frag)
    }

    fn exact(&self, p: &str, frag: &str) -> Option<Resolved> {
        if self.files.contains(p) {
            Some(Resolved::Path(p.to_string(), false, frag.to_string()))
        } else if self.dirs.contains(p) {
            Some(Resolved::Path(p.to_string(), true, String::new()))
        } else {
            None
        }
    }
}

/// `path:40` and `path:40-52`: the path and GitHub's line fragment.
fn split_line(t: &str) -> (&str, String) {
    if let Some((p, l)) = t.rsplit_once(':') {
        let nums: Vec<&str> = l.split('-').collect();
        if nums.len() <= 2
            && nums
                .iter()
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            let frag = nums
                .iter()
                .map(|n| format!("L{n}"))
                .collect::<Vec<_>>()
                .join("-");
            return (p, format!("#{frag}"));
        }
    }
    (t, String::new())
}

fn has_extension(p: &str) -> bool {
    matches!(
        p.rsplit_once('.').map(|(_, e)| e),
        Some(
            "rs" | "ts"
                | "js"
                | "mjs"
                | "sh"
                | "py"
                | "toml"
                | "txt"
                | "tsv"
                | "json"
                | "css"
                | "html"
                | "conf"
        )
    )
}

fn walk(root: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(root.join(rel)) else {
        return;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            ".git" | "target" | "vendor" | "node_modules" | ".worktrees"
        ) {
            continue;
        }
        let r = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        if e.path().is_dir() {
            walk(root, &r, out);
        } else {
            out.push(r);
        }
    }
}

/// Repository paths in rendered HTML that are not inside a link (code blocks exempt):
/// the check behind the renderer's linking, so that no path of any origin (headings,
/// generated HTML) slips through.
pub fn unlinked(html: &str, ix: &RepoIndex) -> Vec<String> {
    let mut found = Vec::new();
    let (mut a, mut pre, mut code) = (0i32, 0i32, 0i32);
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(tag) = rest.strip_prefix('<') {
            let end = tag.find('>').map_or(tag.len(), |i| i + 1);
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            let d = if tag.starts_with('/') { -1 } else { 1 };
            match name.as_str() {
                "a" => a += d,
                "pre" => pre += d,
                "code" => code += d,
                // Not text: scripts, styles and inline SVG.
                "script" | "style" | "svg" if d == 1 => {
                    let close = format!("</{name}>");
                    let skip = rest.find(&close).map_or(rest.len(), |i| i + close.len());
                    rest = &rest[skip..];
                    continue;
                }
                _ => {}
            }
            rest = &tag[end.min(tag.len())..];
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        let text = rest[..end]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&");
        rest = &rest[end..];
        if a > 0 || pre > 0 {
            continue;
        }
        let hit = |r: Resolved| matches!(r, Resolved::Path(..));
        if code > 0 {
            if hit(ix.code(&text)) {
                found.push(text.trim().to_string());
            } else {
                // A command or glob with a path in it (`node js/pi/pi-tests.ts`).
                for w in text.split_whitespace() {
                    if hit(ix.text(w)) {
                        found.push(w.to_string());
                    }
                }
            }
        } else {
            for w in text.split(|c: char| c.is_whitespace() || "()[]{},;\"'“”‘’".contains(c))
            {
                if hit(ix.text(w)) {
                    found.push(w.to_string());
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ix() -> RepoIndex {
        RepoIndex::new(
            [
                "crates/djs-core/src/heap/cells.rs",
                "crates/djs-core/src/heap/gc.rs",
                "crates/djs-core/src/lib.rs",
                "crates/djs-docs/src/lib.rs",
                "js/src/sandbox.ts",
                "scripts/gate.sh",
                "work/issues/0001-x.md",
                "README.md",
            ]
            .map(String::from),
        )
    }

    fn path(p: &str, dir: bool, frag: &str) -> Resolved {
        Resolved::Path(p.into(), dir, frag.into())
    }

    #[test]
    fn code_mentions() {
        let ix = ix();
        assert_eq!(
            ix.code("scripts/gate.sh"),
            path("scripts/gate.sh", false, "")
        );
        assert_eq!(
            ix.code("scripts/gate.sh:40"),
            path("scripts/gate.sh", false, "#L40")
        );
        assert_eq!(ix.code("js/src/"), path("js/src", true, ""));
        assert_eq!(ix.code("js/src"), path("js/src", true, ""));
        assert_eq!(
            ix.code("heap/cells.rs"),
            path("crates/djs-core/src/heap/cells.rs", false, "")
        );
        assert_eq!(
            ix.code("gc.rs"),
            path("crates/djs-core/src/heap/gc.rs", false, "")
        );
        assert_eq!(ix.code("heap/"), path("crates/djs-core/src/heap", true, ""));
        assert_eq!(
            ix.code("lib.rs"),
            Resolved::Ambiguous(vec![
                "crates/djs-core/src/lib.rs".into(),
                "crates/djs-docs/src/lib.rs".into()
            ])
        );
        assert_eq!(ix.code("crates/djs-core/src/gone.rs"), Resolved::Missing);
        for no in [
            "@earendil-works/djs",
            "built-ins/RegExp",
            "a / b",
            "x.y",
            "README.md",
            "work/issues/0001-x.md",
            "other.rs",
            "/etc/x",
        ] {
            assert_eq!(ix.code(no), Resolved::No, "{no}");
        }
    }

    #[test]
    fn text_mentions() {
        let ix = ix();
        assert_eq!(
            ix.text("scripts/gate.sh."),
            path("scripts/gate.sh", false, "")
        );
        assert_eq!(ix.text("crates/nope"), Resolved::Missing);
        assert_eq!(ix.text("js/src/*.ts"), path("js/src", true, ""));
        assert_eq!(ix.text("crates/*/src"), path("crates", true, ""));
        assert_eq!(ix.text("src/heap/gc.rs"), Resolved::No);
        assert_eq!(
            ix.text("js/src/sandbox.ts:7"),
            path("js/src/sandbox.ts", false, "#L7")
        );
        for no in ["and/or", "gc.rs", "heap/cells.rs", "README.md"] {
            assert_eq!(ix.text(no), Resolved::No, "{no}");
        }
    }

    #[test]
    fn unlinked_paths_are_found() {
        let html = "<p>See <code>gc.rs</code>, <a href=\"x\"><code>scripts/gate.sh</code></a> and js/src/sandbox.ts.</p>\
                    <pre><code>crates/djs-core/src/lib.rs</code></pre><h2>In <code>heap/</code></h2>\
                    <p><code>node js/src/sandbox.ts</code>, <code>bash <a>scripts/gate.sh</a></code></p>";
        assert_eq!(
            unlinked(html, &ix()),
            ["gc.rs", "js/src/sandbox.ts.", "heap/", "js/src/sandbox.ts"]
        );
    }
}
