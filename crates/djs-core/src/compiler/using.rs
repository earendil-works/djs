//! `using` and `await using` declarations (0058, docs/compiler.md 7.10).
//!
//! A statement list (block, function body, catch block, codemode body) opens a
//! disposal region at its first `using` declaration: `call_intrinsic %UsingCreate`
//! makes the resource Record (builtins/disposable.ts), which a temporary `s`
//! (`using.record@S`, D2732) holds until the list ends. Each binding's value goes into
//! it with `%UsingAdd`/`%AwaitUsingAdd`. The region is a `try`/`finally` whose finally
//! block is the disposal (a `BlockKind::Finally` env, so `break`, `continue`, `return`
//! and generator returns run it with `gosub`), with `base` the depth of the list:
//!
//! ```text
//!          call_intrinsic %UsingCreate 0; put s
//! L_try:   <statements>
//! L_try_end:
//!          undefined; gosub L_fin; drop; jump L_end
//! L_exc:   (handler for [L_try, L_try_end), depth base: exc)
//!          get s; swap; call_intrinsic %UsingDisposeError 2; gosub L_fin; throw
//! L_fin:   (v ret) get s; call_intrinsic %UsingDispose 1; drop; ret
//! L_end:
//! ```
//!
//! An exception is recorded in the Record first, so the subroutine always throws then.
//! With an `await using` in the list, the subroutine awaits between disposals:
//!
//! ```text
//! L_fin:                                     ; v ret
//! L_loop:  get s; call_intrinsic %UsingDisposeStep 1; jump_if_false L_done
//!          get s; call_intrinsic %UsingAwaitValue 1
//! L_a:     await
//! L_a_end: drop; jump L_loop
//! L_rej:   (handler for [L_a, L_a_end): v ret exc)
//!          get s; swap; call_intrinsic %UsingDisposeError 2; drop; jump L_loop
//! L_done:  ret
//! ```
//!
//! A for-of head (`for (using x of xs)`) opens a region per iteration around the
//! binding and the body; a `for (using x = ...; ...)` head one around the whole loop.
use crate::PushOut;

use alloc::vec::Vec;

use super::PResult;
use super::func::{HandlerLabels, VarKind};
use super::parser::{BlockEnv, BlockKind, Parser, syntax};
use super::smap::{Role, SiteKind};
use crate::bytecode::Label;
use crate::bytecode::intrinsics::intrinsic_id;
use crate::lexer::{Kw, TokenKind};

/// Whether the statement list being parsed may have `using` declarations, and its
/// disposal region once it has one.
#[derive(Clone, Copy, Default)]
pub(super) enum UsingList {
    /// Not here: Script top level, `switch` clauses, single statements in a function
    /// with no statement list yet (namespace bodies, initializers).
    #[default]
    Forbidden,
    /// No `using` declaration yet.
    Allowed,
    Open(UsingRegion),
}

#[derive(Clone, Copy)]
pub(super) struct UsingRegion {
    /// The operand depth of the statement list.
    base: u32,
    /// The resource Record's temporary.
    pub rec: crate::heap::Atom,
    l_try: Label,
    l_fin: Label,
    /// An `await using` is in the list: disposal awaits.
    is_async: bool,
}

fn intrinsic(name: &str) -> u16 {
    intrinsic_id(name).expect("using intrinsic")
}

impl Parser<'_, '_> {
    /// At `using` followed by a binding identifier on the same line. In a for-in/of
    /// head, `using of` is not a declaration (`for (using of of xs)` iterates `using`).
    pub(super) fn at_using(&self, in_for: bool) -> bool {
        if !self.tok.is_contextual(Kw::Using) {
            return false;
        }
        let next = self.peek();
        self.using_binding_follows(next) && !(in_for && next.is_contextual(Kw::Of))
    }

    /// A binding identifier on the same line (as qjs: a reserved word means `using`
    /// is an identifier, so `using let` is an error at `let`, not a declaration).
    fn using_binding_follows(&self, next: crate::lexer::Token) -> bool {
        next.kind == TokenKind::Ident
            && !next.newline_before()
            && !next.is_escaped_reserved()
            && !next.kw.is_strict_reserved()
            && !(next.kw == Kw::Await && self.await_reserved())
    }

