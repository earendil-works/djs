//! Token-level lookahead scans (docs/compiler.md 3.1, 3.4). Scans only move a copy of
//! the lexer; they never emit.

use alloc::vec::Vec;

use crate::lexer::{Kw, Lexer, Token, TokenKind};

/// The previous-token heuristic for scans: may a `/` after `prev` start a regex
/// (QuickJS's `is_regexp_allowed`)? The parser itself always knows.
fn regex_allowed(prev: &Token) -> bool {
    use TokenKind::*;
    match prev.kind {
        Ident => matches!(prev.kw, Kw::Yield | Kw::Await | Kw::Of) && !prev.has_escape(),
        Number | BigInt | String | NoSubstTemplate | TemplateTail | Regex | PrivateName
        | RParen | RBracket | RBrace | This | Super | True | False | Null | PlusPlus
        | MinusMinus => false,
        _ => true,
    }
}

/// A lexer copy that handles regexes and template continuations like the parser
/// would, for balanced-bracket scans.
struct Scanner<'a> {
    lx: Lexer<'a>,
    prev: Token,
    /// Open brackets: `(`, `[`, `{`, or `` ` `` for a template substitution.
    stack: Vec<u8>,
}

impl<'a> Scanner<'a> {
    fn next(&mut self) -> Option<Token> {
        let mut tok = self.lx.next_token().ok()?;
        match tok.kind {
            TokenKind::Slash | TokenKind::SlashAssign if regex_allowed(&self.prev) => {
                tok = self.lx.rescan_as_regex(tok).ok()?;
            }
            TokenKind::RBrace if self.stack.last() == Some(&b'`') => {
                tok = self.lx.rescan_template_continuation(tok).ok()?;
                if tok.kind == TokenKind::TemplateTail {
                    self.stack.pop();
                }
            }
            TokenKind::TemplateHead => self.stack.push(b'`'),
            TokenKind::LParen => self.stack.push(b'('),
            TokenKind::LBracket => self.stack.push(b'['),
            TokenKind::LBrace => self.stack.push(b'{'),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let open = match tok.kind {
                    TokenKind::RParen => b'(',
                    TokenKind::RBracket => b'[',
                    _ => b'{',
                };
                if self.stack.pop() != Some(open) {
                    return None;
                }
            }
            TokenKind::Eof => return None,
            _ => {}
        }
        self.prev = tok;
        Some(tok)
    }
}

/// With `open` the current `(` token and `lx` positioned after it: is this an arrow
/// function head, `( params ) =>` with no newline before the arrow? Bails out at the
/// first token that cannot be part of a parameter list, so parenthesized expressions
/// cost a token or two (docs/compiler.md 3.1).
pub(super) fn is_arrow_head(lx: Lexer<'_>, open: Token) -> bool {
    #[derive(PartialEq)]
    enum State {
        Param,
        AfterParam,
        Default,
    }
    let mut s = Scanner {
        lx,
        prev: open,
        stack: alloc::vec![b'('],
    };
    let mut state = State::Param;
    loop {
        let depth_before = s.stack.len();
        let Some(tok) = s.next() else { return false };
        let depth = s.stack.len();
        if depth_before == 1 {
            // A token directly inside the parentheses (or the closing one).
            match (&state, tok.kind) {
                (_, TokenKind::RParen) => break,
                (State::Param, TokenKind::Ident) => state = State::AfterParam,
                (State::Param, TokenKind::Ellipsis) => {}
                // A destructuring pattern: skipped as balanced, then AfterParam.
                (State::Param, TokenKind::LBrace | TokenKind::LBracket) => {}
                (State::Param, _) => return false,
                (State::AfterParam, TokenKind::Comma) => state = State::Param,
                (State::AfterParam, TokenKind::Assign) => state = State::Default,
                (State::AfterParam, _) => return false,
                (State::Default, TokenKind::Comma) => state = State::Param,
                (State::Default, _) => {}
            }
        } else if depth == 1 && state == State::Param {
            // A pattern parameter closed.
            state = State::AfterParam;
        }
    }
    matches!(s.lx.next_token(), Ok(t) if t.kind == TokenKind::Arrow && !t.newline_before())
}

/// Inside a literal opened by `open` (`{` or `[`), with `lx` somewhere directly in it:
/// is the literal's closing bracket followed by `=`? That makes it a destructuring
/// pattern (docs/compiler.md 3.3).
pub(super) fn closes_before_assign(lx: Lexer<'_>, open: u8) -> bool {
    let mut s = Scanner {
        lx,
        prev: Token {
            kind: TokenKind::Comma,
            kw: Kw::None,
            flags: 0,
            start: 0,
            end: 0,
        },
        stack: alloc::vec![open],
    };
    while !s.stack.is_empty() {
        if s.next().is_none() {
            return false;
        }
    }
    matches!(s.lx.next_token(), Ok(t) if t.kind == TokenKind::Assign)
}

