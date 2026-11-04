//! `for-in` and `for-of` loops (docs/compiler.md 3.8, 4.5, 7.4).
//!
//! The left side is a per-iteration assignment target that appears before the
//! iterated expression in the source but runs after it, every iteration. Code is
//! emitted in source order, so the layout jumps around it. With an expression on the
//! left (`for (a.b of xs)`), as in QuickJS:
//!
//! ```text
//!     jump L_rhs
//! L_assign:                      ; for-of: it next result / for-in: it key
//!     [get_field value]          ; for-of: it next value
//!     <lhs reference; store>     ; (for-of: region 1 of the close handler)
//!     jump L_body
//! L_rhs:
//!     <rhs>; <setup>             ; for-of: it next / for-in: it
//!     jump L_step
//!     [L_throw: ...]             ; for-of: the close handler, see below
//! L_body:
//!     <body>                     ; (for-of: region 2 of the close handler)
//! L_step:                        ; the continue target
//!     <step>; jump_if_false L_assign
//!     <done: drop the loop state>
//! ```
//!
//! With a declaration on the left (`for (const x of xs)`) nothing of the left side runs
//! before the right side, so the store moves next to the body and the extra jump goes:
//! `<rhs>; <setup>; jump L_step; [L_throw]; L_assign: <store>; <body>; L_step: ...`.
//!
//! for-of keeps `it next` on the stack. Setup is `call_intrinsic %GetIterator 1; dup;
//! get_field next`, the step is `dup2; iter_next L_value; iter_check_object; dup;
//! get_field done` (D1290; for-await: `dup2; swap; call 0; await; ...`). `iter_next`
//! jumps to `L_value`, right after `get_field value`, with the value itself when it
//! needs no result object (D1800). The lhs store and the body are covered by a handler (`L_throw`) that
//! closes the iterator with `%IteratorCloseThrow` and rethrows; errors from the
//! iterator itself (`next`, `done`, `value`) are not covered, so they do not close it.
//! `break`, `continue` and `return` leaving the loop close it through the `L_close`
//! subroutine (`BlockKind::Iter`), which sits outside the handler so that an error
//! from `return()` propagates instead of closing twice. It is entered on
//! `v it next ret` (`v` a return value or `undefined`) and, as `gosub` keeps the
//! depth, leaves `v x undefined` for the caller to drop. A `break` of the loop itself
//! goes to `L_break`, which closes inline (it is outside the handler too).
//!
//! for-in keeps the key iterator of `for_in_start` on the stack; its step is
//! `for_in_next` and leaving the loop just drops it.
use crate::PushOut;

use alloc::vec::Vec;

use super::PResult;
use super::func::{HandlerLabels, ScopeKind, VarKind};
use super::parser::{BlockEnv, BlockKind, Checkpoint, OpenIter, Parser, syntax};
use super::pattern::Binding;
use super::scan::Follow;
use super::smap::{Loop, Role, SiteKind};
use crate::bytecode::Label;
use crate::bytecode::intrinsics::intrinsic_id;
use crate::heap::{Atom, atoms};
use crate::lexer::{Kw, TokenKind};

/// The left side of a for-in/of head, as far as the store needs it.
#[derive(Clone, Copy)]
enum Target<'a> {
    /// `var x` or `let x` / `const x`: a store to a declared name.
    Decl { name: Atom, pos: u32, lexical: bool },
    /// `var [x]` or `let {x}`: a pattern, skipped; it is parsed at the store.
    Pattern { cp: Checkpoint<'a>, b: Binding },
    /// An expression target: the store was emitted at `L_assign` already.
    Expr,
}

