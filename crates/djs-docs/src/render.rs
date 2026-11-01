//! Markdown to HTML: pulldown-cmark plus what the guide needs on top: heading anchors
//! and a page outline, inline SVG diagrams, checked links (to other pages and to files of
//! the repository), decision links (`D1050`), the "In short" and "Trade-off" asides,
//! syntax highlighting, and live examples.

use std::collections::{BTreeMap, BTreeSet};
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
    /// Strict pages are also held to the self-containment rule (D2102, `check_prose`).
    pub strict: bool,
    /// The commit source links point at (`source_url`).
    pub rev: &'a str,
    /// The repository's tracked paths, for linking mentions of them (D2104).
    pub index: &'a crate::paths::RepoIndex,
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
    let mut in_link = false;
    // The repository path the current link goes to, if it is a source link.
    let mut link_rel: Option<String> = None;
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
                if env.strict {
                    check_prose(&text, page, env);
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
                in_link = true;
                let dest = resolve_link(page, dest_url, env);
                link_rel = ["blob", "tree"].iter().find_map(|k| {
                    let rest = dest.strip_prefix(&format!("{REPO_URL}/{k}/{}/", env.rev))?;
                    Some(
                        rest.split('#')
                            .next()
                            .unwrap_or("")
                            .trim_end_matches('/')
                            .to_string(),
                    )
                });
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
                if env.strict {
                    check_prose(t, page, env);
                }
                link_text(t, &mut out, env, page, in_link);
                i += 1;
                continue;
            }
            Event::End(TagEnd::Link) => in_link = false,
            Event::Code(t) => {
                if env.strict {
                    check_prose(t, page, env);
                }
                // A path as a link's text names the file the link goes to.
                if let (true, Some(rel), crate::paths::Resolved::Path(p, ..)) =
                    (env.strict && in_link, &link_rel, env.index.code(t))
                    && &p != rel
                {
                    env.errors
                        .push(format!("{page}: `{t}` links to {rel}, not to {p} (D2104)"));
                }
                if !in_link && let Some(html) = code_link(t, page, env) {
                    out.push(Event::InlineHtml(CowStr::from(html)));
                    i += 1;
                    continue;
                }
            }
            _ => {}
        }
        out.push(events[i].clone());
        i += 1;
    }
    let mut body = String::new();
    html::push_html(&mut body, out.into_iter());
    if env.strict {
        for p in crate::paths::unlinked(&body, env.index) {
            env.errors.push(format!(
                "{page}: {p} is not a link: a mention of a repository path links to it (D2104)"
            ));
        }
    }
    // Source links (`source_url`) are marked, so that readers can tell them from pages.
    let body = body.replace(
        &format!("<a href=\"{REPO_URL}/"),
        &format!("<a class=\"source\" href=\"{REPO_URL}/"),
    );
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

