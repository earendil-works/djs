//! Parser for the YAML subset used in test262 frontmatter (`/*--- ... ---*/`).
//!
//! test262 frontmatter is regular enough that a full YAML parser is not needed:
//! top-level keys start in column 0, and their values are one of
//! - a plain or quoted scalar on the same line (possibly continued on indented lines),
//! - a block scalar (`|` or `>`, with optional chomping indicator) on indented lines,
//! - a flow sequence `[a, b]` or flow mapping `{a: b}` on the same line,
//! - a block sequence (`- a`) or block mapping (`a: b`) on indented lines.
//!
//! Only `description`, `includes`, `flags`, `features` and `negative` are interpreted;
//! everything else is skipped.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Parse,
    Resolution,
    Runtime,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Parse => "parse",
            Phase::Resolution => "resolution",
            Phase::Runtime => "runtime",
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negative {
    pub phase: Phase,
    /// Constructor name of the expected error, e.g. `SyntaxError`.
    pub error_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    pub description: String,
    pub includes: Vec<String>,
    pub flags: Vec<String>,
    pub features: Vec<String>,
    pub negative: Option<Negative>,
}

impl Meta {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

/// A value as written in the frontmatter, before interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Scalar(String),
    List(Vec<String>),
    Map(Vec<(String, String)>),
}

/// Extracts the frontmatter text between `/*---` and `---*/`.
pub fn extract(source: &str) -> Option<&str> {
    let start = source.find("/*---")? + "/*---".len();
    let len = source[start..].find("---*/")?;
    Some(&source[start..start + len])
}

/// Parses the frontmatter of a test file (the whole source).
pub fn parse_source(source: &str) -> Result<Meta, String> {
    let fm = extract(source).ok_or("no /*--- ... ---*/ frontmatter")?;
    parse(fm)
}

/// Parses frontmatter text (without the `/*---` / `---*/` delimiters).
pub fn parse(text: &str) -> Result<Meta, String> {
    let mut meta = Meta::default();
    for (key, value) in parse_entries(text)? {
        match key.as_str() {
            "description" => meta.description = expect_scalar(&key, value)?,
            "includes" => meta.includes = expect_list(&key, value)?,
            "flags" => meta.flags = expect_list(&key, value)?,
            "features" => meta.features = expect_list(&key, value)?,
            "negative" => meta.negative = Some(parse_negative(value)?),
            _ => {}
        }
    }
    Ok(meta)
}

fn expect_scalar(key: &str, value: Value) -> Result<String, String> {
    match value {
        Value::Scalar(s) => Ok(s),
        _ => Err(format!("`{key}` must be a scalar")),
    }
}

fn expect_list(key: &str, value: Value) -> Result<Vec<String>, String> {
    match value {
        Value::List(items) => Ok(items),
        // `flags:` with nothing after it (rare, but valid YAML for null).
        Value::Scalar(s) if s.is_empty() => Ok(Vec::new()),
        _ => Err(format!("`{key}` must be a list")),
    }
}

fn parse_negative(value: Value) -> Result<Negative, String> {
    let Value::Map(entries) = value else {
        return Err("`negative` must be a mapping".into());
    };
    let mut phase = None;
    let mut error_type = None;
    for (k, v) in entries {
        match k.as_str() {
            "phase" => {
                phase = Some(match v.as_str() {
                    "parse" => Phase::Parse,
                    "resolution" => Phase::Resolution,
                    "runtime" => Phase::Runtime,
                    other => return Err(format!("unknown negative phase `{other}`")),
                })
            }
            "type" => error_type = Some(v),
            _ => {}
        }
    }
    match (phase, error_type) {
        (Some(phase), Some(error_type)) if !error_type.is_empty() => {
            Ok(Negative { phase, error_type })
        }
        _ => Err("`negative` needs both `phase` and `type`".into()),
    }
}

