//! A small syntax highlighter for JS/TS and Rust. It marks the roles of Elanor's Shiki
//! theme (comment, punctuation, keyword, function, variable, string, number, type), and
//! style.css colors them with the same tokens the theme uses (D2200). Running Shiki
//! itself would need node and npm in every build for little visible difference.

const JS_KW: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "of",
    "return",
    "set",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "type",
    "typeof",
    "var",
    "void",
    "while",
    "yield",
];
/// Constants: Shiki's `constant.language`, colored like numbers.
const JS_CONST: &[&str] = &["true", "false", "null", "undefined", "NaN", "Infinity"];
const JS_TYPES: &[&str] = &[
    "number", "string", "boolean", "bigint", "symbol", "object", "any", "unknown", "never",
];
const RS_KW: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where",
    "while",
];
const RS_CONST: &[&str] = &["true", "false"];
const RS_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char", "str",
];

struct Lang {
    kw: &'static [&'static str],
    consts: &'static [&'static str],
    types: &'static [&'static str],
    rust: bool,
}

pub fn highlight(lang: &str, code: &str) -> String {
    let l = match lang {
        "js" | "javascript" | "ts" | "typescript" => Lang {
            kw: JS_KW,
            consts: JS_CONST,
            types: JS_TYPES,
            rust: false,
        },
        "rust" | "rs" => Lang {
            kw: RS_KW,
            consts: RS_CONST,
            types: RS_TYPES,
            rust: true,
        },
        _ => return esc(code),
    };
    let c: Vec<char> = code.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let span = |out: &mut String, class: &str, s: &str| {
        out.push_str(&format!("<span class=\"tok-{class}\">{}</span>", esc(s)));
    };
    // The previous significant word, for `fn name` / `function name`.
    let mut prev_word = String::new();
    // A run of punctuation goes into one span, to keep the HTML small.
    let mut punct = String::new();
    while i < c.len() {
        let ch = c[i];
        if !ch.is_whitespace() && !ch.is_alphanumeric() && !"_$/\"'`".contains(ch) {
            punct.push(ch);
            prev_word.clear();
            i += 1;
            continue;
        }
        if ch == '/' && !matches!(c.get(i + 1), Some('/' | '*')) {
            punct.push(ch);
            i += 1;
            continue;
        }
        if !punct.is_empty() {
            span(&mut out, "punctuation", &punct);
            punct.clear();
        }
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
        } else if ch == '"' || (ch == '`' && !l.rust) || (ch == '\'' && !rust_lifetime(&l, &c, i)) {
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
        } else if ch.is_alphabetic() || ch == '_' || ch == '$' || ch == '\'' {
            // ('\'' here only starts a Rust lifetime.)
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_' || c[j] == '$') {
                j += 1;
            }
            let w = rest(j);
            // A Rust macro call: `name!(`.
            let bang = l.rust && c.get(j) == Some(&'!') && c.get(j + 1) != Some(&'=');
            let class = if l.kw.contains(&w.as_str()) {
                "keyword"
            } else if l.consts.contains(&w.as_str()) {
                "number"
            } else if l.types.contains(&w.as_str())
                || w.starts_with(|c: char| c.is_ascii_uppercase())
            {
                "type"
            } else if c.get(j) == Some(&'(')
                || bang
                || matches!(prev_word.as_str(), "fn" | "function")
            {
                "function"
            } else {
                "variable"
            };
            span(&mut out, class, &w);
            if bang {
                span(&mut out, "function", "!");
                j += 1;
            }
            prev_word = w;
            i = j;
        } else {
            // Whitespace; and `'` (a JS string) cannot get here.
            out.push(ch);
            i += 1;
        }
    }
    if !punct.is_empty() {
        span(&mut out, "punctuation", &punct);
    }
    out
}

