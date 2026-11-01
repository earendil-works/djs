//! A cheap source scan for constructs that have no test262 feature tag, such as direct
//! `eval(` or the `Function(` constructor.
//!
//! A pattern is a literal string. It matches when it occurs outside a comment and is
//! not preceded by an identifier character, so `eval(` matches `eval(x)` and
//! `globalThis.eval(x)` but not `myeval(x)`. String contents are not excluded (a
//! pattern inside a string literal still matches): the scan may be conservative.

/// Removes `//` and `/* */` comments, keeping string and template literal contents.
/// Quote tracking for `'` and `"` stops at the end of a line, so a regex literal
/// containing a quote character confuses at most one line.
pub fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            out.push(c);
            if c == b'\\' && i + 1 < b.len() {
                out.push(b[i + 1]);
                i += 2;
                continue;
            }
            if c == q || (c == b'\n' && q != b'`') {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'\'' | b'"' | b'`' => {
                quote = Some(c);
                out.push(c);
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    // Keep line breaks so the stripped text has the same lines.
                    if b[i] == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
                i += 2;
                out.push(b' ');
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    // Only ASCII bytes were removed or inserted, at ASCII positions.
    String::from_utf8(out).unwrap_or_default()
}

fn is_ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

/// True if `pattern` occurs in `code` (already comment-stripped) at an identifier
/// boundary on its left.
pub fn contains_pattern(code: &str, pattern: &str) -> bool {
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(pos) = code[from..].find(pattern) {
        let at = from + pos;
        if at == 0 || !is_ident_byte(bytes[at - 1]) {
            return true;
        }
        from = at + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_ignored() {
        let src = "// eval(x)\n/* Function(\n */ a = 1; '//' + \"/*\"; b = `\n// x`;";
        let s = strip_comments(src);
        assert!(!s.contains("eval"));
        assert!(!s.contains("Function"));
        assert!(s.contains("'//'") && s.contains("\"/*\"") && s.contains("// x"));
        assert_eq!(s.lines().count(), src.lines().count());
    }

    #[test]
    fn quotes_reset_at_line_end() {
        // A regex literal with a quote must not hide the next line.
        let s = strip_comments("var r = /'/;\neval(1); // eval(2)");
        assert!(contains_pattern(&s, "eval("));
        assert!(!s.contains("eval(2)"));
    }

    #[test]
    fn boundaries() {
        assert!(contains_pattern("eval('1')", "eval("));
        assert!(contains_pattern("x = globalThis.eval('1')", "eval("));
        assert!(contains_pattern("new Function('a')", "Function("));
        assert!(!contains_pattern("myeval('1')", "eval("));
        assert!(!contains_pattern("$eval('1')", "eval("));
        assert!(!contains_pattern("GeneratorFunction('1')", "Function("));
        assert!(!contains_pattern("(0, eval)('1')", "eval("));
        assert!(contains_pattern("(0, eval)('1')", "eval)("));
    }
}
