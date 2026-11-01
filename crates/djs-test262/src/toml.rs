//! A parser for the small TOML subset used by `tests/test262.toml`.
//!
//! Supported: `#` comments, `[table]` and `[[array-of-tables]]` headers with bare
//! names, `key = value` with bare keys, and values that are basic strings (`"..."` with
//! `\\`, `\"`, `\n`, `\t` escapes), booleans, or arrays of strings that may span lines
//! and have a trailing comma. Anything else is an error, so typos do not pass silently.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(String),
    Bool(bool),
    Array(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Empty for the implicit root table.
    pub name: String,
    /// True for `[[name]]`.
    pub is_array_item: bool,
    pub entries: Vec<(String, Value)>,
    /// 1-based line of the header (0 for the root), for error messages.
    pub line: usize,
}

/// Parses a document into its tables, in file order. The first table is the root.
pub fn parse(text: &str) -> Result<Vec<Table>, String> {
    let mut tables = vec![Table {
        name: String::new(),
        is_array_item: false,
        entries: Vec::new(),
        line: 0,
    }];
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        line: 1,
    };
    loop {
        p.skip_ws_comments_newlines();
        let Some(c) = p.peek() else { break };
        let line = p.line;
        if c == b'[' {
            let is_array_item = p.s.get(p.i + 1) == Some(&b'[');
            p.i += if is_array_item { 2 } else { 1 };
            p.skip_inline_ws();
            let name = p.bare_key()?;
            p.skip_inline_ws();
            let close: &[u8] = if is_array_item { b"]]" } else { b"]" };
            if !p.s[p.i..].starts_with(close) {
                return Err(p.err("expected `]` after table name"));
            }
            p.i += close.len();
            p.end_of_line()?;
            if !is_array_item && tables.iter().any(|t| t.name == name && !t.is_array_item) {
                return Err(p.err(&format!("duplicate table [{name}]")));
            }
            tables.push(Table {
                name,
                is_array_item,
                entries: Vec::new(),
                line,
            });
            continue;
        }
        let key = p.bare_key()?;
        p.skip_inline_ws();
        if p.peek() != Some(b'=') {
            return Err(p.err("expected `=` after key"));
        }
        p.i += 1;
        p.skip_inline_ws();
        let value = p.value()?;
        p.end_of_line()?;
        let table = tables.last_mut().unwrap();
        if table.entries.iter().any(|(k, _)| *k == key) {
            return Err(format!("line {line}: duplicate key `{key}`"));
        }
        table.entries.push((key, value));
    }
    Ok(tables)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    line: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn err(&self, msg: &str) -> String {
        format!("line {}: {msg}", self.line)
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some(b'#') {
            while !matches!(self.peek(), None | Some(b'\n')) {
                self.i += 1;
            }
        }
    }

    fn skip_ws_comments_newlines(&mut self) {
        loop {
            self.skip_inline_ws();
            self.skip_comment();
            match self.peek() {
                Some(b'\n') => {
                    self.i += 1;
                    self.line += 1;
                }
                Some(b'\r') => self.i += 1,
                _ => break,
            }
        }
    }

    fn end_of_line(&mut self) -> Result<(), String> {
        self.skip_inline_ws();
        self.skip_comment();
        if self.peek() == Some(b'\r') {
            self.i += 1;
        }
        match self.peek() {
            None => Ok(()),
            Some(b'\n') => {
                self.i += 1;
                self.line += 1;
                Ok(())
            }
            _ => Err(self.err("unexpected text at end of line")),
        }
    }

    fn bare_key(&mut self) -> Result<String, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            self.i += 1;
        }
        if start == self.i {
            return Err(self.err("expected a bare key"));
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"') => self.string().map(Value::Str),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws_comments_newlines();
                    match self.peek() {
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Array(items));
                        }
                        Some(b'"') => items.push(self.string()?),
                        _ => return Err(self.err("expected a string or `]` in array")),
                    }
                    self.skip_ws_comments_newlines();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {}
                        _ => return Err(self.err("expected `,` or `]` in array")),
                    }
                }
            }
            _ if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Value::Bool(true))
            }
            _ if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Value::Bool(false))
            }
            _ => Err(self.err("expected a value (string, boolean or array of strings)")),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // opening quote
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(out).map_err(|_| self.err("invalid UTF-8 in string"));
                }
                Some(b'\\') => {
                    let c = match self.s.get(self.i + 1) {
                        Some(b'\\') => b'\\',
                        Some(b'"') => b'"',
                        Some(b'n') => b'\n',
                        Some(b't') => b'\t',
                        _ => return Err(self.err("unsupported escape in string")),
                    };
                    out.push(c);
                    self.i += 2;
                }
                Some(c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document() {
        let doc = r#"
# comment
version = "1"   # trailing

[[group]]
name = "skip"
enabled = true
flags = [
  "noStrict",  # why
  "module",    # why
]

[[group]]
name = "later"
features = ["a", "b\"c"]
empty = []
"#;
        let t = parse(doc).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(
            t[0].entries,
            [("version".to_string(), Value::Str("1".into()))]
        );
        assert!(t[1].is_array_item && t[1].name == "group" && t[1].line == 5);
        assert_eq!(t[1].entries[1].1, Value::Bool(true));
        assert_eq!(
            t[1].entries[2].1,
            Value::Array(vec!["noStrict".into(), "module".into()])
        );
        assert_eq!(
            t[2].entries[1].1,
            Value::Array(vec!["a".into(), "b\"c".into()])
        );
        assert_eq!(t[2].entries[2].1, Value::Array(vec![]));
    }

    #[test]
    fn errors() {
        assert!(parse("a = 1").is_err());
        assert!(parse("a = \"x").is_err());
        assert!(parse("a = [\"x\" \"y\"]").is_err());
        assert!(parse("a = \"x\" b").is_err());
        assert!(parse("a = \"x\"\na = \"y\"").is_err());
        assert!(parse("[t]\n[t]").is_err());
        assert!(parse("[[t]\n").is_err());
    }
}
