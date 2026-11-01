//! Markdown to HTML: pulldown-cmark plus what the guide needs on top: heading anchors
//! and a page outline, inline SVG diagrams, checked links (to other pages and to files of
//! the repository), decision links (`D1050`), the "In short" and "Trade-off" asides,
//! syntax highlighting, and live examples.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html,
};

/// Where links to repository files point on the site.
pub const REPO_URL: &str = "https://github.com/earendil-works/djs";

pub struct Page {
    pub title: String,
    pub html: String,
    /// (level, id, text) of the h2 and h3 headings.
    pub toc: Vec<(u8, String, String)>,
}

pub struct Env<'a> {
    pub files: &'a BTreeMap<String, String>,
    pub decisions: &'a BTreeMap<String, String>,
    /// The guide's pages (`values.md`), for checking links between them.
    pub pages: &'a BTreeSet<String>,
    /// The repository root, for checking links to its files.
    pub repo: &'a Path,
    /// Unknown references are errors (the guide); the decision log is rendered leniently.
    pub strict: bool,
    pub errors: &'a mut Vec<String>,
}

pub fn html_name(md: &str) -> String {
    format!("{}.html", md.trim_end_matches(".md"))
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if (c == ' ' || c == '-' || c == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_GFM
}

pub fn render(page: &str, md: &str, env: &mut Env) -> Page {
    let events: Vec<Event> = Parser::new_ext(md, options()).collect();
    let mut out: Vec<Event> = Vec::new();
    let mut toc = Vec::new();
    let mut title = String::new();
    let mut live_n = 0;
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = *level as u8;
                let mut j = i + 1;
                let mut text = String::new();
                while !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                    if let Event::Text(t) | Event::Code(t) = &events[j] {
                        text.push_str(t);
                    }
                    j += 1;
                }
                // Decision headings (`D1050: Value encoding`) are anchored by their ID.
                let id = match text.split_once(':') {
                    Some((d, _))
                        if d.len() > 1
                            && d.starts_with('D')
                            && d[1..].bytes().all(|b| b.is_ascii_digit()) =>
                    {
                        d.to_lowercase()
                    }
                    _ => slug(&text),
                };
                if *level == HeadingLevel::H1 && title.is_empty() {
                    title = text.clone();
                } else if (2..=3).contains(&lvl) {
                    toc.push((lvl, id.clone(), text.clone()));
                }
                let mut inner = String::new();
                html::push_html(&mut inner, events[i + 1..j].iter().cloned());
                out.push(Event::Html(CowStr::from(format!(
                    "<h{lvl} id=\"{id}\"><a class=\"anchor\" href=\"#{id}\">{inner}</a></h{lvl}>\n"
                ))));
                i = j + 1;
                continue;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(s) => s.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut j = i + 1;
                let mut code = String::new();
                while !matches!(events[j], Event::End(TagEnd::CodeBlock)) {
                    if let Event::Text(t) = &events[j] {
                        code.push_str(t);
                    }
                    j += 1;
                }
                let mut words = info.split_whitespace();
                let lang = words.next().unwrap_or("");
                let live = lang == "js" && words.any(|w| w == "live");
                let hl = crate::highlight::highlight(lang, &code);
                let html = if live {
                    live_n += 1;
                    live_block(live_n, lang, &hl, &code)
                } else {
                    format!("<pre class=\"code\"><code class=\"lang-{lang}\">{hl}</code></pre>\n")
                };
                out.push(Event::Html(CowStr::from(html)));
                i = j + 1;
                // A `gen:run` region right after a live block is its output for readers
                // of the plain Markdown; the widget shows it already.
                // (HTML blocks come as Start(HtmlBlock), Html, End(HtmlBlock).)
                if live {
                    let has = |e: Option<&Event>, s: &str| matches!(e, Some(Event::Html(h)) if h.contains(s));
                    if matches!(events.get(i), Some(Event::Start(Tag::HtmlBlock)))
                        && has(events.get(i + 1), "<!-- gen:run")
                    {
                        while i < events.len() && !has(events.get(i), "<!-- /gen -->") {
                            i += 1;
                        }
                        // Past the closing marker and its End(HtmlBlock).
                        i += 1;
                        if matches!(events.get(i), Some(Event::End(TagEnd::HtmlBlock))) {
                            i += 1;
                        }
                    }
                }
                continue;
            }
            Event::Start(Tag::Image { dest_url, .. }) if dest_url.ends_with(".svg") => {
                let mut j = i + 1;
                let mut alt = String::new();
                while !matches!(events[j], Event::End(TagEnd::Image)) {
                    if let Event::Text(t) = &events[j] {
                        alt.push_str(t);
                    }
                    j += 1;
                }
                // Generated diagrams (gen/) or hand-drawn ones (img/), inlined so that
                // they follow the page's colors.
                let svg = env.files.get(dest_url.as_ref()).cloned().or_else(|| {
                    std::fs::read_to_string(env.repo.join("docs/guide").join(dest_url.as_ref()))
                        .ok()
                });
                match svg {
                    Some(svg) => out.push(Event::Html(CowStr::from(format!(
                        "<figure class=\"diagram\">{}<figcaption>{}</figcaption></figure>",
                        svg.trim_start_matches(|c| c != '<')
                            .trim_start_matches("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
                        esc(&alt)
                    )))),
                    None => env
                        .errors
                        .push(format!("{page}: missing diagram {dest_url}")),
                }
                i = j + 1;
                continue;
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let dest = resolve_link(page, dest_url, env);
                out.push(Event::Start(Tag::Link {
                    link_type: *link_type,
                    dest_url: CowStr::from(dest),
                    title: title.clone(),
                    id: id.clone(),
                }));
                i += 1;
                continue;
            }
            Event::Text(t) => {
                link_decisions(t, &mut out, env, page);
                i += 1;
                continue;
            }
            _ => {}
        }
        out.push(events[i].clone());
        i += 1;
    }
    let mut body = String::new();
    html::push_html(&mut body, out.into_iter());
    // "> **Trade-off.** ..." and "> **In short.** ...": plain quotes on GitHub, asides here.
    let body = body
        .replace(
            "<blockquote>\n<p><strong>Trade-off",
            "<blockquote class=\"tradeoff\">\n<p><strong>Trade-off",
        )
        .replace(
            "<blockquote>\n<p><strong>In short",
            "<blockquote class=\"short\">\n<p><strong>In short",
        );
    Page {
        title,
        html: body,
        toc,
    }
}

