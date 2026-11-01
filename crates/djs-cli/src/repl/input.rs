//! What to do with an input: an expression to print, a Script, a top-level `await`
//! rewrite, or more lines to read (D1641).

use djs_core::compiler::CompileError;
use djs_core::lexer::{Kw, Lexer, Token, TokenKind};

/// Whether a Script's `Status::Done` value is its completion value (issue 0058). While it
/// is not, expression inputs are wrapped to get their value. This is the one switch for
/// when 0058 lands.
pub const SCRIPT_COMPLETION_VALUES: bool = false;

/// The source of an expression input, compiled as an async body: its value comes back
/// in a one-element array, so a promise value is not adopted by the body's promise.
pub fn expression_body(src: &str) -> Option<String> {
    let src = src.trim_end().trim_end_matches(';').trim_end();
    if src.is_empty() {
        return None;
    }
    let toks = tokens(src).ok()?;
    // These parse as expressions in parentheses, but as input they are declarations.
    match (toks.first(), toks.get(1)) {
        (Some(t), _) if matches!(t.kind, TokenKind::Function | TokenKind::Class) => return None,
        (Some(a), Some(b)) if a.is_contextual(Kw::Async) && b.kind == TokenKind::Function => {
            return None;
        }
        _ => {}
    }
    // On the first line, so the lines of the input keep their numbers.
    Some(format!("return [({src}\n)];"))
}

/// The input is unfinished: the error is at its end (an unclosed bracket, string
/// template or comment), so the REPL reads another line.
pub fn is_incomplete(src: &str, e: &CompileError) -> bool {
    let end = src.trim_end().len();
    let pos = e.pos as usize;
    pos >= end
        || e.message.contains("end of input")
        || e.message.contains("end of comment")
        || (e.message.contains("end of string") && src.as_bytes().get(pos) == Some(&b'`'))
}

/// A top-level-`await` input rewritten as an async body whose declarations are globals.
#[derive(Debug, PartialEq, Eq)]
pub struct AwaitRewrite {
    /// A Script declaring the input's top-level `let`/`const`/`class` (as `let`) and
    /// `var` names, run before the body.
    pub declare: String,
    /// The body: declarations become assignments to those globals.
    pub body: String,
}