/// With `open` the `{` of a try block and `lx` after it: does the statement have a
/// `finally` clause? (`break`/`return` inside the try block must know before it is
/// parsed, D1080.)
pub(super) fn try_has_finally(lx: Lexer<'_>, open: Token) -> bool {
    let mut s = Scanner {
        lx,
        prev: open,
        stack: alloc::vec![b'{'],
    };
    let skip_group = |s: &mut Scanner<'_>| -> Option<()> {
        while !s.stack.is_empty() {
            s.next()?;
        }
        Some(())
    };
    if skip_group(&mut s).is_none() {
        return false;
    }
    let Some(mut t) = s.next() else { return false };
    if t.kind == TokenKind::Catch {
        let Some(n) = s.next() else { return false };
        if n.kind == TokenKind::LParen && skip_group(&mut s).is_none() {
            return false;
        }
        if n.kind == TokenKind::LParen {
            match s.next() {
                Some(b) if b.kind == TokenKind::LBrace => {}
                _ => return false,
            }
        } else if n.kind != TokenKind::LBrace {
            return false;
        }
        if skip_group(&mut s).is_none() {
            return false;
        }
        match s.lx.next_token() {
            Ok(n) => t = n,
            Err(_) => return false,
        }
    }
    t.kind == TokenKind::Finally
}

/// With `open` the `(` after `for` and `lx` positioned after it: is this a C-style
/// `for` head, with a `;` directly inside the parentheses (docs/compiler.md 3.8,
/// QuickJS does the same)? Otherwise it is for-in/of, also for a head that does not
/// scan (unbalanced, end of input): its parser reports the error, as in QuickJS.
pub(super) fn for_head_is_c_style(lx: Lexer<'_>, open: Token) -> bool {
    let mut s = Scanner {
        lx,
        prev: open,
        stack: alloc::vec![b'('],
    };
    loop {
        let Some(t) = s.next() else { return false };
        if s.stack.is_empty() {
            return false;
        }
        if t.kind == TokenKind::Semicolon && s.stack.len() == 1 {
            return true;
        }
    }
}

/// With `first` the first token of an expression for-in/of head and `lx` after it:
/// does the left side end at `of` (rather than `in`)? Looks for the first `of` or `in`
/// directly inside the head's parentheses; an `of` after `.` or `?.` is a property
/// name.
pub(super) fn for_lhs_ends_with_of(lx: Lexer<'_>, first: Token) -> bool {
    let mut s = Scanner {
        lx,
        prev: first,
        stack: alloc::vec![b'('],
    };
    match first.kind {
        TokenKind::LParen => s.stack.push(b'('),
        TokenKind::LBracket => s.stack.push(b'['),
        TokenKind::LBrace => s.stack.push(b'{'),
        TokenKind::TemplateHead => s.stack.push(b'`'),
        _ => {}
    }
    loop {
        let prev = s.prev.kind;
        let Some(t) = s.next() else { return false };
        if s.stack.len() != 1 {
            if s.stack.is_empty() {
                return false;
            }
            continue;
        }
        if t.kind == TokenKind::In {
            return false;
        }
        if t.is_contextual(Kw::Of) && !matches!(prev, TokenKind::Dot | TokenKind::QuestionDot) {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrow(src: &str) -> bool {
        let mut lx = Lexer::new(src.as_bytes()).unwrap();
        let open = lx.next_token().unwrap();
        assert_eq!(open.kind, TokenKind::LParen);
        is_arrow_head(lx, open)
    }

    #[test]
    fn finally_lookahead() {
        let has = |src: &str| {
            let mut lx = Lexer::new(src.as_bytes()).unwrap();
            assert_eq!(lx.next_token().unwrap().kind, TokenKind::Try);
            let open = lx.next_token().unwrap();
            try_has_finally(lx, open)
        };
        assert!(has("try {} finally {}"));
        assert!(has("try { a({b: [1]}) } catch (e) { `${e}` } finally {}"));
        assert!(has("try {} catch { /}/ } finally {}"));
        assert!(!has("try {} catch (e) {}"));
        assert!(!has("try { } catch (e) { } x; finally"));
        assert!(!has("try {"));
    }

    #[test]
    fn arrow_heads() {
        for src in [
            "() => 1",
            "(a) => a",
            "(a, b) => a",
            "(a,) => a",
            "(...r) => r",
            "(a = 1, b = (2, 3)) => a",
            "({a, b: [c]}, [d] = []) => a",
            "(a = /)/) => a",
            "(a = `${1}`) => a",
            "(a = { x: 1 }) => a",
        ] {
            assert!(arrow(src), "{src}");
        }
        for src in [
            "(a)",
            "(a + b) * c",
            "(function () {})()",
            "(a, b)",
            "(a.b) => 1",
            "(1) => 1",
            "(a)\n=> a",
            "(a",
            "((a)) => 1",
        ] {
            assert!(!arrow(src), "{src}");
        }
    }

    #[test]
    fn for_heads() {
        let scan = |src: &str| {
            let mut lx = Lexer::new(src.as_bytes()).unwrap();
            assert_eq!(lx.next_token().unwrap().kind, TokenKind::For);
            let open = lx.next_token().unwrap();
            let c_style = for_head_is_c_style(lx, open);
            let first = lx.next_token().unwrap();
            (c_style, for_lhs_ends_with_of(lx, first))
        };
        assert!(scan("for (;;) {}").0);
        assert!(scan("for (let i = 0; i < 3; i++) {}").0);
        assert_eq!(scan("for (x of y) {}"), (false, true));
        assert_eq!(scan("for (x in y) {}"), (false, false));
        assert_eq!(scan("for (x of () => { a; b; }) {}"), (false, true));
        assert_eq!(scan("for (x.of in y) {}"), (false, false));
        assert_eq!(scan("for (of of y) {}"), (false, true));
        assert_eq!(scan("for (a[x in y] of z) {}"), (false, true));
        assert_eq!(scan("for (x of /;/) {}"), (false, true));
        assert_eq!(scan("for (x of `${a}`) {}"), (false, true));
        assert!(!scan("for (x of y").0);
    }
}