    /// At `await using x` (no line breaks) in an async function.
    pub(super) fn at_await_using(&self) -> bool {
        if !(self.f.is_async && self.tok.is_contextual(Kw::Await)) {
            return false;
        }
        let mut lx = self.lx;
        let Ok(u) = lx.next_token() else {
            return false;
        };
        if !u.is_contextual(Kw::Using) || u.newline_before() {
            return false;
        }
        let Ok(n) = lx.next_token() else {
            return false;
        };
        self.using_binding_follows(n)
    }

    /// Parses a statement list with `using` declarations allowed (or not), disposing
    /// them at its end.
    pub(super) fn begin_using_list(&mut self, allowed: bool) -> UsingList {
        let list = if allowed {
            UsingList::Allowed
        } else {
            UsingList::Forbidden
        };
        core::mem::replace(&mut self.using, list)
    }

    /// Ends the statement list begun with [`Parser::begin_using_list`]; `saved` is its
    /// result.
    pub(super) fn end_using_list(&mut self, saved: UsingList) {
        if let UsingList::Open(r) = core::mem::replace(&mut self.using, saved) {
            self.close_using(r);
        }
    }

    /// `using x = v, ...;` or `await using x = v, ...;` in a statement list.
    pub(super) fn parse_using_declaration(&mut self, is_await: bool) -> PResult<()> {
        if is_await {
            self.next()?;
        }
        self.next()?;
        // qjs reports a misplaced declaration at its first binding.
        let rec = self.open_using(self.tok.start, is_await)?;
        self.parse_using_bindings(rec, is_await, false)?;
        self.expect_semicolon()
    }

    /// The list's region starts at its first declaration (nothing to dispose before).
    /// Returns the region's Record temporary.
    pub(super) fn open_using(&mut self, pos: u32, is_await: bool) -> PResult<crate::heap::Atom> {
        match &mut self.using {
            UsingList::Forbidden => syntax(
                pos,
                if is_await {
                    "await using declaration not allowed here"
                } else {
                    "using declaration not allowed here"
                },
            ),
            UsingList::Open(r) => {
                r.is_async |= is_await;
                Ok(r.rec)
            }
            UsingList::Allowed => {
                let r = self.using_region_start(self.stmt_depth, is_await, pos);
                self.using_env(&r);
                self.using = UsingList::Open(r);
                Ok(r.rec)
            }
        }
    }

    /// Makes the Record and starts the region (`base` is the operand depth). `at`, the
    /// start of the region's first bound name, names the Record's temporary.
    pub(super) fn using_region_start(&mut self, base: u32, is_await: bool, at: u32) -> UsingRegion {
        self.code().call_intrinsic(intrinsic("UsingCreate"), 0);
        let rec = self.temp("using.record", at);
        self.temp_put(rec);
        let l_try = self.new_label();
        let l_fin = self.new_label();
        self.place(l_try);
        UsingRegion {
            base,
            rec,
            l_try,
            l_fin,
            is_async: is_await,
        }
    }

    /// The region's `Finally` env: jumps out of it dispose.
    pub(super) fn using_env(&mut self, r: &UsingRegion) {
        self.blocks.push_out(BlockEnv {
            labels: Vec::new(),
            kind: BlockKind::Finally,
            break_label: r.l_fin,
            continue_label: None,
            depth: r.base,
            used: false,
        });
        self.stmt_depth = r.base;
    }

