//! A small syntax highlighter for JS/TS and Rust: keywords, strings, numbers, comments.
//! (Phase 2 could drive it with djs's own lexer instead.)

const JS_KW: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "yield",
    "type",
    "interface",
    "as",
];
const RS_KW: &[&str] = &[
    "as", "const", "else", "enum", "false", "fn", "for", "if", "impl", "let", "loop", "match",
    "mod", "mut", "pub", "return", "self", "Self", "struct", "true", "use", "where", "while",
];

pub fn highlight(lang: &str, code: &str) -> String {
    let kw = match lang {
        "js" | "javascript" | "ts" | "typescript" => JS_KW,
        "rust" | "rs" => RS_KW,
        _ => return esc(code),
    };
    let c: Vec<char> = code.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let span = |out: &mut String, class: &str, s: &str| {
        out.push_str(&format!("<span class=\"tok-{class}\">{}</span>", esc(s)));
    };
    while i < c.len() {
        let ch = c[i];
        let rest = |j: usize| c[i..j].iter().collect::<String>();
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            let mut j = i;
            while j < c.len() && c[j] != '\n' {
                j += 1;
            }
            span(&mut out, "comment", &rest(j));
            i = j;
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            let mut j = i + 2;
            while j + 1 < c.len() && !(c[j] == '*' && c[j + 1] == '/') {
                j += 1;
            }
            j = (j + 2).min(c.len());
            span(&mut out, "comment", &rest(j));
            i = j;
        } else if ch == '"' || ch == '\'' || ch == '`' {
            let mut j = i + 1;
            while j < c.len() && c[j] != ch {
                if c[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            j = (j + 1).min(c.len());
            span(&mut out, "string", &rest(j));
            i = j;
        } else if ch.is_ascii_digit() {
            let mut j = i;
            while j < c.len() && (c[j].is_ascii_alphanumeric() || c[j] == '.' || c[j] == '_') {
                j += 1;
            }
            span(&mut out, "number", &rest(j));
            i = j;
        } else if ch.is_alphabetic() || ch == '_' || ch == '$' {
            let mut j = i;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_' || c[j] == '$') {
                j += 1;
            }
            let w = rest(j);
            if kw.contains(&w.as_str()) {
                span(&mut out, "keyword", &w);
            } else if c.get(j) == Some(&'(') {
                span(&mut out, "call", &w);
            } else {
                out.push_str(&esc(&w));
            }
            i = j;
        } else {
            out.push_str(&esc(&ch.to_string()));
            i += 1;
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