fn live_block(n: usize, lang: &str, hl: &str, code: &str) -> String {
    let out_text = crate::run::run(code);
    let disasm = crate::run::disasm(code);
    format!(
        "<div class=\"live\" id=\"live-{n}\">\n\
         <div class=\"live-bar\"><span class=\"live-badge\">live</span>\
         <span class=\"live-note\">runs in djs-wasm in your browser</span>\
         <button class=\"live-edit\">Edit</button><button class=\"live-run\">Run ▸</button></div>\n\
         <pre class=\"code\"><code class=\"lang-{lang}\">{hl}</code></pre>\n\
         <textarea class=\"live-src\" hidden spellcheck=\"false\">{}</textarea>\n\
         <div class=\"live-tabs\" role=\"tablist\"><button data-tab=\"out\" aria-selected=\"true\">Output</button>\
         <button data-tab=\"bytecode\">Bytecode</button><button data-tab=\"heap\">Heap</button>\
         <button data-tab=\"snapshot\">Suspend &amp; snapshot</button><span class=\"live-status\"></span></div>\n\
         <pre class=\"live-pane\" data-pane=\"out\">{}</pre>\n\
         <pre class=\"live-pane\" data-pane=\"bytecode\" hidden>{}</pre>\n\
         <div class=\"live-pane\" data-pane=\"heap\" hidden><p class=\"muted\">Run the example to walk the VM's arena.</p></div>\n\
         <div class=\"live-pane\" data-pane=\"snapshot\" hidden><p class=\"muted\">Run the example to see it suspend and snapshot.</p></div>\n\
         </div>\n",
        esc(code),
        esc(&out_text),
        esc(&disasm)
    )
}

/// A link to another page becomes its `.html`; a relative link to a repository file goes
/// to the file on GitHub. Either must exist.
fn resolve_link(page: &str, dest: &str, env: &mut Env) -> String {
    if dest.contains("://") || dest.starts_with('#') || dest.starts_with("mailto:") {
        return dest.to_string();
    }
    let (path, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, format!("#{f}")),
        None => (dest, String::new()),
    };
    if env.pages.contains(path) {
        return format!("{}{frag}", html_name(path));
    }
    // Relative to docs/guide.
    let mut parts: Vec<&str> = vec!["docs", "guide"];
    for c in path.split('/') {
        match c {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            c => parts.push(c),
        }
    }
    let rel = parts.join("/");
    // The decision log is part of the site.
    if rel == "docs/decisions.md" {
        return format!("decisions.html{frag}");
    }
    let full = env.repo.join(&rel);
    if !full.exists() {
        if env.strict {
            env.errors.push(format!("{page}: broken link {dest}"));
        }
        return dest.to_string();
    }
    let kind = if full.is_dir() { "tree" } else { "blob" };
    format!("{REPO_URL}/{kind}/main/{rel}{frag}")
}

