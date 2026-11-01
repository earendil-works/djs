//! Statements.

use alloc::vec::Vec;

use super::PResult;
use super::func::{FuncKind, HandlerLabels, ScopeKind, VarKind};
use super::parser::{BlockEnv, BlockKind, Parser, syntax};
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

impl Parser<'_, '_> {
    /// StatementListItem: declarations are allowed here, not in single-statement
    /// positions.
    pub(super) fn parse_statement_list_item(&mut self) -> PResult<()> {
        match self.tok.kind {
            TokenKind::Const => self.parse_lexical_declaration(VarKind::Const),
            TokenKind::Ident if self.tok.is_contextual(Kw::Let) => {
                self.parse_lexical_declaration(VarKind::Let)
            }
            TokenKind::Function => self.parse_function_declaration(),
            TokenKind::Class => self.unsupported(self.tok.start, "classes are"),
            _ => self.parse_statement(),
        }
    }

    pub(super) fn parse_statement(&mut self) -> PResult<()> {
        self.enter()?;
        let r = self.parse_statement_inner();
        self.leave();
        r
    }

    fn parse_statement_inner(&mut self) -> PResult<()> {
        let tok = self.tok;
        match tok.kind {
            TokenKind::LBrace => self.parse_block(),
            TokenKind::Semicolon => self.next(),
            TokenKind::Var => {
                self.next()?;
                self.parse_var_declarations(false)?;
                self.expect_semicolon()
            }
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(Vec::new()),
            TokenKind::Do => self.parse_do_while(Vec::new()),
            TokenKind::For => self.parse_for(Vec::new()),
            TokenKind::Break | TokenKind::Continue => self.parse_break_continue(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Throw => self.parse_throw(),
            TokenKind::Switch => self.parse_switch(Vec::new()),
            TokenKind::Try => self.parse_try(),
            TokenKind::Debugger => {
                self.next()?;
                self.code().debugger();
                self.expect_semicolon()
            }
            TokenKind::With => syntax(tok.start, "invalid keyword: with"),
            TokenKind::Import | TokenKind::Export => {
                let text = self.tok_text();
                syntax(tok.start, alloc::format!("unsupported keyword: {text}"))
            }
            TokenKind::Const => syntax(
                tok.start,
                "lexical declarations can't appear in single-statement context",
            ),
            TokenKind::Ident if tok.is_contextual(Kw::Let) => syntax(
                tok.start,
                "lexical declarations can't appear in single-statement context",
            ),
            TokenKind::Function => syntax(
                tok.start,
                "function declarations can't appear in single-statement context",
            ),
            TokenKind::Class => syntax(
                tok.start,
                "class declarations can't appear in single-statement context",
            ),
            TokenKind::Ident if self.peek().kind == TokenKind::Colon => self.parse_labeled(),
            _ => {
                self.parse_expr(false)?;
                self.code().drop();
                self.expect_semicolon()
            }
        }
    }

    fn parse_block(&mut self) -> PResult<()> {
        self.next()?;
        self.push_scope(ScopeKind::Block)?;
        while !self.is(TokenKind::RBrace) {
            if self.is(TokenKind::Eof) {
                return syntax(self.tok.start, "expecting '}'");
            }
            self.parse_statement_list_item()?;
        }
        self.next()?;
        self.pop_scope();
        Ok(())
    }

    // ---- Declarations ---------------------------------------------------------------

    fn parse_lexical_declaration(&mut self, kind: VarKind) -> PResult<()> {
        self.next()?;
        self.parse_lexical_bindings(kind, false)?;
        self.expect_semicolon()
    }

    /// `let`/`const` bindings after the keyword. In a `for` head, stops before `in`/`of`
    /// without requiring a `const` initializer (the caller rejects for-in/of in P0).
    pub(super) fn parse_lexical_bindings(&mut self, kind: VarKind, in_for: bool) -> PResult<()> {
        loop {
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                return self.unsupported(self.tok.start, "destructuring is");
            }
            let pos = self.tok.start;
            let name = self.binding_name()?;
            self.next()?;
            let v = self.declare_lexical(name, kind, self.tok.start)?;
            if self.eat(TokenKind::Assign)? {
                self.name_hint = Some((name, self.tok.start));
                self.parse_assign(in_for)?;
            } else if in_for && (self.is(TokenKind::In) || self.tok.is_contextual(Kw::Of)) {
                return Ok(());
            } else if kind == VarKind::Const {
                return syntax(self.tok.start, "missing initializer for const variable");
            } else {
                self.code().undefined();
            }
            let r = self.init_ref(name, pos);
            self.code().name_init(r);
            self.f.vars[v as usize].init_end = self.prev_end;
            if !self.eat(TokenKind::Comma)? {
                return Ok(());
            }
        }
    }

