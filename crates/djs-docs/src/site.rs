//! The static site: SUMMARY.md navigation, the page template, assets and the live
//! examples' files (djs-wasm and the host package with its types stripped).

use std::fmt::Write as _;
use std::path::Path;

use crate::render::{self, Page, esc, html_name};
use crate::{Guide, page_set, repo};

const STYLE: &str = include_str!("../assets/style.css");
const LIVE: &str = include_str!("../assets/live.js");
const HIGHLIGHT: &str = include_str!("../assets/highlight.js");
const SEARCH: &str = include_str!("../assets/search.js");
const STRIP: &str = include_str!("../assets/strip.mjs");
const PAGE: &str = include_str!("../assets/page.js");
/// Elanor's published files, vendored by scripts/elanor-update.sh (D2200).
const ELANOR_DIR: &str = "crates/djs-docs/assets/elanor";
const ELANOR_VERSION: &str = include_str!("../assets/elanor/VERSION");
const ICON_SEARCH: &str = include_str!("../assets/elanor/icons/search.svg");
const ICON_DARK: &str = include_str!("../assets/elanor/icons/dark-mode.svg");
const ICON_LIGHT: &str = include_str!("../assets/elanor/icons/light-mode.svg");
const ICON_MENU: &str = include_str!("../assets/elanor/icons/menu.svg");

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
    let rev = source_rev(&repo);
    let mut out = Vec::new();
    let mut all: Vec<(String, &str, bool)> = g
        .pages
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str(), true))
        .collect();
    // The log's own preamble addresses contributors (format, append-only); on the site it
    // gets an introduction for readers instead (D2102).
    let entries = g
        .decisions_md
        .match_indices("\n## D")
        .find(|(i, _)| g.decisions_md[i + 5..].starts_with(|c: char| c.is_ascii_digit()))
        .map_or(g.decisions_md.as_str(), |(i, _)| &g.decisions_md[i + 1..]);
    let log = format!("{DECISIONS_INTRO}\n{entries}");
    all.push(("decisions.md".into(), log.as_str(), false));
    for (name, src, strict) in all {
        let mut env = render::Env {
            files: &g.cx.files,
            decisions: &g.cx.decisions,
            pages: &pages,
            repo: &repo,
            strict,
            rev: &rev,
            errors: &mut g.cx.errors,
        };
        let page = render::render(&name, src, &mut env);
        out.push((name, page));
    }
    out
}

/// The top of the decision log page.
const DECISIONS_INTRO: &str = "# Decision log

While djs was built, every decision that others needed to know about was recorded, in the
order it was made: what was decided and why, often with the measurements behind it. The
chapters of this guide explain the decisions that matter for understanding djs; each
mention of an ID such as D1050 links to its entry here.

The entries are a working record, written for the people building djs: they name files
of the repository and the issues in which the work happened, and a later entry may
refine or replace an earlier one, saying so.
";

/// The commit source links point at: the one being deployed (`$GITHUB_SHA` in the Pages
/// workflow), else the checkout's HEAD, else `main`.
fn source_rev(repo: &Path) -> String {
    if let Ok(sha) = std::env::var("GITHUB_SHA")
        && !sha.is_empty()
    {
        return sha;
    }
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string())
}

pub fn build(g: &mut Guide, out: &Path, wasm: &Path) -> Result<(), String> {
    let rendered = render_all(g);
    let rev = source_rev(&repo());
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
            &template(&g.summary, name, page, pos.map(|i| (i, &order[..])), &rev),
        )?;
    }
    write("style.css", STYLE)?;
    write("page.js", PAGE)?;
    copy_dir(&repo().join(ELANOR_DIR), &out.join("elanor"))?;
    write("live.js", LIVE)?;
    write("highlight.js", HIGHLIGHT)?;
    write("search.js", SEARCH)?;
    write("search.json", &crate::search::index(&rendered, html_name))?;
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

/// Copies the vendored Elanor files (tokens, fonts and their licenses, icons, logos).
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let rd = std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for e in rd.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// An Elanor icon inline, so that it takes the text color (`.icon` in style.css).
fn icon(svg: &str, class: &str) -> String {
    let at = svg.find("<svg").unwrap_or(0);
    svg[at..].trim_end().replacen(
        "<svg ",
        &format!("<svg class=\"icon {class}\" aria-hidden=\"true\" focusable=\"false\" "),
        1,
    )
}

/// The pinned Elanor commit and the token brand, from assets/elanor/VERSION.
fn elanor_version() -> (&'static str, &'static str) {
    let field = |k: &str| {
        ELANOR_VERSION
            .lines()
            .find_map(|l| l.strip_prefix(k)?.strip_prefix(' '))
            .unwrap_or("?")
    };
    (field("elanor"), field("brand"))
}

