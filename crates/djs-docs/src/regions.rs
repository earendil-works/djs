//! Generated regions: `<!-- gen:NAME ARGS -->` ... `<!-- /gen -->` in a guide page.
//!
//! The content between the markers comes from djs-core's own tables (`LAYOUTS`, `OPS`,
//! `INTRINSICS`, the value tags), from the decision log, or from compiling and running
//! the page's preceding `js` block. It is checked in, so a page reads the same on GitHub,
//! and `djs-docs check` fails when it is stale.
//!
//! `run` regions (example output) are only rewritten with `--examples`: a changed output
//! is a behavior change someone should look at, so the gate fails on it instead of
//! regenerating it after a rebase.

use std::collections::BTreeMap;
use std::fmt::Write;

use djs_core::bytecode::intrinsics::{INTRINSICS, IntrinsicKind};
use djs_core::bytecode::ops::{OPS, OpInfo};
use djs_core::heap::cells::LAYOUTS;
use djs_core::heap::layout::EXPORTED_TAGS;
use djs_core::value::{TAG_PREFIX, tag};

use crate::svg;

/// What the generators read besides djs-core: repository text files.
pub struct Sources {
    /// `docs/decisions.md`.
    pub decisions_md: String,
    /// `crates/djs-core/src/bytecode/ops.rs`: the `///` docs of the opcodes.
    pub ops_rs: String,
    /// `builtins/intrinsics.d.ts`: the docs of the Rust intrinsics.
    pub intrinsics_dts: String,
}

pub struct Ctx {
    /// Generated files (diagrams), relative to the guide directory.
    pub files: BTreeMap<String, String>,
    /// Decision ID (`D1050`) -> title.
    pub decisions: BTreeMap<String, String>,
    op_docs: BTreeMap<String, String>,
    intrinsic_docs: BTreeMap<String, String>,
    pub errors: Vec<String>,
    /// Problems that keep a region's old text without failing (no test262 checkout).
    pub warnings: Vec<String>,
    /// Status numbers, computed on the first `gen:status` region (D2101).
    status: Option<crate::status::Status>,
    /// Rewrite `run` regions too.
    pub examples: bool,
    /// `run` regions whose output changed (page names).
    pub changed_examples: Vec<String>,
}

impl Ctx {
    pub fn new(src: &Sources, examples: bool) -> Ctx {
        Ctx {
            files: BTreeMap::new(),
            decisions: parse_decisions(&src.decisions_md),
            op_docs: parse_op_docs(&src.ops_rs),
            intrinsic_docs: parse_intrinsic_docs(&src.intrinsics_dts),
            errors: Vec::new(),
            warnings: Vec::new(),
            status: None,
            examples,
            changed_examples: Vec::new(),
        }
    }
}

pub fn parse_decisions(md: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for line in md.lines() {
        if let Some((num, title)) = line.strip_prefix("## D").and_then(|r| r.split_once(": "))
            && num.bytes().all(|b| b.is_ascii_digit())
        {
            m.insert(format!("D{num}"), title.to_string());
        }
    }
    m
}

/// Opcode name -> the `///` doc lines before its `ops!` entry, joined.
fn parse_op_docs(ops_rs: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let Some(start) = ops_rs.find("\nops! {") else {
        return m;
    };
    let mut doc = String::new();
    for line in ops_rs[start..].lines().skip(1) {
        let t = line.trim();
        if let Some(d) = t.strip_prefix("///") {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(d.trim());
        } else if let Some((_, rest)) = t.split_once(' ').filter(|_| t.contains(" pop ")) {
            if let Some((method, _)) = rest.split_once('(') {
                m.insert(
                    method.trim_end_matches('_').to_string(),
                    std::mem::take(&mut doc),
                );
            }
        } else if !t.starts_with("//") {
            doc.clear();
        }
    }
    m
}

/// Intrinsic name -> its doc comment in intrinsics.d.ts, without the tags.
fn parse_intrinsic_docs(dts: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut doc = String::new();
    for line in dts.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("/**") {
            doc = rest.trim_end_matches("*/").trim().to_string();
        } else if t.starts_with('*') && !t.starts_with("*/") {
            doc.push(' ');
            doc.push_str(t.trim_start_matches('*').trim_end_matches("*/").trim());
        } else if let Some(rest) = t.strip_prefix("declare function ") {
            let name = rest.split(['(', '<']).next().unwrap_or("").to_string();
            let clean: Vec<&str> = doc
                .split_whitespace()
                .filter(|w| !w.starts_with('@'))
                .collect();
            m.insert(name, clean.join(" "));
            doc.clear();
        } else if !t.starts_with("*/") {
            doc.clear();
        }
    }
    m
}