/// Splits the frontmatter into top-level `(key, value)` entries.
fn parse_entries(text: &str) -> Result<Vec<(String, Value)>, String> {
    // Some tests use CR or CRLF line terminators on purpose.
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = text.split('\n').collect();
    // A few frontmatters are indented as a whole; the first key sets the base.
    let base = lines
        .iter()
        .find(|l| !is_blank_or_comment(l))
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    if base > 0 {
        for line in &mut lines {
            let indent = line.len() - line.trim_start().len();
            *line = &line[indent.min(base)..];
        }
    }
    let mut entries = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if is_blank_or_comment(line) {
            i += 1;
            continue;
        }
        if line.starts_with([' ', '\t']) {
            return Err(format!("unexpected indented line: `{}`", line.trim()));
        }
        let (key, rest) = split_key(line).ok_or_else(|| format!("expected `key:`: `{line}`"))?;
        // The value's continuation lines: everything up to the next column-0 line.
        let mut end = i + 1;
        while end < lines.len()
            && (lines[end].trim().is_empty() || lines[end].starts_with([' ', '\t']))
        {
            end += 1;
        }
        let body = &lines[i + 1..end];
        let value = parse_value(key, rest, body)?;
        entries.push((key.to_string(), value));
        i = end;
    }
    Ok(entries)
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#')
}

/// Splits `key: rest` (or `key:`). Keys are plain words.
fn split_key(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let key = &line[..colon];
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let rest = &line[colon + 1..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    Some((key, rest.trim()))
}

fn parse_value(key: &str, rest: &str, body: &[&str]) -> Result<Value, String> {
    let rest = strip_comment(rest);
    if let Some(indicator) = rest.strip_prefix(['|', '>']) {
        if !indicator
            .chars()
            .all(|c| matches!(c, '-' | '+' | '0'..='9'))
        {
            return Err(format!("`{key}`: bad block scalar header `{rest}`"));
        }
        let folded = rest.starts_with('>');
        return Ok(Value::Scalar(block_scalar(body, folded)));
    }
    if rest.starts_with('[') {
        return parse_flow_list(rest)
            .map(Value::List)
            .map_err(|e| format!("`{key}`: {e}"));
    }
    if rest.starts_with('{') {
        return parse_flow_map(rest)
            .map(Value::Map)
            .map_err(|e| format!("`{key}`: {e}"));
    }
    if !rest.is_empty() {
        // A scalar, possibly continued on indented lines (plain multi-line scalar).
        let mut s = rest.to_string();
        for line in body {
            let t = line.trim();
            if !t.is_empty() {
                s.push(' ');
                s.push_str(t);
            }
        }
        return Ok(Value::Scalar(unquote(&s)));
    }
    // Nothing on the key line: a block sequence, a block mapping or an empty value.
    let items: Vec<&str> = body
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if items.is_empty() {
        return Ok(Value::Scalar(String::new()));
    }
    if items.iter().all(|l| *l == "-" || l.starts_with("- ")) {
        return Ok(Value::List(
            items
                .iter()
                .map(|l| unquote(strip_comment(l[1..].trim())))
                .collect(),
        ));
    }
    let mut map = Vec::new();
    for item in items {
        let (k, v) = split_key(item)
            .ok_or_else(|| format!("`{key}`: expected `key: value`, got `{item}`"))?;
        map.push((k.to_string(), unquote(strip_comment(v))));
    }
    Ok(Value::Map(map))
}

/// Joins the lines of a block scalar, removing the common indentation. Literal (`|`)
/// keeps newlines; folded (`>`) joins non-empty lines with spaces. Trailing newlines
/// are dropped (we never need them).
fn block_scalar(body: &[&str], folded: bool) -> String {
    let indent = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let lines: Vec<&str> = body
        .iter()
        .map(|l| if l.len() >= indent { &l[indent..] } else { "" })
        .collect();
    let mut out = String::new();
    if folded {
        let mut prev_blank = true;
        for line in &lines {
            if line.trim().is_empty() {
                out.push('\n');
                prev_blank = true;
            } else {
                if !prev_blank {
                    out.push(' ');
                }
                out.push_str(line.trim_end());
                prev_blank = false;
            }
        }
    } else {
        out = lines.join("\n");
    }
    out.trim_end().to_string()
}

fn parse_flow_list(s: &str) -> Result<Vec<String>, String> {
    let inner = s
        .strip_prefix('[')
        .and_then(|s| strip_comment(s).strip_suffix(']'))
        .ok_or("unterminated flow list (must be on one line)")?;
    Ok(split_flow(inner)
        .into_iter()
        .map(|item| unquote(item.trim()))
        .filter(|i| !i.is_empty())
        .collect())
}

fn parse_flow_map(s: &str) -> Result<Vec<(String, String)>, String> {
    let inner = s
        .strip_prefix('{')
        .and_then(|s| strip_comment(s).strip_suffix('}'))
        .ok_or("unterminated flow mapping (must be on one line)")?;
    let mut map = Vec::new();
    for item in split_flow(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let colon = item
            .find(':')
            .ok_or_else(|| format!("expected `key: value`, got `{item}`"))?;
        map.push((
            item[..colon].trim().to_string(),
            unquote(item[colon + 1..].trim()),
        ));
    }
    Ok(map)
}

/// Splits on commas outside of quotes.
fn split_flow(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, ',') => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Removes a trailing ` # comment` outside of quotes.
fn strip_comment(s: &str) -> &str {
    let mut quote = None;
    let mut prev_space = true;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '#') if prev_space => return s[..i].trim_end(),
            _ => {}
        }
        prev_space = c == ' ' || c == '\t';
    }
    s
}