    /// `var` bindings after the keyword.
    pub(super) fn parse_var_declarations(&mut self, in_for: bool) -> PResult<()> {
        loop {
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                return self.unsupported(self.tok.start, "destructuring is");
            }
            let pos = self.tok.start;
            let name = self.binding_name()?;
            self.next()?;
            self.declare_var(name, self.tok.start)?;
            if self.eat(TokenKind::Assign)? {
                self.name_hint = Some((name, self.tok.start));
                self.parse_assign(in_for)?;
                let r = self.name_ref(name, pos);
                self.mark(pos);
                self.code().name_put(r);
            }
            if !self.eat(TokenKind::Comma)? {
                return Ok(());
            }
        }
    }

    // ---- Control flow ---------------------------------------------------------------

    fn parse_paren_expr(&mut self) -> PResult<()> {
        self.expect(TokenKind::LParen, "(")?;
        self.parse_expr(false)?;
        self.expect(TokenKind::RParen, ")")
    }

    fn parse_if(&mut self) -> PResult<()> {
        self.next()?;
        self.parse_paren_expr()?;
        let l_else = self.new_label();
        self.code().jump_if_false(l_else);
        self.parse_statement()?;
        if self.eat(TokenKind::Else)? {
            let l_end = self.new_label();
            self.code().jump(l_end);
            self.place(l_else);
            // `else if` chains loop here instead of recursing (docs/compiler.md 2.4).
            if self.is(TokenKind::If) {
                self.parse_if_chain(l_end)?;
            } else {
                self.parse_statement()?;
            }
            self.place(l_end);
        } else {
            self.place(l_else);
        }
        Ok(())
    }

    /// The `if` of an `else if`, sharing the chain's end label.
    fn parse_if_chain(&mut self, l_end: crate::bytecode::Label) -> PResult<()> {
        loop {
            self.next()?;
            self.parse_paren_expr()?;
            let l_else = self.new_label();
            self.code().jump_if_false(l_else);
            self.parse_statement()?;
            if !self.eat(TokenKind::Else)? {
                self.place(l_else);
                return Ok(());
            }
            self.code().jump(l_end);
            self.place(l_else);
            if !self.is(TokenKind::If) {
                return self.parse_statement();
            }
        }
    }

    fn push_loop(&mut self, labels: Vec<Atom>) -> (crate::bytecode::Label, crate::bytecode::Label) {
        let l_break = self.new_label();
        let l_cont = self.new_label();
        self.blocks.push(BlockEnv {
            labels,
            kind: BlockKind::Loop,
            break_label: l_break,
            continue_label: Some(l_cont),
            depth: self.stmt_depth,
            used: false,
        });
        (l_break, l_cont)
    }

    fn parse_while(&mut self, labels: Vec<Atom>) -> PResult<()> {
        self.next()?;
        let (l_break, l_cont) = self.push_loop(labels);
        self.place(l_cont);
        self.parse_paren_expr()?;
        self.code().jump_if_false(l_break);
        self.parse_statement()?;
        self.code().jump(l_cont);
        self.place(l_break);
        self.blocks.pop();
        Ok(())
    }

    fn parse_do_while(&mut self, labels: Vec<Atom>) -> PResult<()> {
        self.next()?;
        let (l_break, l_cont) = self.push_loop(labels);
        let l_top = self.new_label();
        self.place(l_top);
        self.parse_statement()?;
        self.expect(TokenKind::While, "while")?;
        self.place(l_cont);
        self.parse_paren_expr()?;
        self.code().jump_if_true(l_top);
        self.place(l_break);
        self.blocks.pop();
        // `do ... while (x)` takes an optional `;` even without a newline.
        self.eat(TokenKind::Semicolon)?;
        Ok(())
    }

    /// `for (init; test; update) body`, laid out as
    /// `init; L_test: test; jump_if_false L_break; jump L_body; L_cont: update; jump
    /// L_test; L_body: body; jump L_cont; L_break:` so everything is emitted in source
    /// order. Without an update: `L_test: test; jump_if_false L_break; body; L_cont:
    /// jump L_test; L_break:`.
    fn parse_for(&mut self, labels: Vec<Atom>) -> PResult<()> {
        let for_pos = self.tok.start;
        self.next()?;
        if self.tok.is_contextual(Kw::Await) {
            return self.unsupported(self.tok.start, "for await loops are");
        }
        if !self.is(TokenKind::LParen) {
            return syntax(self.tok.start, "expecting '('");
        }
        if !super::scan::for_head_is_c_style(self.lx, self.tok) {
            self.next()?;
            return self.parse_for_in_of(labels, for_pos);
        }
        self.expect(TokenKind::LParen, "(")?;
        let outer = self.scope;
        let mut scoped = false;
        match self.tok.kind {
            TokenKind::Semicolon => {}
            TokenKind::Var => {
                self.next()?;
                self.parse_var_declarations(true)?;
            }
            TokenKind::Const => {
                self.next()?;
                self.push_scope(ScopeKind::For)?;
                scoped = true;
                self.parse_lexical_bindings(VarKind::Const, true)?;
            }
            TokenKind::Ident if self.tok.is_contextual(Kw::Let) => {
                self.next()?;
                self.push_scope(ScopeKind::For)?;
                scoped = true;
                self.parse_lexical_bindings(VarKind::Let, true)?;
            }
            _ => {
                self.parse_expr(true)?;
                self.code().drop();
            }
        }
        self.expect(TokenKind::Semicolon, ";")?;
        // Per-iteration copies of `let` bindings (CreatePerIterationEnvironment) only
        // matter for captured bindings, which need closures (P1-1: `rebox`).
        let (l_break, l_cont) = self.push_loop(labels);
        let l_test = self.new_label();
        let l_body = self.new_label();
        // Per-iteration copies of captured `let` bindings (docs/compiler.md 5.9):
        // after the init and before each update.
        let iter_scope = if scoped {
            Some(self.scope as u16)
        } else {
            None
        };
        if let Some(s) = iter_scope {
            self.code().iter_scope(s);
        }
        self.place(l_test);
        if !self.is(TokenKind::Semicolon) {
            self.parse_expr(false)?;
            self.code().jump_if_false(l_break);
        }
        self.expect(TokenKind::Semicolon, ";")?;
        let has_update = !self.is(TokenKind::RParen);
        if has_update {
            self.code().jump(l_body);
            self.place(l_cont);
            if let Some(s) = iter_scope {
                self.code().iter_scope(s);
            }
            self.parse_expr(false)?;
            self.code().drop();
            self.code().jump(l_test);
        }
        self.expect(TokenKind::RParen, ")")?;
        self.place(l_body);
        self.parse_statement()?;
        if !has_update {
            // No update: `continue` goes straight to the test.
            self.place(l_cont);
            if let Some(s) = iter_scope {
                self.code().iter_scope(s);
            }
            self.code().jump(l_test);
        } else {
            self.code().jump(l_cont);
        }
        self.place(l_break);
        self.blocks.pop();
        if scoped {
            self.pop_scope();
        }
        debug_assert_eq!(self.scope, outer);
        Ok(())
    }

    fn parse_break_continue(&mut self) -> PResult<()> {
        let is_break = self.is(TokenKind::Break);
        let pos = self.tok.start;
        self.next()?;
        let label = if self.is(TokenKind::Ident) && !self.tok.newline_before() {
            let a = self.ident_atom()?;
            self.next()?;
            Some(a)
        } else {
            None
        };
        let mut found = None;
        for (i, b) in self.blocks.iter().enumerate().rev() {
            if matches!(b.kind, BlockKind::Finally | BlockKind::Iter) {
                continue;
            }
            let target = match label {
                Some(l) if b.labels.contains(&l) => {
                    if is_break {
                        Some(b.break_label)
                    } else {
                        // `continue L` to a non-loop label is not found.
                        b.continue_label
                    }
                }
                Some(_) => continue,
                None if is_break && b.kind != BlockKind::Labeled => Some(b.break_label),
                None if !is_break && b.kind == BlockKind::Loop => b.continue_label,
                None => continue,
            };
            found = target.map(|t| (i, t, b.depth));
            break;
        }
        let Some((index, target, depth)) = found else {
            let msg = match (label, is_break) {
                (Some(_), _) => "break/continue label not found",
                (None, true) => "break must be inside loop or switch",
                (None, false) => "continue must be inside loop",
            };
            return syntax(pos, msg);
        };
        if is_break {
            self.blocks[index].used = true;
        }
        // Run the finally blocks and close the iterators being left, innermost first
        // (docs/compiler.md 7.4).
        let mut cur = self.emit_unwind(index + 1, false);
        while cur > depth {
            self.code().drop();
            cur -= 1;
        }
        self.code().jump(target);
        self.expect_semicolon()
    }

    /// Leaves the finally blocks and for-of loops of `self.blocks[from..]`, innermost
    /// first: each finally block runs (`gosub`) and each iterator is closed (`gosub`
    /// to its close subroutine). With `keep_value` the value on top of the stack (a
    /// `return` value) stays on top and is the finally blocks' value slot; otherwise
    /// `undefined` is. Returns the operand depth left, not counting a kept value.
    fn emit_unwind(&mut self, from: usize, keep_value: bool) -> u32 {
        let mut cur = self.stmt_depth;
        let exits: Vec<(BlockKind, crate::bytecode::Label, u32)> = self.blocks[from..]
            .iter()
            .rev()
            .filter(|b| matches!(b.kind, BlockKind::Finally | BlockKind::Iter))
            .map(|b| (b.kind, b.break_label, b.depth))
            .collect();
        for b in &mut self.blocks[from..] {
            if b.kind == BlockKind::Iter {
                b.used = true;
            }
        }
        for (kind, l_sub, base) in exits {
            // A finally block runs at its try's base; an iterator closes with the
            // iterator and `next` still on the stack.
            let keep = if kind == BlockKind::Iter {
                base + 2
            } else {
                base
            };
            while cur > keep {
                if keep_value {
                    self.code().nip();
                } else {
                    self.code().drop();
                }
                cur -= 1;
            }
            if !keep_value {
                self.code().undefined();
            }
            if kind == BlockKind::Iter {
                // `it next v` -> `v it next`, which the subroutine consumes.
                self.code().rot3r();
            }
            self.code().gosub(l_sub);
            if kind == BlockKind::Iter {
                // The subroutine leaves `v it undefined` (a `gosub` keeps the depth).
                self.code().drop();
                self.code().drop();
            }
            if !keep_value {
                self.code().drop();
            }
            cur = base;
        }
        cur
    }

    fn has_unwind(&self) -> bool {
        self.blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Finally | BlockKind::Iter))
    }

    fn parse_return(&mut self) -> PResult<()> {
        let pos = self.tok.start;
        if self.f.kind == FuncKind::Script {
            return syntax(pos, "return not in a function");
        }
        self.next()?;
        let bare = self.is(TokenKind::Semicolon)
            || self.is(TokenKind::RBrace)
            || self.is(TokenKind::Eof)
            || self.tok.newline_before();
        if bare && !self.has_unwind() {
            self.code().return_undef();
        } else {
            if bare {
                self.code().undefined();
            } else {
                self.parse_expr(false)?;
            }
            self.emit_unwind(0, true);
            self.code().return_();
        }
        self.expect_semicolon()
    }

    /// `try` (docs/compiler.md 7.4, D1084). With `base` the operand depth here:
    ///
    /// ```text
    /// L_try:   <try block>
    /// L_try_end:
    ///          [undefined; gosub L_fin; drop]; jump L_end
    /// L_catch: (handler for [L_try, L_try_end), stack: exc) <bind or drop>; <catch block>
    /// L_catch_end:
    ///          [undefined; gosub L_fin; drop]; jump L_end
    /// L_exc:   (handler for the try or catch region, if there is a finally)
    ///          gosub L_fin; throw
    /// L_fin:   (stack: value slot, return address) <finally block>; ret
    /// L_end:
    /// ```
    fn parse_try(&mut self) -> PResult<()> {
        self.next()?;
        if !self.is(TokenKind::LBrace) {
            return syntax(self.tok.start, "expecting '{'");
        }
        let base = self.stmt_depth;
        let has_finally = super::scan::try_has_finally(self.lx, self.tok);
        let l_fin = self.new_label();
        let l_end = self.new_label();
        if has_finally {
            self.blocks.push(BlockEnv {
                labels: Vec::new(),
                kind: BlockKind::Finally,
                break_label: l_fin,
                continue_label: None,
                depth: base,
                used: false,
            });
        }
        let normal_exit = |p: &mut Self| {
            if has_finally {
                p.code().undefined();
                p.code().gosub(l_fin);
                p.code().drop();
            }
            p.code().jump(l_end);
        };
        let l_try = self.new_label();
        let l_try_end = self.new_label();
        self.place(l_try);
        self.parse_block()?;
        self.place(l_try_end);
        normal_exit(self);
        let mut exc_region = (l_try, l_try_end);
        if self.is(TokenKind::Catch) {
            self.next()?;
            let l_catch = self.new_label();
            self.place(l_catch);
            self.f.handlers.push(HandlerLabels {
                start: l_try,
                end: l_try_end,
                target: l_catch,
                depth: base,
            });
            self.push_scope(ScopeKind::Catch)?;
            if self.eat(TokenKind::LParen)? {
                if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                    return self.unsupported(self.tok.start, "destructuring catch parameters are");
                }
                if self.tok.kind != TokenKind::Ident {
                    return syntax(self.tok.start, "identifier expected");
                }
                let pos = self.tok.start;
                let name = self.binding_name()?;
                self.next()?;
                let v = self.f.add_var(name, VarKind::CatchParam, self.scope);
                let _ = v;
                self.expect(TokenKind::RParen, ")")?;
                let r = self.init_ref(name, pos);
                self.code().name_init(r);
            } else {
                self.code().drop();
            }
            // The catch block's declarations share the clause's scope, so they
            // conflict with the parameter (`catch (e) { let e }`).
            self.expect(TokenKind::LBrace, "{")?;
            while !self.is(TokenKind::RBrace) {
                if self.is(TokenKind::Eof) {
                    return syntax(self.tok.start, "expecting '}'");
                }
                self.parse_statement_list_item()?;
            }
            self.next()?;
            self.pop_scope();
            let l_catch_end = self.new_label();
            self.place(l_catch_end);
            normal_exit(self);
            exc_region = (l_catch, l_catch_end);
        } else if !has_finally {
            return syntax(self.tok.start, "expecting catch or finally");
        }
        if has_finally {
            self.blocks.pop();
            let l_exc = self.new_label();
            self.place(l_exc);
            self.f.handlers.push(HandlerLabels {
                start: exc_region.0,
                end: exc_region.1,
                target: l_exc,
                depth: base,
            });
            self.code().gosub(l_fin);
            self.code().throw();
            self.expect(TokenKind::Finally, "finally")?;
            self.place(l_fin);
            self.stmt_depth = base + 2;
            self.parse_block()?;
            self.stmt_depth = base;
            self.code().ret();
        }
        self.place(l_end);
        Ok(())
    }

    fn parse_throw(&mut self) -> PResult<()> {
        let pos = self.tok.start;
        self.next()?;
        if self.tok.newline_before() {
            return syntax(self.tok.start, "line terminator not allowed after throw");
        }
        self.parse_expr(false)?;
        self.mark(pos);
        self.code().throw();
        self.expect_semicolon()
    }

    /// `switch`: the discriminant stays on the stack during the case tests. Each test
    /// is placed right before its body in source order:
    /// `L_test_i: dup; <case>; strict_eq; jump_if_false L_test_i+1; drop; L_body_i:
    /// <body>; jump L_body_i+1`. When no case matches: `drop` and jump to `default` or
    /// the end.
    fn parse_switch(&mut self, labels: Vec<Atom>) -> PResult<()> {
        self.next()?;
        self.parse_paren_expr()?;
        self.expect(TokenKind::LBrace, "{")?;
        self.push_scope(ScopeKind::Switch)?;
        let l_break = self.new_label();
        self.blocks.push(BlockEnv {
            labels,
            kind: BlockKind::Switch,
            break_label: l_break,
            continue_label: None,
            depth: self.stmt_depth,
            used: false,
        });
        let mut next_test = self.new_label();
        let mut next_body: Option<crate::bytecode::Label> = None;
        let mut default_body = None;
        self.code().jump(next_test);
        while !self.is(TokenKind::RBrace) {
            let body = next_body.take().unwrap_or_else(|| self.new_label());
            match self.tok.kind {
                TokenKind::Case => {
                    self.next()?;
                    self.place(next_test);
                    next_test = self.new_label();
                    self.code().dup();
                    self.parse_expr(false)?;
                    self.code().strict_eq();
                    self.code().jump_if_false(next_test);
                    self.code().drop();
                    self.expect(TokenKind::Colon, ":")?;
                }
                TokenKind::Default => {
                    if default_body.is_some() {
                        return syntax(self.tok.start, "duplicate default");
                    }
                    self.next()?;
                    self.expect(TokenKind::Colon, ":")?;
                    default_body = Some(body);
                }
                _ => return syntax(self.tok.start, "invalid switch statement"),
            }
            self.place(body);
            while !matches!(
                self.tok.kind,
                TokenKind::Case | TokenKind::Default | TokenKind::RBrace
            ) {
                if self.is(TokenKind::Eof) {
                    return syntax(self.tok.start, "expecting '}'");
                }
                self.parse_statement_list_item()?;
            }
            // Fall through into the next body, skipping its test.
            let nb = self.new_label();
            self.code().jump(nb);
            next_body = Some(nb);
        }
        self.next()?;
        // No case matched.
        self.place(next_test);
        self.code().drop();
        self.code().jump(default_body.unwrap_or(l_break));
        if let Some(nb) = next_body {
            self.place(nb);
        }
        self.place(l_break);
        self.blocks.pop();
        self.pop_scope();
        Ok(())
    }

    fn parse_labeled(&mut self) -> PResult<()> {
        let mut labels = Vec::new();
        while self.is(TokenKind::Ident) && self.peek().kind == TokenKind::Colon {
            let tok = self.tok;
            if tok.is_escaped_reserved() || tok.kw.is_strict_reserved() {
                return self.unexpected();
            }
            let a = self.ident_atom()?;
            if labels.contains(&a) || self.blocks.iter().any(|b| b.labels.contains(&a)) {
                return syntax(tok.start, "duplicate label name");
            }
            labels.push(a);
            self.next()?;
            self.next()?;
        }
        match self.tok.kind {
            TokenKind::While => self.parse_while(labels),
            TokenKind::Do => self.parse_do_while(labels),
            TokenKind::For => self.parse_for(labels),
            _ => {
                let l_break = self.new_label();
                self.blocks.push(BlockEnv {
                    labels,
                    kind: BlockKind::Labeled,
                    break_label: l_break,
                    continue_label: None,
                    depth: self.stmt_depth,
                    used: false,
                });
                if self.is(TokenKind::Function) {
                    return syntax(
                        self.tok.start,
                        "function declarations can't appear in single-statement context",
                    );
                }
                self.parse_statement()?;
                self.place(l_break);
                self.blocks.pop();
                Ok(())
            }
        }
    }
}