/// Re-generates every region of `md`.
pub fn regen(page: &str, md: &str, cx: &mut Ctx) -> String {
    let mut out = String::new();
    let mut lines = md.lines();
    let mut last_js: Option<String> = None;
    let mut fence: Option<(String, String)> = None; // (info, body)
    while let Some(line) = lines.next() {
        out.push_str(line);
        out.push('\n');
        if let Some((info, body)) = &mut fence {
            if line.trim_start().starts_with("```") {
                if info.split_whitespace().next() == Some("js") {
                    last_js = Some(body.clone());
                }
                fence = None;
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        if let Some(info) = line.trim_start().strip_prefix("```") {
            fence = Some((info.trim().to_string(), String::new()));
            continue;
        }
        let Some(spec) = line
            .trim()
            .strip_prefix("<!-- gen:")
            .and_then(|s| s.strip_suffix("-->"))
        else {
            continue;
        };
        let mut old = String::new();
        let mut closed = false;
        for l in lines.by_ref() {
            if l.trim() == "<!-- /gen -->" {
                closed = true;
                break;
            }
            old.push_str(l);
            old.push('\n');
        }
        if !closed {
            cx.errors.push(format!(
                "{page}: <!-- gen:{} --> without <!-- /gen -->",
                spec.trim()
            ));
        }
        let words = split_args(spec);
        let name = words.first().map_or("", String::as_str);
        let args: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
        let body = generate(page, name, &args, last_js.as_deref(), &old, cx);
        out.push_str(&body);
        out.push_str("<!-- /gen -->\n");
    }
    out
}

fn generate(
    page: &str,
    name: &str,
    args: &[&str],
    last_js: Option<&str>,
    old: &str,
    cx: &mut Ctx,
) -> String {
    match name {
        "value-tags" => value_tags(),
        "value-bits" => {
            cx.files.insert(
                "gen/value-bits.svg".into(),
                svg::value_bits(TAG_PREFIX >> 48),
            );
            "![The two encodings of a 64-bit value](gen/value-bits.svg)\n".into()
        }
        "cell" => {
            let mut s = String::new();
            for name in args {
                match LAYOUTS.iter().find(|l| l.name == *name) {
                    Some(l) => {
                        let file = format!("gen/cell-{name}.svg");
                        cx.files.insert(file.clone(), svg::cell(l));
                        let _ = writeln!(s, "![The {name} cell]({file})\n");
                    }
                    None => cx.errors.push(format!("{page}: no cell kind {name}")),
                }
            }
            s
        }
        "cell-all" => {
            let mut s = String::new();
            for l in LAYOUTS {
                let file = format!("gen/cell-{}.svg", l.name);
                cx.files.insert(file.clone(), svg::cell(l));
                let _ = writeln!(
                    s,
                    "## {} (kind {})\n\n![The {} cell]({file})\n",
                    l.name, l.kind, l.name
                );
            }
            s
        }
        "ops" => ops_table(args, page, cx),
        "ops-all" => {
            let names: Vec<&str> = OPS.iter().map(|o| o.name).collect();
            ops_table(&names, page, cx)
        }
        "intrinsics-all" => intrinsics_table(cx),
        "decisions" => {
            let mut s = String::new();
            for id in args {
                match cx.decisions.get(*id) {
                    Some(title) => {
                        let _ = writeln!(s, "- {id}: {title}");
                    }
                    None => cx.errors.push(format!("{page}: no decision {id}")),
                }
            }
            s
        }
        "excerpt" => excerpt(args, page, cx),
        "status" => {
            let st = cx
                .status
                .get_or_insert_with(|| crate::status::Status::compute(&crate::repo()));
            if let (["conformance"], Err(e)) = (args, &st.test262) {
                cx.warnings.push(format!(
                    "{page}: gen:status conformance kept as it was: {e}"
                ));
                return old.to_string();
            }
            match args {
                [what] => crate::status::generate(st, what).unwrap_or_else(|e| {
                    cx.errors.push(format!("{page}: {e}"));
                    String::new()
                }),
                _ => {
                    cx.errors
                        .push(format!("{page}: gen:status takes one argument"));
                    String::new()
                }
            }
        }
        "run" | "disasm" => {
            let Some(src) = last_js else {
                cx.errors
                    .push(format!("{page}: gen:{name} without a js block before it"));
                return String::new();
            };
            if name == "disasm" {
                return fenced("text", &crate::run::disasm(src));
            }
            let new = fenced("text", &crate::run::run(src));
            if new != old && !cx.examples {
                cx.changed_examples.push(page.to_string());
                return old.to_string();
            }
            new
        }
        _ => {
            cx.errors.push(format!("{page}: unknown generator {name}"));
            String::new()
        }
    }
}

/// A region's arguments: whitespace-separated words, or `"quoted strings"` (no escapes).
fn split_args(spec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = spec.trim();
    while !rest.is_empty() {
        if let Some(q) = rest.strip_prefix('"') {
            let end = q.find('"').unwrap_or(q.len());
            out.push(q[..end].to_string());
            rest = q.get(end + 1..).unwrap_or("").trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest[..end].to_string());
            rest = rest[end..].trim_start();
        }
    }
    out
}

/// `excerpt PATH ANCHOR START N`: `N` lines of a repository file, starting at the first
/// line that contains `START` after the first line that contains `ANCHOR`, so that
/// quoted code (generated code in particular) cannot drift from the file: `check` fails
/// when it changes or the anchors disappear.
fn excerpt(args: &[&str], page: &str, cx: &mut Ctx) -> String {
    let [path, anchor, start, n] = args else {
        cx.errors
            .push(format!("{page}: gen:excerpt takes PATH ANCHOR START N"));
        return String::new();
    };
    let Ok(n) = n.parse::<usize>() else {
        cx.errors
            .push(format!("{page}: gen:excerpt: bad line count {n}"));
        return String::new();
    };
    let Ok(text) = std::fs::read_to_string(crate::repo().join(path)) else {
        cx.errors
            .push(format!("{page}: gen:excerpt: cannot read {path}"));
        return String::new();
    };
    match excerpt_lines(&text, anchor, start, n) {
        Some(lines) => {
            let lang = match path.rsplit('.').next() {
                Some("rs") => "rust",
                Some(ext @ ("ts" | "js")) => ext,
                _ => "text",
            };
            fenced(lang, &dedent(&lines))
        }
        None => {
            cx.errors.push(format!(
                "{page}: gen:excerpt: {path} has no {start:?} after {anchor:?}"
            ));
            String::new()
        }
    }
}

fn excerpt_lines<'a>(text: &'a str, anchor: &str, start: &str, n: usize) -> Option<Vec<&'a str>> {
    let mut lines = text.lines().skip_while(|l| !l.contains(anchor));
    lines.next()?;
    let mut lines = lines.skip_while(|l| !l.contains(start)).peekable();
    lines.peek()?;
    Some(lines.take(n).collect())
}

