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

/// What follows the closing bracket of a `{...}` or `[...]` (docs/compiler.md 3.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Follow {
    /// `=`: a destructuring assignment pattern.
    Assign,
    /// `,`, `]` or `}`: inside a pattern, a nested pattern.
    ElemEnd,
    /// `of` or `in`: in a for-in/of head, a pattern.
    InOf,
    Other,
}

fn classify(t: &Token) -> Follow {
    match t.kind {
        TokenKind::Assign => Follow::Assign,
        TokenKind::Comma | TokenKind::RBracket | TokenKind::RBrace => Follow::ElemEnd,
        TokenKind::In => Follow::InOf,
        _ if t.is_contextual(Kw::Of) => Follow::InOf,
        _ => Follow::Other,
    }
}

/// The memoized bracket scan (docs/compiler.md 3.3): one scan of a literal records what
/// follows every `{`/`[` group closed inside it, so nested literals are not rescanned.
#[derive(Default)]
pub(super) struct PatternScan {
    start: u32,
    end: u32,
    /// `(start offset of the bracket, what follows its closer)`, sorted.
    entries: Vec<(u32, Follow)>,
}

impl PatternScan {
    /// With `open` the current `{` or `[` token and `lx` positioned after it.
    pub fn follow(&mut self, lx: Lexer<'_>, open: Token) -> Follow {
        if self.start <= open.start
            && open.start < self.end
            && let Ok(i) = self.entries.binary_search_by_key(&open.start, |e| e.0)
        {
            return self.entries[i].1;
        }
        self.scan(lx, open)
    }

    fn scan(&mut self, lx: Lexer<'_>, open: Token) -> Follow {
        self.start = 0;
        self.end = 0;
        self.entries.clear();
        let kind = if open.kind == TokenKind::LBrace {
            b'{'
        } else {
            b'['
        };
        let mut s = Scanner {
            lx,
            prev: open,
            stack: alloc::vec![kind],
        };
        // Parallel to `s.stack`: the start of each open `{`/`[`.
        let mut opens: Vec<Option<u32>> = alloc::vec![Some(open.start)];
        let mut pending: Option<u32> = None;
        let mut end = open.end;
        while !s.stack.is_empty() {
            let before = s.stack.len();
            let Some(tok) = s.next() else {
                self.entries.clear();
                return Follow::Other;
            };
            if let Some(p) = pending.take() {
                self.entries.push((p, classify(&tok)));
            }
            if s.stack.len() > before {
                let bracket = matches!(tok.kind, TokenKind::LBrace | TokenKind::LBracket);
                opens.push(bracket.then_some(tok.start));
            } else if s.stack.len() < before {
                pending = opens.pop().flatten();
            }
            end = tok.end;
        }
        let follow = match s.lx.next_token() {
            Ok(t) => classify(&t),
            Err(_) => Follow::Other,
        };
        self.entries.push((open.start, follow));
        self.entries.sort_unstable_by_key(|e| e.0);
        self.start = open.start;
        self.end = end;
        follow
    }
}

/// With `open` the current `{` or `[` and `lx` after it: the lexer after the matching
/// closer and the closer's end offset, or `None` if the group does not scan.
pub(super) fn skip_group<'a>(lx: Lexer<'a>, open: Token) -> Option<(Lexer<'a>, u32)> {
    let kind = if open.kind == TokenKind::LBrace {
        b'{'
    } else {
        b'['
    };
    let mut s = Scanner {
        lx,
        prev: open,
        stack: alloc::vec![kind],
    };
    let mut end = open.end;
    while !s.stack.is_empty() {
        end = s.next()?.end;
    }
    Some((s.lx, end))
}

/// With `open` a `{` and `lx` after it: does the object pattern have a rest property
/// (`...` directly inside)?
pub(super) fn object_has_rest(lx: Lexer<'_>, open: Token) -> bool {
    let mut s = Scanner {
        lx,
        prev: open,
        stack: alloc::vec![b'{'],
    };
    while !s.stack.is_empty() {
        let Some(t) = s.next() else { return false };
        if t.kind == TokenKind::Ellipsis && s.stack.len() == 1 {
            return true;
        }
    }
    false
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
    fn pattern_follow() {
        let src = "[a, {b: [c]}, [d] = e] = f; ({x} = y); [g] + 1";
        let mut lx = Lexer::new(src.as_bytes()).unwrap();
        let mut memo = PatternScan::default();
        let mut found = Vec::new();
        loop {
            let t = lx.next_token().unwrap();
            match t.kind {
                TokenKind::Eof => break,
                TokenKind::LBracket | TokenKind::LBrace => {
                    found.push((t.start, memo.follow(lx, t)));
                }
                _ => {}
            }
        }
        use Follow::*;
        assert_eq!(
            found,
            [
                (0, Assign),
                (4, ElemEnd),
                (8, ElemEnd),
                (14, Assign),
                (29, Assign),
                (39, Other),
            ]
        );
        // The first scan covered the nested groups: they come from the memo.
        let mut lx = Lexer::new(b"[[[1], 2]] = x").unwrap();
        let open = lx.next_token().unwrap();
        assert_eq!(memo.follow(lx, open), Assign);
        assert_eq!(memo.entries.len(), 3);
        let inner = lx.next_token().unwrap();
        assert_eq!(memo.follow(lx, inner), ElemEnd);
        assert_eq!(memo.entries.len(), 3);
        let mut lx = Lexer::new(b"for ([a] of b)").unwrap();
        lx.next_token().unwrap();
        lx.next_token().unwrap();
        let open = lx.next_token().unwrap();
        assert_eq!(memo.follow(lx, open), InOf);
        let mut lx = Lexer::new(b"[a, `${[b]}`] = c").unwrap();
        let open = lx.next_token().unwrap();
        assert_eq!(memo.follow(lx, open), Assign);
        assert!(memo.entries.iter().any(|&(p, f)| p == 7 && f == Other));
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
