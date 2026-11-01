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
}