/// Rewrites an input with top-level `await` (D1641), or `None` if it has no `await` or
/// does not tokenize. Top level here means outside any bracket:
/// - `let`/`const`/`var` become `0,` padded to the keyword's width, so `let {a} = o`
///   is the destructuring assignment `0, {a} = o` to the predeclared names;
/// - `class C {}` becomes `C = class C {};`;
/// - `function f() {}` stays (it is hoisted in the body) and is followed by
///   `;globalThis.f = f;`.
pub fn rewrite_await(src: &str) -> Option<AwaitRewrite> {
    let toks = tokens(src).ok()?;
    let text = |t: &Tok| &src[t.start..t.end];
    if !toks
        .iter()
        .any(|t| t.kind == TokenKind::Ident && text(t) == "await")
    {
        return None;
    }
    let mut lets: Vec<String> = Vec::new();
    let mut vars: Vec<String> = Vec::new();
    // (position, text to insert, bytes to replace)
    let mut edits: Vec<(usize, String, usize)> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t.depth != 0 {
            i += 1;
            continue;
        }
        let next = toks.get(i + 1).map(|n| n.kind);
        let is_binding_start = matches!(
            next,
            Some(TokenKind::Ident | TokenKind::LBracket | TokenKind::LBrace)
        );
        let lexical = t.is_contextual(Kw::Let) || t.kind == TokenKind::Const;
        if (lexical || t.kind == TokenKind::Var) && is_binding_start {
            let names = if lexical { &mut lets } else { &mut vars };
            i = declarators(src, &toks, i + 1, names);
            let pad = " ".repeat(t.end - t.start - 2);
            edits.push((t.start, format!("0,{pad}"), t.end - t.start));
            continue;
        }
        let stmt = statement_start(&toks, i);
        let (is_fn, fn_at) = if t.kind == TokenKind::Function {
            (true, i)
        } else if t.is_contextual(Kw::Async)
            && next == Some(TokenKind::Function)
            && !toks[i + 1].newline
        {
            (true, i + 1)
        } else {
            (false, i)
        };
        if stmt && (is_fn || t.kind == TokenKind::Class) {
            let mut j = fn_at + 1;
            if toks.get(j).is_some_and(|t| t.kind == TokenKind::Star) {
                j += 1;
            }
            let name = toks
                .get(j)
                .filter(|n| n.kind == TokenKind::Ident)
                .map(|n| text(n).to_string());
            let Some(name) = name else {
                i += 1;
                continue;
            };
            let end = body_end(&toks, j, is_fn)?;
            if is_fn {
                edits.push((toks[end].end, format!(";globalThis.{name} = {name};"), 0));
                vars.push(name);
            } else {
                edits.push((t.start, format!("{name} = "), 0));
                edits.push((toks[end].end, ";".into(), 0));
                lets.push(name);
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    let mut body = String::with_capacity(src.len() + 64);
    let mut at = 0;
    for (pos, insert, replace) in edits {
        body.push_str(&src[at..pos]);
        body.push_str(&insert);
        at = pos + replace;
    }
    body.push_str(&src[at..]);
    let mut declare = String::new();
    dedup(&mut lets);
    dedup(&mut vars);
    if !lets.is_empty() {
        declare += &format!("let {};", lets.join(", "));
    }
    if !vars.is_empty() {
        declare += &format!("var {};", vars.join(", "));
    }
    Some(AwaitRewrite { declare, body })
}

fn dedup(v: &mut Vec<String>) {
    let mut seen = Vec::new();
    v.retain(|n| {
        let new = !seen.contains(n);
        seen.push(n.clone());
        new
    });
}

/// Token `i` starts a statement: it is first, follows `;` or `}`, or follows a line
/// break after a token that ends an expression (ASI).
fn statement_start(toks: &[Tok], i: usize) -> bool {
    let Some(prev) = i.checked_sub(1).map(|p| toks[p]) else {
        return true;
    };
    matches!(prev.kind, TokenKind::Semicolon | TokenKind::RBrace)
        || (toks[i].newline && ends_expression(prev.kind))
}

/// The index of the `}` that ends the function or class whose name is token `name`.
fn body_end(toks: &[Tok], name: usize, is_fn: bool) -> Option<usize> {
    let depth = toks[name].depth;
    let mut j = name + 1;
    if is_fn {
        // Skip the parameters: the body is the first `{` after them.
        while toks.get(j)?.kind != TokenKind::LParen {
            j += 1;
        }
        j += 1;
        while !(toks.get(j)?.kind == TokenKind::RParen && toks[j].depth == depth) {
            j += 1;
        }
    }
    while !(toks.get(j)?.kind == TokenKind::LBrace && toks[j].depth == depth) {
        j += 1;
    }
    j += 1;
    while !(toks.get(j)?.kind == TokenKind::RBrace && toks[j].depth == depth) {
        j += 1;
    }
    Some(j)
}

/// Reads the declarators of a `let`/`const`/`var` starting at token `i`, adding the bound
/// names. Returns the index after the declaration.
fn declarators(src: &str, toks: &[Tok], mut i: usize, names: &mut Vec<String>) -> usize {
    let depth = toks[i].depth;
    loop {
        i = pattern(src, toks, i, names);
        if toks.get(i).is_some_and(|t| t.kind == TokenKind::Assign) {
            i = skip_expression(toks, i + 1, depth);
        }
        match toks.get(i) {
            Some(t) if t.kind == TokenKind::Comma && t.depth == depth => i += 1,
            _ => return i,
        }
    }
}

/// Skips an assignment expression: to the next `,` or `;` at `depth`, a closing bracket
/// below it, or a line break before a declaration keyword (ASI).
fn skip_expression(toks: &[Tok], mut i: usize, depth: usize) -> usize {
    while let Some(t) = toks.get(i) {
        if t.depth < depth
            || (t.depth == depth && matches!(t.kind, TokenKind::Comma | TokenKind::Semicolon))
        {
            break;
        }
        if t.depth == depth
            && t.newline
            && i > 0
            && ends_expression(toks[i - 1].kind)
            && (matches!(
                t.kind,
                TokenKind::Var | TokenKind::Const | TokenKind::Function | TokenKind::Class
            ) || t.is_contextual(Kw::Let))
        {
            break;
        }
        i += 1;
    }
    i
}

/// A binding pattern at token `i`: an identifier, `[...]` or `{...}`. Adds the bound
/// names and returns the index after it.
fn pattern(src: &str, toks: &[Tok], i: usize, names: &mut Vec<String>) -> usize {
    let Some(t) = toks.get(i) else { return i };
    match t.kind {
        TokenKind::Ident => {
            names.push(src[t.start..t.end].to_string());
            i + 1
        }
        TokenKind::LBracket => {
            let inner = t.depth + 1;
            let mut j = i + 1;
            loop {
                let Some(t) = toks.get(j) else { return j };
                match t.kind {
                    TokenKind::RBracket if t.depth + 1 == inner => return j + 1,
                    TokenKind::Comma => j += 1,
                    TokenKind::Ellipsis => j = pattern(src, toks, j + 1, names),
                    _ => {
                        let k = pattern(src, toks, j, names);
                        j = if k == j { j + 1 } else { k };
                        if toks.get(j).is_some_and(|t| t.kind == TokenKind::Assign) {
                            j = skip_expression(toks, j + 1, inner);
                        }
                    }
                }
            }
        }
        TokenKind::LBrace => {
            let inner = t.depth + 1;
            let mut j = i + 1;
            loop {
                let Some(t) = toks.get(j) else { return j };
                match t.kind {
                    TokenKind::RBrace if t.depth + 1 == inner => return j + 1,
                    TokenKind::Comma => j += 1,
                    TokenKind::Ellipsis => j = pattern(src, toks, j + 1, names),
                    _ => {
                        // A key: a name, a literal or `[computed]`.
                        let key = j;
                        if t.kind == TokenKind::LBracket {
                            j += 1;
                            while toks.get(j).is_some_and(|k| {
                                !(k.kind == TokenKind::RBracket && k.depth == inner)
                            }) {
                                j += 1;
                            }
                        }
                        j += 1;
                        if toks.get(j).is_some_and(|t| t.kind == TokenKind::Colon) {
                            j = pattern(src, toks, j + 1, names);
                        } else if toks[key].kind == TokenKind::Ident {
                            names.push(src[toks[key].start..toks[key].end].to_string());
                        }
                        if toks.get(j).is_some_and(|t| t.kind == TokenKind::Assign) {
                            j = skip_expression(toks, j + 1, inner);
                        }
                    }
                }
            }
        }
        _ => i,
    }
}

/// A token with its bracket depth (before it: an opening bracket has the outer depth,
/// its closing bracket too).
#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: TokenKind,
    kw: Kw,
    start: usize,
    end: usize,
    depth: usize,
    newline: bool,
    escaped: bool,
}

impl Tok {
    fn is_contextual(&self, kw: Kw) -> bool {
        self.kind == TokenKind::Ident && self.kw == kw && !self.escaped
    }
}

/// A token after which `/` is division, not a regex.
fn ends_expression(k: TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        Ident
            | PrivateName
            | Number
            | BigInt
            | String
            | NoSubstTemplate
            | TemplateTail
            | Regex
            | RParen
            | RBracket
            | RBrace
            | This
            | Super
            | True
            | False
            | Null
            | PlusPlus
            | MinusMinus
    )
}

