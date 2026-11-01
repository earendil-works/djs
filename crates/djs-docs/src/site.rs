//! The static site: SUMMARY.md navigation, the page template, assets and the live
//! examples' files (djs-wasm and the host package with its types stripped).

use std::fmt::Write as _;
use std::path::Path;

use crate::render::{self, Page, esc, html_name};
use crate::{Guide, page_set, repo};

const STYLE: &str = include_str!("../assets/style.css");
const LIVE: &str = include_str!("../assets/live.js");
const STRIP: &str = include_str!("../assets/strip.mjs");

/// One line of SUMMARY.md: `# Part`, `- [Title](page.md)`, or a draft `- [Title]()`.
pub struct Entry {
    pub depth: usize,
    pub title: String,
    pub path: Option<String>,
    pub part: bool,
}

pub fn summary(s: &str) -> Vec<Entry> {
    let mut v = Vec::new();
    for line in s.lines() {
        if let Some(p) = line.strip_prefix("# ") {
            v.push(Entry {
                depth: 0,
                title: p.trim().into(),
                path: None,
                part: true,
            });
        } else if let Some(i) = line.find("- [") {
            let Some((title, rest)) = line[i + 3..].split_once("](") else {
                continue;
            };
            let path = rest.trim_end().trim_end_matches(')');
            v.push(Entry {
                depth: i / 2,
                title: title.into(),
                path: (!path.is_empty()).then(|| path.into()),
                part: false,
            });
        }
    }
    v
}

/// Renders the guide's pages and the decision log: (source name, page).
pub fn render_all(g: &mut Guide) -> Vec<(String, Page)> {
    let pages = page_set(g);
    let repo = repo();
    let mut out = Vec::new();
    let mut all: Vec<(String, &str, bool)> = g
        .pages
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str(), true))
        .collect();
    all.push(("decisions.md".into(), g.decisions_md.as_str(), false));
    for (name, src, strict) in all {
        let mut env = render::Env {
            files: &g.cx.files,
            decisions: &g.cx.decisions,
            pages: &pages,
            repo: &repo,
            strict,
            errors: &mut g.cx.errors,
        };
        let page = render::render(&name, src, &mut env);
        out.push((name, page));
    }
    out
}

pub fn build(g: &mut Guide, out: &Path, wasm: &Path) -> Result<(), String> {
    let rendered = render_all(g);
    std::fs::create_dir_all(out.join("djs")).map_err(|e| format!("{}: {e}", out.display()))?;
    let order: Vec<&Entry> = g.summary.iter().filter(|e| e.path.is_some()).collect();
    let write = |name: &str, text: &str| {
        std::fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    for (name, page) in &rendered {
        let pos = order
            .iter()
            .position(|e| e.path.as_deref() == Some(name.as_str()));
        write(
            &html_name(name),
            &template(&g.summary, name, page, pos.map(|i| (i, &order[..]))),
        )?;
    }
    write("style.css", STYLE)?;
    write("live.js", LIVE)?;
    if let Some(first) = order.first().and_then(|e| e.path.as_deref()) {
        let to = html_name(first);
        write(
            "index.html",
            &format!(
                "<!doctype html><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0; url={to}\"><a href=\"{to}\">How djs works</a>"
            ),
        )?;
    }
    // The live examples: djs-wasm and the host package, types stripped by node.
    let djs = out.join("djs");
    if wasm.exists() {
        std::fs::copy(wasm, djs.join("djs_wasm.wasm"))
            .map_err(|e| format!("{}: {e}", wasm.display()))?;
    } else {
        eprintln!(
            "djs-docs: {} is missing (scripts/build-wasm.sh): the live examples will show their build-time output only",
            wasm.display()
        );
    }
    let strip = djs.join("strip.mjs");
    std::fs::write(&strip, STRIP).map_err(|e| e.to_string())?;
    let st = std::process::Command::new("node")
        .args([
            "--no-warnings",
            &strip.to_string_lossy(),
            &repo().join("js/src").to_string_lossy(),
            &djs.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("node: {e}"))?;
    let _ = std::fs::remove_file(&strip);
    if !st.success() {
        return Err("stripping js/src for the browser failed (node >= 22.13 needed)".into());
    }
    println!("djs-docs: {} pages in {}", rendered.len(), out.display());
    Ok(())
}

fn template(
    entries: &[Entry],
    current: &str,
    page: &Page,
    pos: Option<(usize, &[&Entry])>,
) -> String {
    let mut nav = String::from("<ol>");
    for e in entries {
        if e.part {
            let _ = write!(nav, "<li class=\"part\">{}</li>", esc(&e.title));
            continue;
        }
        let cls = format!("d{}", e.depth);
        match &e.path {
            Some(p) => {
                let cur = if p == current {
                    " aria-current=\"page\""
                } else {
                    ""
                };
                let _ = write!(
                    nav,
                    "<li class=\"{cls}\"><a href=\"{}\"{cur}>{}</a></li>",
                    html_name(p),
                    esc(&e.title)
                );
            }
            None => {
                let _ = write!(nav, "<li class=\"{cls} draft\">{}</li>", esc(&e.title));
            }
        }
    }
    let cur = if current == "decisions.md" {
        " aria-current=\"page\""
    } else {
        ""
    };
    let _ = write!(
        nav,
        "<li class=\"d0\"><a href=\"decisions.html\"{cur}>Decision log</a></li></ol>"
    );
    let mut toc = String::new();
    if !page.toc.is_empty() {
        toc.push_str("<p class=\"toc-title\">On this page</p><ol>");
        for (lvl, id, text) in page.toc.iter().take(80) {
            let _ = write!(
                toc,
                "<li class=\"h{lvl}\"><a href=\"#{id}\">{}</a></li>",
                esc(text)
            );
        }
        toc.push_str("</ol>");
    }
    let mut pager = String::new();
    if let Some((i, order)) = pos {
        pager.push_str("<nav class=\"pager\">");
        let link = |class: &str, label: &str, e: &Entry| {
            format!(
                "<a class=\"{class}\" href=\"{}\"><small>{label}</small>{}</a>",
                html_name(e.path.as_deref().unwrap_or_default()),
                esc(&e.title)
            )
        };
        if i > 0 {
            pager.push_str(&link("prev", "Previous", order[i - 1]));
        }
        if let Some(e) = order.get(i + 1) {
            pager.push_str(&link("next", "Next", e));
        }
        pager.push_str("</nav>");
    }
    let edit = if current == "decisions.md" {
        "docs/decisions.md".to_string()
    } else {
        format!("docs/guide/{current}")
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · How djs works</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<header class="top"><a class="brand" href="index.html">djs</a><span class="sub">How djs works</span><span class="spacer"></span><a class="src" href="{repo}/blob/main/{edit}">page source</a></header>
<div class="layout">
<nav class="book" aria-label="Chapters">{nav}</nav>
<main><article>
{body}
</article>{pager}</main>
<aside class="toc">{toc}</aside>
</div>
<script type="module" src="live.js"></script>
</body>
</html>
"#,
        title = esc(&page.title),
        body = page.html,
        repo = render::REPO_URL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_lines() {
        let s = summary("# The tour\n\n- [Values](values.md)\n  - [Sub](sub.md)\n- [Draft]()\n");
        assert!(s[0].part && s[0].title == "The tour");
        assert_eq!(s[1].path.as_deref(), Some("values.md"));
        assert_eq!((s[2].depth, s[2].title.as_str()), (1, "Sub"));
        assert_eq!(s[3].path, None);
    }
}