/// Removes YAML quotes from a scalar. Double-quoted scalars get the common escapes.
fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        return s[1..s.len() - 1].replace("''", "'");
    }
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let mut out = String::new();
        let mut chars = s[1..s.len() - 1].chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => out.push(c),
                None => out.push('\\'),
            }
        }
        return out;
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real samples from test262 (7ab7faf), trimmed where noted.

    const ARRAY_FROM: &str = r#"// Copyright (C) 2015 the V8 project authors. All rights reserved.
/*---
esid: sec-array.from
description: Error invoking map function (traversed via iterator)
info: |
    [...]
    6. If usingIterator is not undefined, then
       [...]
       g. Repeat
          vii. If mapping is true, then
               1. Let mappedValue be Call(mapfn, T, «nextValue, k»).
features: [Symbol.iterator]
---*/

var closeCount = 0;
"#;

    const NEGATIVE: &str = r#"/*---
description: >
  ImportCall parameter list forwards the Yield production parameter - invalid IdentifierReference
esid: sec-import-call-runtime-semantics-evaluation
info: |
  ImportCall[Yield, Await]:
    import ( AssignmentExpression[+In, ?Yield, ?Await] ,opt )
features: [dynamic-import, import-attributes]
flags: [onlyStrict]
negative:
  phase: parse
  type: SyntaxError
---*/

$DONOTEVALUATE();
"#;

    const BLOCK_LIST: &str = r#"/*---
esid: sec-weakmap-iterable
description: >
  Closes the iterator object after not object error on next item.
info: |
  WeakMap ( [ iterable ] )

  ...
  9. Repeat
    ...
    f. If Type(nextItem) is not Object,
      i. Let error be Completion{[[type]]: throw, [[value]]: a newly created
      TypeError object, [[target]]:empty}.
features:
  - Symbol
  - Symbol.iterator
---*/
"#;

    const QUOTED: &str = r#"/*---
info: Operator remove leading StrWhiteSpaceChar
esid: sec-parseint-string-radix
description: "StrWhiteSpaceChar :: FF (U+000C)"
---*/
"#;

    const ASYNC: &str = r#"/*---
esid: sec-promise-resolve-functions
description: >
    Resolve function is called after Promise is resolved, the second
    value is ignored.
info: |
    25.4.1.3.2 Promise Resolve Functions