impl<'a> Parser<'a, '_> {
    /// A for-in/of loop after `for (`, with the current token at the start of the
    /// head. `for_pos` is the `for` keyword (errors from the iteration report there,
    /// as in QuickJS).
    // Out of line: its frame is not on the stack while a C-style loop's body nests
    // (D1941).
    #[inline(never)]
    pub(super) fn parse_for_in_of(
        &mut self,
        labels: Vec<Atom>,
        for_pos: u32,
        is_await: bool,
    ) -> PResult<()> {
        let base = self.stmt_depth;
        let outer = self.scope;
        let mut scoped = false;
        let l_assign = self.new_label();
        // for-of: after `get_field value`, where `iter_next` jumps with the value.
        let l_value = self.new_label();
        let l_body = self.new_label();
        // Handler regions of the for-of close handler: [start, end) pairs.
        let mut regions: Vec<(Label, Label)> = Vec::new();
        // `for (using x of xs)`: Some(is_await).
        let mut using_head = None;
        let target = match self.tok.kind {
            TokenKind::Ident if self.at_using(true) || self.at_await_using() => {
                let is_await = self.tok.kw == Kw::Await;
                if is_await {
                    self.next()?;
                }
                self.next()?;
                self.push_scope(ScopeKind::For)?;
                scoped = true;
                if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                    return syntax(
                        self.tok.start,
                        "binding patterns are not allowed in using declarations",
                    );
                }
                using_head = Some(is_await);
                let (name, pos) = self.for_binding()?;
                self.declare_lexical(name, VarKind::Const, pos)?;
                Target::Decl {
                    name,
                    pos,
                    lexical: true,
                }
            }
            TokenKind::Var => {
                self.next()?;
                if let Some(t) = self.for_pattern(Binding::Var)? {
                    t
                } else {
                    let (name, pos) = self.for_binding()?;
                    self.declare_var(name, self.tok.start)?;
                    Target::Decl {
                        name,
                        pos,
                        lexical: false,
                    }
                }
            }
            TokenKind::Const => {
                self.next()?;
                self.push_scope(ScopeKind::For)?;
                scoped = true;
                if let Some(t) = self.for_pattern(Binding::Lexical(VarKind::Const))? {
                    t
                } else {
                    let (name, pos) = self.for_binding()?;
                    self.declare_lexical(name, VarKind::Const, pos)?;
                    Target::Decl {
                        name,
                        pos,
                        lexical: true,
                    }
                }
            }
            TokenKind::Ident if self.tok.is_contextual(Kw::Let) => {
                self.next()?;
                self.push_scope(ScopeKind::For)?;
                scoped = true;
                if let Some(t) = self.for_pattern(Binding::Lexical(VarKind::Let))? {
                    t
                } else {
                    let (name, pos) = self.for_binding()?;
                    self.declare_lexical(name, VarKind::Let, pos)?;
                    Target::Decl {
                        name,
                        pos,
                        lexical: true,
                    }
                }
            }
            _ => {
                let is_of = is_await || self.for_expr_head_is_of()?;
                let l_rhs = self.new_label();
                self.code().jump(l_rhs);
                self.place(l_assign);
                if is_of {
                    self.code().get_field(atoms::VALUE, 0);
                    self.place(l_value);
                }
                let start = self.new_label();
                let end = self.new_label();
                self.place(start);
                if is_of {
                    self.open_iters
                        .push_out(OpenIter::ForOf(base + 2, is_await));
                }
                if self.starts_assignment_pattern(Follow::InOf) {
                    // `for ([a, b] of xs)`: the pattern takes the value.
                    self.parse_pattern(Binding::Assign)?;
                } else {
                    // `it next value` (for-in: `it key`): the reference goes on top of
                    // the value, and the store wants the value on top of the reference.
                    let e = self.parse_lhs(false)?;
                    if self.is(TokenKind::Assign) {
                        return self.for_initializer_error();
                    }
                    let lv = self.lvalue(e, "invalid for in/of left hand-side")?;
                    match lv.refs() {
                        0 => {}
                        1 => self.code().swap(),
                        2 => self.code().rot3l(),
                        // `v this base key`: store a copy, then drop the value.
                        _ => self.code().pick(3),
                    }
                    self.emit_store(lv, false);
                    if lv.refs() == 3 {
                        self.code().drop();
                    }
                }
                if is_of {
                    self.open_iters.pop();
                }
                self.place(end);
                regions.push_out((start, end));
                self.code().jump(l_body);
                self.place(l_rhs);
                Target::Expr
            }
        };
        if matches!(target, Target::Decl { .. } | Target::Pattern { .. })
            && self.is(TokenKind::Assign)
        {
            return self.for_initializer_error();
        }
        let is_of = self.tok.is_contextual(Kw::Of);
        if using_head.is_some() && self.is(TokenKind::In) {
            return syntax(
                self.tok.start,
                "using declaration not allowed in for-in loop",
            );
        }
        if is_await && !is_of {
            return syntax(self.tok.start, "'for await' loop should be used with 'of'");
        }
        if !is_of && !self.is(TokenKind::In) {
            return syntax(
                self.tok.start,
                "expected 'of' or 'in' in for control expression",
            );
        }
        self.next()?;
        // The right side: for-of takes an AssignmentExpression, for-in an Expression.
        // Lexical bindings are in their TDZ here (`for (let x of x)`).
        if is_of {
            self.parse_assign(false)?;
        } else {
            self.parse_expr(false)?;
        }
        self.expect(TokenKind::RParen, ")")?;
        let head_end = self.prev_end;
        if let Target::Decl {
            name,
            lexical: true,
            ..
        } = target
            && let Some(v) = self.f.find_in_scope(self.scope, name)
        {
            // Initialized per iteration before the body runs.
            self.f.vars[v as usize].init_end = self.prev_end;
        }
        self.mark(for_pos);
        if is_of {
            let name = if is_await {
                "GetAsyncIterator"
            } else {
                "GetIterator"
            };
            let get_iterator = intrinsic_id(name).expect("%GetIterator");
            self.code().call_intrinsic(get_iterator, 1);
            self.code().dup();
            self.code().get_field(atoms::NEXT, 0);
            let kind = if is_await { Loop::Await } else { Loop::Of };
            self.roles_push(for_pos, &[Role::LoopIterator(kind), Role::LoopNext(kind)]);
        } else {
            self.code().for_in_start();
            self.roles_push(for_pos, &[Role::LoopIterator(Loop::In)]);
        }
        let l_step = self.new_label();
        self.code().jump(l_step);
        let l_throw = self.new_label();
        if is_of {
            // `it next exc` -> `exc`, after closing the iterator (errors from
            // `return()` lose to the body's exception).
            self.place(l_throw);
            self.code().nip();
            self.code().swap();
            if is_await {
                // A promise that always fulfills (errors from `return()` are dropped).
                let close = intrinsic_id("AsyncIteratorCloseThrow").expect("intrinsic");
                self.code().call_intrinsic(close, 1);
                // In the close handler: the body's exception under the promise, the loop's
                // `it next` consumed.
                let kept = self.roles_take(2);
                self.roles_push(for_pos, &[Role::Exception]);
                self.site(SiteKind::ForAwaitClose, for_pos);
                self.roles_pop(1);
                self.roles_put(&kept);
                self.code().await_(0);
            } else {
                let close = intrinsic_id("IteratorCloseThrow").expect("%IteratorCloseThrow");
                self.code().call_intrinsic(close, 1);
            }
            self.code().drop();
            self.code().throw();
        }
        let state = if is_of { 2 } else { 1 };
        let mut using = None;
        match target {
            Target::Decl { name, pos, lexical } => {
                self.place(l_assign);
                if is_of {
                    self.code().get_field(atoms::VALUE, 0);
                    self.place(l_value);
                }
                let start = self.new_label();
                self.place(start);
                if let Some(is_await) = using_head {
                    // Each iteration's resource is disposed at its end (compiler/using.rs).
                    using = Some(self.using_region_start(base + state, is_await, pos));
                    self.code().swap();
                    self.mark(pos);
                    self.emit_using_add(is_await);
                }
                if lexical {
                    // A fresh binding (Box) per iteration.
                    let s = self.scope as u16;
                    self.code().enter_scope(s);
                    let r = self.init_ref(name, pos);
                    self.mark(pos);
                    self.code().name_init(r);
                } else {
                    let r = self.name_ref(name, pos);
                    self.mark(pos);
                    self.code().name_put(r);
                }
                regions.push_out((start, l_step));
                self.place(l_body);
            }
            Target::Pattern { cp, b } => {
                self.place(l_assign);
                if is_of {
                    self.code().get_field(atoms::VALUE, 0);
                    self.place(l_value);
                }
                let start = self.new_label();
                self.place(start);
                if matches!(b, Binding::Lexical(_)) {
                    // Fresh bindings (Boxes) per iteration.
                    let s = self.scope as u16;
                    self.code().enter_scope(s);
                }
                let mark = self.pattern_vars.len();
                let end = self.checkpoint();
                self.rewind(cp);
                if is_of {
                    self.open_iters
                        .push_out(OpenIter::ForOf(base + 2, is_await));
                }
                self.parse_pattern(b)?;
                if is_of {
                    self.open_iters.pop();
                }
                self.rewind(end);
                // Initialized per iteration before the body runs.
                self.end_pattern_bindings(mark, head_end);
                regions.push_out((start, l_step));
                self.place(l_body);
            }
            Target::Expr => {
                self.place(l_body);
                let start = self.new_label();
                self.place(start);
                regions.push_out((start, l_step));
            }
        }

        // The body.
        let l_close = self.new_label();
        let l_break = self.new_label();
        if is_of {
            self.blocks.push_out(BlockEnv {
                labels: Vec::new(),
                kind: BlockKind::Iter,
                break_label: l_close,
                continue_label: None,
                depth: base,
                used: false,
            });
        }
        self.blocks.push_out(BlockEnv {
            labels,
            kind: BlockKind::Loop,
            break_label: l_break,
            continue_label: Some(l_step),
            depth: base + state,
            used: false,
        });
        self.stmt_depth = base + state;
        if let Some(r) = &using {
            self.using_env(r);
        }
        self.ts_semi = true;
        // On an error the parser state is abandoned (a nested function's may not be
        // restored), so only pop on success.
        self.parse_statement()?;
        if let Some(r) = using {
            self.close_using(r);
        }
        self.stmt_depth = base;
        let loop_env = self.blocks.pop().expect("loop env");
        let iter_env = if is_of { self.blocks.pop() } else { None };

        self.place(l_step);
        if is_of {
            for &(start, end) in &regions {
                self.f.handlers.push_out(HandlerLabels {
                    start,
                    end,
                    target: l_throw,
                    depth: base + 2,
                    depth_from: None,
                });
            }
            self.mark(for_pos);
            self.code().dup2();
            if is_await {
                self.code().swap();
                self.code().call(0);
                self.site(SiteKind::ForAwaitNext, for_pos);
                self.code().await_(0);
            } else {
                self.code().iter_next(l_value);
            }
            self.code().iter_check_object();
            self.code().dup();
            self.code().get_field(atoms::DONE, 0);
            self.code().jump_if_false(l_assign);
            self.code().drop();
            self.code().drop();
            self.code().drop();
            let close_used = iter_env.is_some_and(|e| e.used);
            let l_end = self.new_label();
            if close_used || loop_env.used {
                self.code().jump(l_end);
            }
            let close = intrinsic_id("IteratorClose").expect("%IteratorClose");
            if close_used {
                // `v it next ret` -> `v x undefined` (a subroutine keeps the depth;
                // the caller drops the two), after closing the iterator.
                let sub = self.subroutine(l_close);
                self.place(l_close);
                self.code().rot3r();
                self.code().drop();
                self.mark(for_pos);
                if is_await {
                    // `v ret it` -> `v ret undefined undefined`: only the subroutine's
                    // value and return address stay below the close's await.
                    self.emit_async_close_normal(
                        (SiteKind::ForAwaitClose, for_pos),
                        Some((2, &[Role::FinallyValue, Role::FinallyReturn(sub)])),
                    );
                    self.code().undefined();
                    self.code().undefined();
                } else {
                    self.code().dup();
                    self.code().call_intrinsic(close, 1);
                }
                self.code().rot3l();
                self.code().ret();
            }
            if loop_env.used {
                // `it next` -> ``, after closing the iterator.
                self.place(l_break);
                self.code().drop();
                self.mark(for_pos);
                if is_await {
                    self.emit_async_close_normal(
                        (SiteKind::ForAwaitClose, for_pos),
                        Some((2, &[])),
                    );
                } else {
                    self.code().call_intrinsic(close, 1);
                    self.code().drop();
                }
            }
            self.place(l_end);
        } else {
            self.code().for_in_next();
            self.code().jump_if_false(l_assign);
            self.code().drop();
            self.place(l_break);
            self.code().drop();
        }
        self.roles_pop(if is_of { 2 } else { 1 });
        if scoped {
            self.pop_scope();
        }
        debug_assert_eq!(self.scope, outer);
        Ok(())
    }

    /// A pattern in a declaration head (`for (const [k, v] of ...)`): skipped, as it
    /// runs after the right side.
    fn for_pattern(&mut self, b: Binding) -> PResult<Option<Target<'a>>> {
        if !matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
            return Ok(None);
        }
        let cp = self.skip_pattern(b)?;
        Ok(Some(Target::Pattern { cp, b }))
    }

    /// The binding of a declaration in a for-in/of head and its position.
    fn for_binding(&mut self) -> PResult<(Atom, u32)> {
        let pos = self.tok.start;
        let name = self.binding_name()?;
        self.next()?;
        Ok((name, pos))
    }

    /// For an expression head: whether it is for-of, found by a scan for the `of` or
    /// `in` that ends the left side (it must be known before the left side is
    /// emitted, to emit the `get_field value`). A head without either is for-in here;
    /// the parser reports the error at the real token.
    fn for_expr_head_is_of(&mut self) -> PResult<bool> {
        if self.tok.is_contextual(Kw::Async) && self.peek().is_contextual(Kw::Of) {
            return syntax(
                self.tok.start,
                "'for of' expression cannot start with 'async'",
            );
        }
        Ok(super::scan::for_lhs_ends_with_of(self.lx, self.tok))
    }

    /// `for (x = 1 of ...)`: QuickJS parses the initializer and reports at the token
    /// after it.
    fn for_initializer_error<T>(&mut self) -> PResult<T> {
        self.next()?;
        self.parse_assign(true)?;
        let msg = if self.tok.is_contextual(Kw::Of) {
            "a declaration in the head of a for-of loop can't have an initializer"
        } else if self.is(TokenKind::In) {
            "a declaration in the head of a for-in loop can't have an initializer"
        } else {
            "expected 'of' or 'in' in for control expression"
        };
        syntax(self.tok.start, msg)
    }
}