/// Removes the indentation all non-empty lines share.
fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut s = String::new();
    for l in lines {
        s.push_str(l.get(indent..).unwrap_or("").trim_end());
        s.push('\n');
    }
    s
}

/// A word that points into the development records: a `.md` document or an issue number
/// (`0080`).
fn is_dev_ref(w: &str) -> bool {
    let w = w.trim_end_matches(['.', ':', ';']);
    w.ends_with(".md")
        || (w.len() == 4 && w.starts_with('0') && w.bytes().all(|b| b.is_ascii_digit()))
}

/// Drops references to developer records from a quoted source comment, so that the
/// guide stays self-contained (D2102): in each parenthesized group, the comma-separated
/// parts that name a `.md` file or an issue go, and a group left empty goes with them
/// (`only (docs/compiler.md 7.3).` -> `only.`, `(IteratorHelper, 0051)` ->
/// `(IteratorHelper)`).
fn without_dev_refs(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(len) = rest[open..].find(')') else {
            break;
        };
        let inner = &rest[open + 1..open + len];
        let kept: Vec<&str> = inner
            .split(", ")
            .filter(|part| !part.split_whitespace().any(is_dev_ref))
            .collect();
        if kept.is_empty() {
            out.push_str(rest[..open].trim_end());
        } else {
            out.push_str(&rest[..open]);
            out.push('(');
            out.push_str(&kept.join(", "));
            out.push(')');
        }
        rest = &rest[open + len + 1..];
    }
    out.push_str(rest);
    out
}