fn template(
    entries: &[Entry],
    current: &str,
    page: &Page,
    pos: Option<(usize, &[&Entry])>,
    rev: &str,
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
    // The page's Markdown, at the commit the site was built from, like every other source
    // link (`render::source_url`, D2102).
    let edit = render::source_url(
        &if current == "decisions.md" {
            "docs/decisions.md".to_string()
        } else {
            format!("docs/guide/{current}")
        },
        false,
        "",
        rev,
    );
    let (sha, brand) = elanor_version();
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · How djs works</title>
<link rel="icon" href="elanor/logos/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="elanor/tokens.css">
<link rel="stylesheet" href="elanor/fonts/mallos-serif.css">
<link rel="stylesheet" href="elanor/fonts/commit-mono.css">
<link rel="stylesheet" href="style.css">
<script>try{{const s=localStorage.getItem("djs-color-scheme");if(s)document.documentElement.dataset.colorScheme=s}}catch{{}}</script>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>
<header class="top">
<button class="icon-button menu-button" type="button" aria-label="Chapters" aria-expanded="false">{menu}</button>
<div class="identity"><a class="mark" href="https://earendil.com" title="Earendil"><img class="on-light" src="elanor/logos/earendil-logo-on-light.svg" alt="Earendil"><img class="on-dark" src="elanor/logos/earendil-logo-on-dark.svg" alt="Earendil"></a><a class="brand" href="index.html"><b>djs</b><span>How djs works</span></a></div>
<span class="spacer"></span>
<div class="search">{search_icon}<input type="search" id="search" placeholder="Search  /" aria-label="Search the guide" autocomplete="off"><ol id="search-results" hidden></ol></div>
<div class="actions"><a class="icon-button src" href="{edit}" title="This page's source">Source</a><button class="icon-button scheme" type="button" aria-label="Switch between light and dark">{to_dark}{to_light}</button></div>
</header>
<div class="layout">
<nav class="book" aria-label="Chapters">{nav}</nav>
<main id="main" tabindex="-1"><div class="page"><article data-text-context="prose">
{body}
</article>{pager}
<footer class="foot"><p>djs is built by <a href="https://earendil.com">Earendil</a>. <a href="{repo}">Source on GitHub</a>.</p><p>Designed with Elanor, Earendil's design system (<a href="https://github.com/earendil-works/elanor/commit/{sha}">{short}</a>, {brand} tokens). Fonts: Mallos Serif and Commit Mono, <a href="elanor/fonts/licenses/mallos-serif.txt">SIL Open Font License 1.1</a>. Icons: Material Symbols, <a href="elanor/icons/LICENSE">Apache License 2.0</a>.</p></footer></div>
<aside class="toc">{toc}</aside></main>
</div>
<script type="module" src="page.js"></script>
{live}<script type="module" src="search.js"></script>
</body>
</html>
"##,
        title = esc(&page.title),
        // Wide tables scroll inside their wrapper instead of widening the column.
        body = page
            .html
            .replace("<table>", "<div class=\"table-scroll\"><table>")
            .replace("</table>", "</table></div>"),
        // Only pages with live examples load their editor (live.js loads djs-wasm itself,
        // on first use).
        live = if page.html.contains("<div class=\"live\"") {
            "<script type=\"module\" src=\"live.js\"></script>\n"
        } else {
            ""
        },
        repo = render::REPO_URL,
        menu = icon(ICON_MENU, "menu-icon"),
        search_icon = icon(ICON_SEARCH, "search-icon"),
        to_dark = icon(ICON_DARK, "to-dark"),
        to_light = icon(ICON_LIGHT, "to-light"),
        short = &sha[..sha.len().min(10)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every token the site's CSS uses exists in the vendored Elanor tokens: an Elanor
    /// update that renames one fails here instead of silently unstyling the site.
    #[test]
    fn tokens_exist() {
        let tokens = std::fs::read_to_string(repo().join(ELANOR_DIR).join("tokens.css")).unwrap();
        let svg_style = crate::svg::style("x");
        for (file, css) in [("style.css", STYLE), ("svg.rs", svg_style.as_str())] {
            for part in css.split("var(").skip(1) {
                let name = part.split([',', ')']).next().unwrap().trim();
                assert!(
                    tokens.contains(&format!("{name}:")),
                    "{file} uses {name}, which elanor/tokens.css does not define"
                );
            }
        }
    }

    #[test]
    fn icons_inline() {
        let i = icon(ICON_SEARCH, "s");
        assert!(
            i.starts_with("<svg class=\"icon s\" aria-hidden=\"true\""),
            "{i}"
        );
        let (sha, brand) = elanor_version();
        assert_eq!((sha.len(), brand), (40, "earendil"));
    }

    #[test]
    fn summary_lines() {
        let s = summary("# The tour\n\n- [Values](values.md)\n  - [Sub](sub.md)\n- [Draft]()\n");
        assert!(s[0].part && s[0].title == "The tour");
        assert_eq!(s[1].path.as_deref(), Some("values.md"));
        assert_eq!((s[2].depth, s[2].title.as_str()), (1, "Sub"));
        assert_eq!(s[3].path, None);
    }
}