/// A live example (D2103). The markup is the page without JavaScript: the highlighted
/// code and the output and bytecode from the build. live.js turns the code into an
/// editor (a textarea over this `pre`, which keeps showing the highlighting), shows the
/// Run and Reset buttons and makes the tabs work.
fn live_block(n: usize, lang: &str, hl: &str, code: &str) -> String {
    let out_text = crate::run::run(code);
    let disasm = crate::run::disasm(code);
    let tab = |id: &str, label: &str, selected: bool| {
        format!(
            "<button role=\"tab\" id=\"live-{n}-tab-{id}\" data-tab=\"{id}\" aria-controls=\"live-{n}-{id}\" \
             aria-selected=\"{selected}\" tabindex=\"{}\"{}>{label}</button>",
            if selected { 0 } else { -1 },
            // Without JavaScript the tabs cannot switch: only Output shows.
            if selected { "" } else { " hidden" }
        )
    };
    let pane = |tag: &str, id: &str, hidden: bool, body: &str| {
        format!(
            "<{tag} class=\"live-pane\" data-pane=\"{id}\" id=\"live-{n}-{id}\" role=\"tabpanel\" \
             aria-labelledby=\"live-{n}-tab-{id}\" tabindex=\"0\"{}>{body}</{tag}>\n",
            if hidden { " hidden" } else { "" }
        )
    };
    format!(
        "<div class=\"live\" id=\"live-{n}\">\n\
         <div class=\"live-bar\"><span class=\"live-badge\">live</span>\
         <span class=\"live-note\">output from the build</span>\
         <button class=\"live-reset\" hidden>Reset</button>\
         <button class=\"live-run\" hidden>Run</button></div>\n\
         <div class=\"live-editor\"><pre class=\"code\"><code class=\"lang-{lang}\">{hl}</code></pre></div>\n\
         <div class=\"live-tabs\" role=\"tablist\" aria-label=\"Results\">{}{}{}{}\
         <span class=\"live-status\" role=\"status\"></span></div>\n\
         {}{}{}{}</div>\n",
        tab("out", "Output", true),
        tab("bytecode", "Bytecode", false),
        tab("heap", "Heap", false),
        tab("snapshot", "Suspend &amp; snapshot", false),
        pane("pre", "out", false, &esc(&out_text)),
        pane("pre", "bytecode", true, &esc(&disasm)),
        pane(
            "div",
            "heap",
            true,
            "<p class=\"muted\">Run the example to walk the VM's arena.</p>"
        ),
        pane(
            "div",
            "snapshot",
            true,
            "<p class=\"muted\">Run the example to see it suspend and snapshot.</p>"
        ),
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
    // The guide is self-contained (D2102): it may point at code, never at other
    // documents of the repository as the place where something is explained.
    if env.strict && rel.ends_with(".md") {
        env.errors.push(format!(
            "{page}: link to {rel}, a document outside the guide: explain it in the guide instead (D2102)"
        ));
    }
    source_url(&rel, full.is_dir(), &frag, env.rev)
}

/// The one place source links are made: a repository path on GitHub, at the commit the
/// site is built from, so that the code a page points at is the code it describes.
pub fn source_url(rel: &str, dir: bool, frag: &str, rev: &str) -> String {
    let kind = if dir { "tree" } else { "blob" };
    format!("{REPO_URL}/{kind}/{rev}/{rel}{frag}")
}

/// The self-containment rule for prose (D2102): no bare goal numbers (`G2`: name the
/// goal and link goals.md), no issue numbers (`0088`), and no mentions of documents
/// outside the guide (`ARCH.md`, `docs/host.md`, `work/...`), in text or inline code.
/// Code blocks are not prose.
fn check_prose(text: &str, page: &str, env: &mut Env) {
    for word in text.split(|c: char| c.is_whitespace() || "()[]{},;:\"'`“”‘’".contains(c)) {
        let word = word.trim_end_matches(['.', '!', '?']);
        if word.len() == 2 && word.starts_with('G') && matches!(word.as_bytes()[1], b'1'..=b'5') {
            env.errors.push(format!(
                "{page}: bare goal number {word}: name the goal and link it (goals.md) (D2102)"
            ));
        } else if word.len() == 4
            && word.starts_with('0')
            && word.bytes().all(|b| b.is_ascii_digit())
        {
            env.errors.push(format!(
                "{page}: issue number {word}: the guide cannot point at work/issues; say what was found (D2102)"
            ));
        } else if word.starts_with("work/")
            || matches!(word, "ARCH" | "TESTING")
            || (word.ends_with(".md") && word.len() > 3 && !env.pages.contains(word))
        {
            env.errors.push(format!(
                "{page}: mentions {word}, a document outside the guide: explain it in the guide instead (D2102)"
            ));
        }
    }
}

/// A source link for inline code that names a repository path (D2104). In the guide a
/// mention that names no tracked path, or more than one, is an error; the decision log
/// is a record of its time and links only full paths that still exist.
fn code_link(t: &str, page: &str, env: &mut Env) -> Option<String> {
    use crate::paths::Resolved;
    match env.index.code(t) {
        Resolved::Path(p, dir, frag) => {
            let full = t
                .trim()
                .split(':')
                .next()
                .unwrap_or("")
                .trim_end_matches('/')
                == p;
            (env.strict || full).then(|| {
                format!(
                    "<a href=\"{}\"><code>{}</code></a>",
                    source_url(&p, dir, &frag, env.rev),
                    esc(t)
                )
            })
        }
        Resolved::Missing if env.strict => {
            env.errors.push(format!(
                "{page}: `{t}` names no file or directory of the repository (renamed or removed?) (D2104)"
            ));
            None
        }
        Resolved::Ambiguous(all) if env.strict => {
            env.errors.push(format!(
                "{page}: `{t}` could be any of {}: write the path (D2104)",
                all.join(", ")
            ));
            None
        }
        Resolved::No => code_words(t, page, env),
        _ => None,
    }
}

/// Inline code that is a command or glob with repository paths in it (`node
/// js/pi/pi-tests.ts`, `builtins/*.ts`): the paths become links inside the code (a glob
/// links its directory). None if it has no paths.
fn code_words(t: &str, page: &str, env: &mut Env) -> Option<String> {
    use crate::paths::Resolved;
    let mut html = String::from("<code>");
    let mut any = false;
    for (k, w) in t.split(' ').enumerate() {
        if k > 0 {
            html.push(' ');
        }
        match env.index.text(w) {
            Resolved::Path(p, dir, frag) => {
                any = true;
                let core = w.trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
                let _ = std::fmt::Write::write_fmt(
                    &mut html,
                    format_args!(
                        "<a href=\"{}\">{}</a>{}",
                        source_url(&p, dir, &frag, env.rev),
                        esc(core),
                        esc(&w[core.len()..])
                    ),
                );
            }
            Resolved::Missing if env.strict => {
                env.errors.push(format!(
                    "{page}: {w} in `{t}` names no file or directory of the repository (renamed or removed?) (D2104)"
                ));
                html.push_str(&esc(w));
            }
            _ => html.push_str(&esc(w)),
        }
    }
    html.push_str("</code>");
    any.then_some(html)
}

/// Prose: `D1050` becomes a link to the decision, with its title as the tooltip, and a
/// repository path (`crates/djs-core/src/heap/gc.rs`) a source link (D2104). In the
/// guide an unknown ID or a path that does not exist is an error, so references cannot
/// dangle.
fn link_text<'a>(
    t: &CowStr<'a>,
    out: &mut Vec<Event<'a>>,
    env: &mut Env,
    page: &str,
    in_link: bool,
) {
    use crate::paths::Resolved;
    let s = t.as_ref();
    // (start, end, html) of each replacement, in order.
    let mut subs: Vec<(usize, usize, String)> = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
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
                    Some(title) => subs.push((
                        i,
                        j,
                        format!(
                            "<a class=\"decision\" href=\"decisions.html#{}\" title=\"{}\">{id}</a>",
                            id.to_lowercase(),
                            esc(title)
                        ),
                    )),
                    None if env.strict => env.errors.push(format!("{page}: unknown decision {id}")),
                    None => {}
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    if !in_link {
        let sep = |c: char| c.is_whitespace() || "()[]{},;\"'“”‘’".contains(c);
        let mut at = 0;
        for w in s.split(sep) {
            let start = at;
            at += w.len() + s[at + w.len()..].chars().next().map_or(0, char::len_utf8);
            match env.index.text(w) {
                Resolved::Path(p, dir, frag) => {
                    let core = w.trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
                    subs.push((
                        start,
                        start + core.len(),
                        format!("<a href=\"{}\">{}</a>", source_url(&p, dir, &frag, env.rev), esc(core)),
                    ));
                }
                Resolved::Missing if env.strict => env.errors.push(format!(
                    "{page}: {w} names no file or directory of the repository (renamed or removed?) (D2104)"
                )),
                Resolved::Ambiguous(all) if env.strict => env.errors.push(format!(
                    "{page}: {w} could be any of {}: write the path (D2104)",
                    all.join(", ")
                )),
                _ => {}
            }
        }
        subs.sort_by_key(|x| x.0);
    }
    if subs.is_empty() {
        out.push(Event::Text(t.clone()));
        return;
    }
    let mut buf = String::new();
    let mut last = 0;
    for (a, b, h) in subs {
        if a < last {
            continue;
        }
        buf.push_str(&esc(&s[last..a]));
        buf.push_str(&h);
        last = b;
    }
    buf.push_str(&esc(&s[last..]));
    out.push(Event::InlineHtml(CowStr::from(buf)));
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
        let index = crate::paths::RepoIndex::load(repo);
        let r = f(&mut Env {
            files: &files,
            decisions: &decisions,
            pages: &pages,
            repo,
            strict,
            rev: "main",
            index: &index,
            errors: &mut errors,
        });
        (r, errors)
    }

    #[test]
    fn decisions_and_links() {
        let md = "# Title\n\nSee D1050 and D9999, [values](values.md#x), [code](../../crates/djs-core/src/value.rs), [log](../decisions.md#d1050), [gone](nope.md).\n\n## Sub `code`\n";
        let (p, errors) = with_env(true, |env| render("p.md", md, env));
        assert_eq!(p.title, "Title");
        assert!(
            p.html
                .contains("href=\"decisions.html#d1050\" title=\"Value encoding\">D1050</a>")
        );
        assert!(p.html.contains("href=\"values.html#x\""));
        assert!(p.html.contains(&format!(
            "<a class=\"source\" href=\"{REPO_URL}/blob/main/crates/djs-core/src/value.rs\""
        )));
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
    fn the_guide_is_self_contained() {
        let md = "# T\n\nGoal G2 (see ARCH.md, `docs/host.md`, work/issues/x, 0088 and values.md); \
                  [arch](../../ARCH.md).\n\n## Heading G4\n\n```js\nconst f = \"README.md\";\n```\n";
        let (_, errors) = with_env(true, |env| render("p.md", md, env));
        let short: Vec<&str> = errors
            .iter()
            .map(|e| e.split(':').nth(1).unwrap_or("").trim())
            .collect();
        assert_eq!(
            short,
            [
                "bare goal number G2",
                "mentions ARCH.md, a document outside the guide",
                "mentions docs/host.md, a document outside the guide",
                "mentions work/issues/x, a document outside the guide",
                "issue number 0088",
                "link to ARCH.md, a document outside the guide",
                "bare goal number G4",
            ]
        );
        // The decision log is a record, not explanation: exempt.
        let (_, errors) = with_env(false, |env| render("decisions.md", md, env));
        assert!(errors.is_empty());
    }

    #[test]
    fn repository_paths_link_to_the_source() {
        let md = "# T\n\nIn `crates/djs-core/src/heap/gc.rs`, `js/src/`, `heap/cells.rs:12`, \
                  `node js/pi/pi-tests.ts`, scripts/gate.sh. and `x / y`.\n\n\
                  Gone: `crates/djs-core/src/nope.rs`, crates/nope; vague: `lib.rs`; \
                  [`js/src/`](../../js/src/sandbox.ts).\n\n```sh\nscripts/gate.sh\n```\n";
        let (p, errors) = with_env(true, |env| render("p.md", md, env));
        let src = |kind: &str, path: &str| {
            format!("<a class=\"source\" href=\"{REPO_URL}/{kind}/main/{path}\">")
        };
        for want in [
            format!(
                "{}<code>crates/djs-core/src/heap/gc.rs</code></a>",
                src("blob", "crates/djs-core/src/heap/gc.rs")
            ),
            format!("{}<code>js/src/</code></a>", src("tree", "js/src")),
            format!(
                "{}<code>heap/cells.rs:12</code></a>",
                src("blob", "crates/djs-core/src/heap/cells.rs#L12")
            ),
            format!(
                "<code>node {}js/pi/pi-tests.ts</a></code>",
                src("blob", "js/pi/pi-tests.ts")
            ),
            format!(
                "{}scripts/gate.sh</a>. and <code>x / y</code>",
                src("blob", "scripts/gate.sh")
            ),
            "<pre class=\"code\"><code class=\"lang-sh\">scripts/gate.sh".to_string(),
        ] {
            assert!(p.html.contains(&want), "{want} in {}", p.html);
        }
        let want = [
            "p.md: `crates/djs-core/src/nope.rs` names no file or directory",
            "p.md: crates/nope names no file or directory",
            "p.md: `lib.rs` could be any of crates/",
            "p.md: `js/src/` links to js/src/sandbox.ts, not to js/src (D2104)",
        ];
        assert_eq!(errors.len(), want.len(), "{errors:#?}");
        for (e, w) in errors.iter().zip(want) {
            assert!(e.starts_with(w), "{e} vs {w}");
        }
        // The decision log links full paths only, and quietly.
        let (p, errors) = with_env(false, |env| render("decisions.md", md, env));
        assert!(errors.is_empty(), "{errors:?}");
        assert!(p.html.contains("<code>heap/cells.rs:12</code>"));
        assert!(
            p.html
                .contains(&src("blob", "crates/djs-core/src/heap/gc.rs"))
        );
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
        // Without JavaScript: the highlighted code and the build's output, visible.
        for want in [
            "<div class=\"live-editor\"><pre class=\"code\"><code class=\"lang-js\"><span class=\"tok-variable\">console</span>",
            "data-pane=\"out\" id=\"live-1-out\" role=\"tabpanel\" aria-labelledby=\"live-1-tab-out\" tabindex=\"0\">42\n</pre>",
            "aria-controls=\"live-1-out\" aria-selected=\"true\" tabindex=\"0\">Output</button>",
            "data-pane=\"bytecode\" id=\"live-1-bytecode\" role=\"tabpanel\" aria-labelledby=\"live-1-tab-bytecode\" tabindex=\"0\" hidden>",
        ] {
            assert!(p.html.contains(want), "{want} in {}", p.html);
        }
        // The region's copy of the output is not rendered a second time.
        assert!(!p.html.contains("lang-text"), "{}", p.html);
        assert!(p.html.contains("<p>after</p>"));
    }
}