/// The tokens of `src` with their depths. The lexer is context-free; this decides
/// regexes by the previous token and continues templates at the `}` that closes a
/// substitution, which is enough to find the top level.
fn tokens(src: &str) -> Result<Vec<Tok>, ()> {
    let mut lx = Lexer::new(src.as_bytes()).map_err(|_| ())?;
    let mut out: Vec<Tok> = Vec::new();
    // Open brackets; `true` for a template substitution.
    let mut stack: Vec<bool> = Vec::new();
    loop {
        let mut t: Token = lx.next_token().map_err(|_| ())?;
        let prev = out.last().map(|p| p.kind);
        match t.kind {
            TokenKind::Eof => return Ok(out),
            TokenKind::Slash | TokenKind::SlashAssign if !prev.is_some_and(ends_expression) => {
                t = lx.rescan_as_regex(t).map_err(|_| ())?;
            }
            TokenKind::RBrace if stack.last() == Some(&true) => {
                t = lx.rescan_template_continuation(t).map_err(|_| ())?;
            }
            _ => {}
        }
        let depth_before = stack.len();
        let depth = match t.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                stack.push(false);
                depth_before
            }
            TokenKind::TemplateHead => {
                stack.push(true);
                depth_before
            }
            TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::TemplateTail => {
                stack.pop();
                stack.len()
            }
            TokenKind::TemplateMiddle => depth_before - 1,
            _ => depth_before,
        };
        out.push(Tok {
            kind: t.kind,
            kw: t.kw,
            start: t.start as usize,
            end: t.end as usize,
            depth,
            newline: t.newline_before(),
            escaped: t.has_escape(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rw(src: &str) -> (String, String) {
        let r = rewrite_await(src).expect("rewrites");
        (r.declare, r.body)
    }

    #[test]
    fn no_await_no_rewrite() {
        assert_eq!(rewrite_await("let x = 1"), None);
        assert_eq!(rewrite_await("let x = 'await'"), None);
    }

    #[test]
    fn declarations() {
        assert_eq!(
            rw("let x = await f(), y;\nconst {a, b: [c, d = 1], ...e} = await g(x, y);"),
            (
                "let x, y, a, c, d, e;".into(),
                "0,  x = await f(), y;\n0,    {a, b: [c, d = 1], ...e} = await g(x, y);".into()
            )
        );
        assert_eq!(
            rw("var v = await 1\nlet w = 2"),
            ("let w;var v;".into(), "0,  v = await 1\n0,  w = 2".into())
        );
    }

    #[test]
    fn patterns() {
        assert_eq!(
            rw("let {[k]: v, 'a b': u = {x: 1}, w} = await o, [, z] = [];").0,
            "let v, u, w, z;"
        );
        assert_eq!(
            rw("let {p, q: [w]} = await f({p: 1, q: [2]})").0,
            "let p, w;"
        );
    }

    #[test]
    fn nested_scopes_stay() {
        assert_eq!(
            rw("for (let i of await xs()) { let j = i; }\n{ let k; }"),
            (
                "".into(),
                "for (let i of await xs()) { let j = i; }\n{ let k; }".into()
            )
        );
    }

    #[test]
    fn functions_and_classes() {
        assert_eq!(
            rw("async function f() { return {}; }\nclass C extends B { m() {} }\nawait f()"),
            (
                "let C;var f;".into(),
                "async function f() { return {}; };globalThis.f = f;\nC = class C extends B { m() {} };\nawait f()".into()
            )
        );
        // A function expression is not a declaration.
        assert_eq!(
            rw("x = function g() {}; await x"),
            ("".into(), "x = function g() {}; await x".into())
        );
    }

    #[test]
    fn regex_and_templates() {
        assert_eq!(
            rw("let r = /[)}]/g, t = `${ {a: 1}.a }`; await r"),
            (
                "let r, t;".into(),
                "0,  r = /[)}]/g, t = `${ {a: 1}.a }`; await r".into()
            )
        );
    }

    #[test]
    fn expressions() {
        assert_eq!(
            expression_body("1 + 1;").as_deref(),
            Some("return [(1 + 1\n)];")
        );
        assert_eq!(expression_body("function f() {}"), None);
        assert_eq!(expression_body("async function f() {}"), None);
        assert_eq!(expression_body("class A {}"), None);
    }
}