    /// The bindings after `using`: identifiers with initializers (`no_in` in a `for`
    /// head), added to the Record in temporary `rec`.
    pub(super) fn parse_using_bindings(
        &mut self,
        rec: crate::heap::Atom,
        is_await: bool,
        no_in: bool,
    ) -> PResult<()> {
        // The slim build has no `using` (D2411).
        if !crate::FULL {
            return syntax(
                self.tok.start,
                "`using` declarations are not available in this build",
            );
        }
        loop {
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                return syntax(
                    self.tok.start,
                    "binding patterns are not allowed in using declarations",
                );
            }
            let pos = self.tok.start;
            let name = self.binding_name()?;
            self.next()?;
            self.ts_annotation()?;
            let v = self.declare_lexical(name, VarKind::Const, self.tok.start)?;
            if !self.eat(TokenKind::Assign)? {
                return syntax(self.tok.start, "missing initializer for using declaration");
            }
            self.parse_named(name, |p| p.parse_assign(no_in))?;
            self.mark(pos);
            self.emit_using_add(rec, is_await);
            let r = self.init_ref(name, pos);
            self.code().name_init(r);
            self.f.vars[v as usize].init_end = self.prev_end;
            if !self.eat(TokenKind::Comma)? {
                return Ok(());
            }
        }
    }

    /// `v -> v`: adds the value to the Record in temporary `rec`
    /// (AddDisposableResource).
    pub(super) fn emit_using_add(&mut self, rec: crate::heap::Atom, is_await: bool) {
        self.temp_get(rec);
        self.code().swap();
        let add = if is_await {
            "AwaitUsingAdd"
        } else {
            "UsingAdd"
        };
        self.code().call_intrinsic(intrinsic(add), 2);
    }

    /// Ends a region: the normal exit, the exception handler and the disposal
    /// subroutine. The operand depth is back under the Record.
    pub(super) fn close_using(&mut self, r: UsingRegion) {
        let env = self.blocks.pop();
        debug_assert!(
            env.is_some_and(|e| e.kind == BlockKind::Finally && e.break_label == r.l_fin)
        );
        let l_try_end = self.new_label();
        let l_exc = self.new_label();
        let l_end = self.new_label();
        self.place(l_try_end);
        self.code().undefined();
        self.code().gosub(r.l_fin);
        self.code().drop();
        self.code().jump(l_end);
        self.f.handlers.push_out(HandlerLabels {
            start: r.l_try,
            end: l_try_end,
            target: l_exc,
            depth: r.base,
            depth_from: None,
        });
        self.place(l_exc);
        self.temp_get(r.rec);
        self.code().swap();
        self.code()
            .call_intrinsic(intrinsic("UsingDisposeError"), 2);
        self.code().gosub(r.l_fin);
        self.code().throw();
        let sub = self.subroutine(r.l_fin);
        self.place(r.l_fin);
        if r.is_async {
            let (l_loop, l_done) = (self.new_label(), self.new_label());
            let (l_a, l_a_end, l_rej) = (self.new_label(), self.new_label(), self.new_label());
            self.place(l_loop);
            self.temp_get(r.rec);
            self.code().call_intrinsic(intrinsic("UsingDisposeStep"), 1);
            self.code().jump_if_false(l_done);
            self.temp_get(r.rec);
            self.code().call_intrinsic(intrinsic("UsingAwaitValue"), 1);
            self.place(l_a);
            // `v ret`: the subroutine's value and return address, named by the site
            // (the list's end).
            let rs = [Role::FinallyValue, Role::FinallyReturn(sub)];
            self.roles_push(self.prev_end, &rs);
            self.site(SiteKind::AwaitUsing, self.prev_end);
            self.roles_pop(rs.len());
            self.code().await_(0);
            self.place(l_a_end);
            self.code().drop();
            self.code().jump(l_loop);
            self.f.handlers.push_out(HandlerLabels {
                start: l_a,
                end: l_a_end,
                target: l_rej,
                depth: r.base + 2,
                depth_from: None,
            });
            self.place(l_rej);
            self.temp_get(r.rec);
            self.code().swap();
            self.code()
                .call_intrinsic(intrinsic("UsingDisposeError"), 2);
            self.code().drop();
            self.code().jump(l_loop);
            self.place(l_done);
        } else {
            self.temp_get(r.rec);
            self.code().call_intrinsic(intrinsic("UsingDispose"), 1);
            self.code().drop();
        }
        self.code().ret();
        self.place(l_end);
        self.stmt_depth = r.base;
    }
}