includes: [promiseHelper.js, compareArray.js]
flags: [async]
---*/
"#;

    // Indented four spaces (22 tests in test262 are written like this).
    const NEGATIVE_WIDE: &str = "/*---\ndescription: x\nnegative:\n    phase: runtime\n    type: ReferenceError\nflags: [raw, generated]\n---*/";

    #[test]
    fn flow_list_and_block_info() {
        let m = parse_source(ARRAY_FROM).unwrap();
        assert_eq!(
            m.description,
            "Error invoking map function (traversed via iterator)"
        );
        assert_eq!(m.features, ["Symbol.iterator"]);
        assert!(m.includes.is_empty() && m.flags.is_empty() && m.negative.is_none());
    }

    #[test]
    fn negative_mapping() {
        let m = parse_source(NEGATIVE).unwrap();
        assert_eq!(
            m.description,
            "ImportCall parameter list forwards the Yield production parameter - invalid IdentifierReference"
        );
        assert_eq!(m.features, ["dynamic-import", "import-attributes"]);
        assert_eq!(m.flags, ["onlyStrict"]);
        assert_eq!(
            m.negative,
            Some(Negative {
                phase: Phase::Parse,
                error_type: "SyntaxError".into()
            })
        );
        let m = parse_source(NEGATIVE_WIDE).unwrap();
        assert_eq!(m.negative.as_ref().unwrap().phase, Phase::Runtime);
        assert!(m.has_flag("raw") && m.has_flag("generated"));
    }

    #[test]
    fn block_list_after_info_with_blank_lines() {
        let m = parse_source(BLOCK_LIST).unwrap();
        assert_eq!(m.features, ["Symbol", "Symbol.iterator"]);
        assert_eq!(
            m.description,
            "Closes the iterator object after not object error on next item."
        );
    }

    #[test]
    fn quoted_description() {
        let m = parse_source(QUOTED).unwrap();
        assert_eq!(m.description, "StrWhiteSpaceChar :: FF (U+000C)");
    }

    #[test]
    fn includes_and_folded_description() {
        let m = parse_source(ASYNC).unwrap();
        assert_eq!(m.includes, ["promiseHelper.js", "compareArray.js"]);
        assert_eq!(m.flags, ["async"]);
        assert_eq!(
            m.description,
            "Resolve function is called after Promise is resolved, the second value is ignored."
        );
    }

    #[test]
    fn misc_forms() {
        let m = parse(
            "flags: []\nfeatures: ['a', \"b\"] # c\nnegative: {phase: parse, type: SyntaxError}\n",
        )
        .unwrap();
        assert!(m.flags.is_empty());
        assert_eq!(m.features, ["a", "b"]);
        assert_eq!(m.negative.unwrap().error_type, "SyntaxError");
        let m = parse("description: a long\n  plain scalar\n").unwrap();
        assert_eq!(m.description, "a long plain scalar");
        let m = parse("description: |\n  one\n\n  two\n").unwrap();
        assert_eq!(m.description, "one\n\ntwo");
        // language/statements/function/13.2-30-s.js: indented as a whole.
        let m = parse("\n description: >\n    Functions created\n    using bind\n flags: [x]\n")
            .unwrap();
        assert_eq!(m.description, "Functions created using bind");
        assert_eq!(m.flags, ["x"]);
        // CR line terminators (Function/prototype/toString/line-terminator-normalisation-CR.js).
        let m = parse("\resid: x\rdescription: y\rinfo: |\r  z\rincludes: [a.js]\r").unwrap();
        assert_eq!(m.includes, ["a.js"]);
    }

    #[test]
    fn errors() {
        assert!(parse_source("no frontmatter here").is_err());
        assert!(parse("negative:\n  phase: parse\n").is_err());
        assert!(parse("negative:\n  phase: later\n  type: X\n").is_err());
        assert!(parse("features: [a,\n  b]\n").is_err());
        assert!(parse("just text\n").is_err());
    }
}