/// `D1050` in prose becomes a link to the decision, with its title as the tooltip. In
/// the guide an unknown ID is an error, so references cannot dangle.
fn link_decisions<'a>(t: &CowStr<'a>, out: &mut Vec<Event<'a>>, env: &mut Env, page: &str) {
    let s = t.as_ref();
    let b = s.as_bytes();
    let mut last = 0;
    let mut i = 0;
    let mut buf = String::new();
    while i < b.len() {
        let boundary = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        if b[i] == b'D' && boundary {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let end_ok = j == b.len() || !b[j].is_ascii_alphanumeric();
            if (3..=4).contains(&(j - i - 1)) && end_ok {
                let id = &s[i..j];
                match env.decisions.get(id) {
                    Some(title) => {
                        buf.push_str(&esc(&s[last..i]));
                        let _ = write!(
                            buf,
                            "<a class=\"decision\" href=\"decisions.html#{}\" title=\"{}\">{id}</a>",
                            id.to_lowercase(),
                            esc(title)
                        );
                        last = j;
                    }
                    None if env.strict => env.errors.push(format!("{page}: unknown decision {id}")),
                    None => {}
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    if last == 0 {
        out.push(Event::Text(t.clone()));
    } else {
        buf.push_str(&esc(&s[last..]));
        out.push(Event::InlineHtml(CowStr::from(buf)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env<R>(strict: bool, f: impl FnOnce(&mut Env) -> R) -> (R, Vec<String>) {
        let files = BTreeMap::new();
        let mut decisions = BTreeMap::new();
        decisions.insert("D1050".to_string(), "Value encoding".to_string());
        let pages: BTreeSet<String> = ["values.md".to_string()].into();
        let mut errors = Vec::new();
        let repo = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let r = f(&mut Env {
            files: &files,
            decisions: &decisions,
            pages: &pages,
            repo,
            strict,
            errors: &mut errors,
        });
        (r, errors)
    }

    #[test]
    fn decisions_and_links() {
        let md = "# Title\n\nSee D1050 and D9999, [values](values.md#x), [ARCH](../../ARCH.md), [log](../decisions.md#d1050), [gone](nope.md).\n\n## Sub `code`\n";
        let (p, errors) = with_env(true, |env| render("p.md", md, env));
        assert_eq!(p.title, "Title");
        assert!(
            p.html
                .contains("href=\"decisions.html#d1050\" title=\"Value encoding\">D1050</a>")
        );
        assert!(p.html.contains("href=\"values.html#x\""));
        assert!(
            p.html
                .contains(&format!("href=\"{REPO_URL}/blob/main/ARCH.md\""))
        );
        assert!(p.html.contains("href=\"decisions.html#d1050\">log"));
        assert_eq!(p.toc, [(2, "sub-code".to_string(), "Sub code".to_string())]);
        assert_eq!(
            errors,
            ["p.md: unknown decision D9999", "p.md: broken link nope.md"]
        );
        // The decision log itself is lenient.
        let (_, errors) = with_env(false, |env| {
            render("decisions.md", "D9999 [x](nope.md)", env)
        });
        assert!(errors.is_empty());
    }

    #[test]
    fn asides_and_decision_anchors() {
        let md = "## D1050: Value encoding\n\n> **Trade-off.** x\n\n> **In short.** y\n";
        let (p, _) = with_env(false, |env| render("decisions.md", md, env));
        assert!(p.html.contains("<h2 id=\"d1050\">"));
        assert!(p.html.contains("<blockquote class=\"tradeoff\">"));
        assert!(p.html.contains("<blockquote class=\"short\">"));
    }

    #[test]
    fn live_blocks_carry_their_output() {
        let md = "```js live\nconsole.log(6 * 7)\n```\n<!-- gen:run -->\n```text\n42\n```\n<!-- /gen -->\n\nafter\n";
        let (p, _) = with_env(true, |env| render("p.md", md, env));
        assert!(
            p.html
                .contains("<pre class=\"live-pane\" data-pane=\"out\">42\n</pre>"),
            "{}",
            p.html
        );
        // The region's copy of the output is not rendered a second time.
        assert!(!p.html.contains("lang-text"), "{}", p.html);
        assert!(p.html.contains("<p>after</p>"));
    }
}