/// `'a` in Rust is a lifetime, not a character literal (`'a'`).
fn rust_lifetime(l: &Lang, c: &[char], i: usize) -> bool {
    l.rust
        && c.get(i + 1).is_some_and(|c| c.is_alphabetic() || *c == '_')
        && c.get(i + 2) != Some(&'\'')
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn roles() {
        let h = highlight("js", "const x = f(1, \"a\"); // c\nnew Map(null)");
        for want in [
            "<span class=\"tok-keyword\">const</span>",
            "<span class=\"tok-variable\">x</span>",
            "<span class=\"tok-function\">f</span>",
            "<span class=\"tok-number\">1</span>",
            "<span class=\"tok-string\">\"a\"</span>",
            "<span class=\"tok-comment\">// c</span>",
            "<span class=\"tok-type\">Map</span>",
            "<span class=\"tok-number\">null</span>",
            "<span class=\"tok-punctuation\">);</span>",
        ] {
            assert!(h.contains(want), "{want} in {h}");
        }
    }

    /// The live examples' editor highlights with assets/highlight.js, a port of the JS
    /// half of this file: it must give the same HTML for every JS/TS block of the guide
    /// and for the corner cases below (D2103). Needs node, like the rest of the gate.
    #[test]
    fn js_port_agrees() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut samples: Vec<String> = [
            "const x = f(1, \"a\"); // c\nnew Map(null)",
            "a / b /* c */ / d; x = /re/g; `t ${x} \\` u`; 'it\\'s'",
            "function f() {} async function* g() {} fn h",
            "naïve → done; ünïcode_$ x1 1_000n 0x1F 1.5e3 .5",
            "type T = { a: number; b: string | unknown }; interface I extends Base {}",
            "unterminated \"string\nand /* comment",
            "\t tab\u{a0}nbsp\u{2028}ls ",
            "",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for e in std::fs::read_dir(root.join("../../docs/guide")).unwrap() {
            let text = std::fs::read_to_string(e.unwrap().path()).unwrap_or_default();
            let mut block: Option<String> = None;
            for line in text.lines() {
                match &mut block {
                    None if line.starts_with("```js") || line.starts_with("```ts") => {
                        block = Some(String::new())
                    }
                    Some(b) if line == "```" => {
                        samples.push(std::mem::take(b));
                        block = None;
                    }
                    Some(b) => {
                        b.push_str(line);
                        b.push('\n');
                    }
                    None => {}
                }
            }
        }
        assert!(samples.len() > 20, "the guide's examples were not found");
        let script = format!(
            "import {{ highlight }} from {:?};\n\
             let s = ''; process.stdin.on('data', (d) => (s += d));\n\
             process.stdin.on('end', () => process.stdout.write(JSON.stringify(JSON.parse(s).map(highlight))));",
            format!("file://{}", root.join("assets/highlight.js").display())
        );
        let mut child = std::process::Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("node is needed to test assets/highlight.js");
        let input = format!(
            "[{}]",
            samples
                .iter()
                .map(|s| json_str(s))
                .collect::<Vec<_>>()
                .join(",")
        );
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "node failed");
        let got = String::from_utf8(out.stdout).unwrap();
        let want = format!(
            "[{}]",
            samples
                .iter()
                .map(|s| json_str(&highlight("js", s)))
                .collect::<Vec<_>>()
                .join(",")
        );
        if got != want {
            let (g, w): (Vec<char>, Vec<char>) = (got.chars().collect(), want.chars().collect());
            let at = g.iter().zip(&w).take_while(|(a, b)| a == b).count();
            let ctx = |v: &[char]| {
                v[at.saturating_sub(80)..(at + 80).min(v.len())]
                    .iter()
                    .collect::<String>()
            };
            panic!(
                "assets/highlight.js differs from highlight.rs\n js: {}\n rs: {}",
                ctx(&g),
                ctx(&w)
            );
        }
    }

    /// JSON.stringify's form of a string, so the two outputs compare as text.
    fn json_str(s: &str) -> String {
        let mut o = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                '\u{8}' => o.push_str("\\b"),
                '\u{c}' => o.push_str("\\f"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    }

    #[test]
    fn rust_specials() {
        let h = highlight("rust", "fn get<'a>(x: &'a u32) -> char { vec![b'x'] }");
        assert!(h.contains("<span class=\"tok-function\">get</span>"), "{h}");
        assert!(h.contains("<span class=\"tok-variable\">'a</span>"), "{h}");
        assert!(h.contains("<span class=\"tok-type\">u32</span>"), "{h}");
        assert!(
            h.contains(
                "<span class=\"tok-function\">vec</span><span class=\"tok-function\">!</span>"
            ),
            "{h}"
        );
        assert!(h.contains("<span class=\"tok-string\">'x'</span>"), "{h}");
    }
}