fn fenced(lang: &str, body: &str) -> String {
    format!("```{lang}\n{}\n```\n", body.trim_end())
}

fn value_tags() -> String {
    let prefix = TAG_PREFIX >> 48;
    let mut tags = EXPORTED_TAGS.to_vec();
    tags.sort_by_key(|t| t.1);
    let mut s = String::from("| tag | name | high word | payload |\n| ---: | --- | --- | --- |\n");
    for (name, t) in tags {
        let payload = if t >= tag::FIRST_REF {
            "`u32` arena offset of a cell"
        } else {
            "immediate"
        };
        let _ = writeln!(
            s,
            "| {t} | `{name}` | `0x{prefix:04X}_{t:04X}` | {payload} |"
        );
    }
    s
}

fn stack_effect(o: &OpInfo) -> String {
    let n = |base: u8, imm: Option<u8>| match imm {
        Some(i) if base == 0 => format!("imm{i}"),
        Some(i) => format!("{base}+imm{i}"),
        None => base.to_string(),
    };
    format!("{} → {}", n(o.pop, o.pop_imm), n(o.push, o.push_imm))
}

/// Table cells must not contain `|` or line breaks.
fn cell_text(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn ops_table(names: &[&str], page: &str, cx: &mut Ctx) -> String {
    let mut s = String::from(
        "| opcode | byte | immediates | size | pop → push | cost | description |\n\
         | --- | ---: | --- | ---: | --- | --- | --- |\n",
    );
    for name in names {
        let Some((i, o)) = OPS.iter().enumerate().find(|(_, o)| o.name == *name) else {
            cx.errors.push(format!("{page}: no opcode {name}"));
            continue;
        };
        let imms: Vec<String> = o.imms.iter().map(|k| format!("{k:?}")).collect();
        let doc = cx
            .op_docs
            .get(o.name)
            .map(|d| cell_text(&without_dev_refs(d)))
            .unwrap_or_default();
        let _ = writeln!(
            s,
            "| `{}` | {i} | {} | {} | {} | {:?} | {doc} |",
            o.name,
            if imms.is_empty() {
                "–".into()
            } else {
                imms.join(", ")
            },
            o.size,
            stack_effect(o),
            o.cost
        );
    }
    s
}

fn intrinsics_table(cx: &mut Ctx) -> String {
    let mut s = String::from(
        "| id | name | kind | parameters | description |\n| ---: | --- | --- | --- | --- |\n",
    );
    for (i, e) in INTRINSICS.iter().enumerate() {
        let kind = match e.kind {
            IntrinsicKind::Leaf => "leaf",
            IntrinsicKind::Reentrant => "reentrant",
            IntrinsicKind::Builtin => "builtin (JS)",
        };
        let params = format!("{}{}", e.params, if e.variadic { " + rest" } else { "" });
        let doc = cx
            .intrinsic_docs
            .get(e.name)
            .map(|d| cell_text(&without_dev_refs(d)))
            .unwrap_or_default();
        let _ = writeln!(s, "| {i} | `{}` | {kind} | {params} | {doc} |", e.name);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_refs_are_dropped_from_quoted_comments() {
        assert_eq!(without_dev_refs("only (docs/compiler.md 7.3)."), "only.");
        assert_eq!(
            without_dev_refs("A label (symbolic labels, docs/compiler.md 4.3), more (x)."),
            "A label (symbolic labels), more (x)."
        );
        assert_eq!(
            without_dev_refs("of the realm (0039). Slot i (IteratorHelper, 0051)."),
            "of the realm. Slot i (IteratorHelper)."
        );
    }

    #[test]
    fn args_may_be_quoted() {
        assert_eq!(
            split_args(r#"excerpt a/b.rs "fn x(" "5 => {" 3 "#),
            ["excerpt", "a/b.rs", "fn x(", "5 => {", "3"]
        );
    }

    #[test]
    fn excerpts_start_after_the_anchor() {
        let text = "fn a() {\n    5 => {}\n}\nfn b() {\n    5 => {\n        x\n    }\n}\n";
        let lines = excerpt_lines(text, "fn b(", "5 =>", 2).unwrap();
        assert_eq!(dedent(&lines), "5 => {\n    x\n");
        assert!(excerpt_lines(text, "fn c(", "5 =>", 2).is_none());
        assert!(excerpt_lines(text, "fn b(", "6 =>", 2).is_none());
    }

    fn cx() -> Ctx {
        Ctx::new(
            &Sources {
                decisions_md: "## D1050: Value encoding\ntext\n## D007: Internal tag\n".into(),
                ops_rs: "x\nops! {\n    /// `-> undefined`\n    Undefined undefined() pop 0 push 1 cost Basic;\n    /// `a b -> a+b`\n    /// more\n    Add add() pop 2 push 1 cost Basic;\n    In in_() pop 2 push 1 cost Access;\n}\n".into(),
                intrinsics_dts: "/** IsArray ( argument ), 7.2.2. @leaf */\ndeclare function IsArray(v: Value): boolean;\n/**\n * Two\n * lines. @reentrant\n */\ndeclare function Call(f: Fn): Value;\n".into(),
            },
            false,
        )
    }

    #[test]
    fn sources_are_parsed() {
        let c = cx();
        assert_eq!(
            c.decisions.get("D1050").map(String::as_str),
            Some("Value encoding")
        );
        assert_eq!(
            c.decisions.get("D007").map(String::as_str),
            Some("Internal tag")
        );
        assert_eq!(c.op_docs["undefined"], "`-> undefined`");
        assert_eq!(c.op_docs["add"], "`a b -> a+b` more");
        assert_eq!(c.op_docs["in"], "");
        assert_eq!(c.intrinsic_docs["IsArray"], "IsArray ( argument ), 7.2.2.");
        assert_eq!(c.intrinsic_docs["Call"], "Two lines.");
    }

    #[test]
    fn regions_are_replaced_and_kept_in_place() {
        let mut c = cx();
        let md = "# T\n\n<!-- gen:decisions D1050 -->\nold\n<!-- /gen -->\ntail\n";
        let out = regen("p.md", md, &mut c);
        assert_eq!(
            out,
            "# T\n\n<!-- gen:decisions D1050 -->\n- D1050: Value encoding\n<!-- /gen -->\ntail\n"
        );
        assert!(c.errors.is_empty());
        // Idempotent.
        assert_eq!(regen("p.md", &out, &mut c), out);
    }

    #[test]
    fn unknown_references_are_errors() {
        let mut c = cx();
        regen(
            "p.md",
            "<!-- gen:decisions D9 -->\n<!-- /gen -->\n<!-- gen:ops nope -->\n<!-- /gen -->\n<!-- gen:cell Nope -->\n<!-- /gen -->\n<!-- gen:what -->\n",
            &mut c,
        );
        assert_eq!(c.errors.len(), 5, "{:?}", c.errors);
    }

    #[test]
    fn example_output_changes_need_examples() {
        let md =
            "```js\nconsole.log(1 + 1)\n```\n<!-- gen:run -->\n```text\n3\n```\n<!-- /gen -->\n";
        let mut c = cx();
        assert_eq!(regen("p.md", md, &mut c), md);
        assert_eq!(c.changed_examples, ["p.md"]);
        let mut c = cx();
        c.examples = true;
        assert!(regen("p.md", md, &mut c).contains("```text\n2\n```"));
    }

    #[test]
    fn real_tables_have_docs() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let read = |p: &str| std::fs::read_to_string(format!("{root}/{p}")).unwrap();
        let c = Ctx::new(
            &Sources {
                decisions_md: read("docs/decisions.md"),
                ops_rs: read("crates/djs-core/src/bytecode/ops.rs"),
                intrinsics_dts: read("builtins/intrinsics.d.ts"),
            },
            false,
        );
        // Every opcode has an entry (possibly empty); most have docs.
        for o in OPS {
            assert!(
                c.op_docs.contains_key(o.name),
                "no ops! entry found for {}",
                o.name
            );
        }
        assert!(c.op_docs.values().filter(|d| !d.is_empty()).count() > OPS.len() / 2);
        assert!(c.intrinsic_docs.contains_key("IsArray"));
    }
}
