//! Destructuring patterns (docs/compiler.md 3.3, 3.10).
//!
//! A pattern consumes the value on top of the stack. Its targets are bindings
//! (`let`/`const`/`var`, parameters, catch parameters) or, in assignment patterns,
//! references (`[o.x, a[i]] = v`), evaluated before the value they receive.
//!
//! Patterns usually come before their value in the source (`let [a] = v`, `[a] = v`,
//! `[a] = v` as a default), but their code runs after it. The parser skips the pattern
//! (a balanced-bracket scan), parses and emits what follows, then rewinds the lexer to
//! the pattern, emits it and continues after the value (`deferred_pattern`). Only the
//! lexer moves, nothing emitted is rewritten (D1080, D1360).
//!
//! Object patterns keep the value on the stack and read each property with `pick` +
//! `get_field`/`get_elem`. With a rest property they also keep an array of the keys
//! seen, for `%CopyDataPropertiesExcluding`. Computed keys go through
//! `%ToPropertyKey` once, before the target is evaluated.
//!
//! Array patterns keep `next it` on the stack (the iterator's `next` and the
//! iterator). Each element evaluates its target references first, then steps the
//! iterator, reaching under the references with `pick`. "Done" is stored by poking
//! `undefined` into the iterator slot: later steps then produce `undefined` without
//! calling `next`, and nothing closes the iterator. An exception in the pattern's own
//! code (targets, defaults, stores, nested patterns) closes the iterator through a
//! handler with `%IteratorCloseThrow`; the steps are left out of its regions, as an
//! error from the iterator itself does not close it. After the last element a
//! not-done iterator is closed with `%IteratorClose` (D1361).

use alloc::vec::Vec;

use super::PResult;
use super::expr::{Expr, LValue};
use super::func::{BODY_SCOPE, FuncKind, HandlerLabels, VarKind};
use super::parser::{Checkpoint, Parser, syntax};
use super::scan::Follow;
use crate::bytecode::Label;
use crate::bytecode::intrinsics::intrinsic_id;
use crate::heap::{Atom, atoms};
use crate::lexer::{Kw, TokenKind};

/// How a pattern binds its names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Binding {
    /// `var`: assignments to the hoisted bindings.
    Var,
    /// `let` / `const` (and catch parameters, as `let`): initializations.
    Lexical(VarKind),
    /// Formal parameters.
    Param,
    /// An assignment pattern: the targets are references.
    Assign,
    /// `export const {a} = ...` in a TypeScript namespace: the names are members of the
    /// namespace (compiler/ts_decl.rs), stored like `var`s.
    Member,
}

/// An element's target, after its references (if any) are on the stack.
#[derive(Clone, Copy)]
enum Target<'a> {
    /// A declared name (binding patterns).
    Bind { name: Atom, pos: u32 },
    /// A reference (assignment patterns): 0, 1 or 2 values on the stack.
    Ref(LValue),
    /// A nested pattern, skipped: it is parsed once its value is on the stack.
    Pattern(Checkpoint<'a>),
}

impl Target<'_> {
    /// Values the target keeps on the stack until its store.
    fn refs(&self) -> u8 {
        match self {
            Target::Ref(lv) => lv.refs(),
            _ => 0,
        }
    }
}

fn intrinsic(name: &str) -> u16 {
    intrinsic_id(name).expect("destructuring intrinsic")
}

impl<'a> Parser<'a, '_> {
    /// Is the current `{`/`[` the start of an assignment pattern (`[a, b] = v`)? The
    /// memoized scan of docs/compiler.md 3.3.
    pub(super) fn starts_assignment_pattern(&mut self, follows: Follow) -> bool {
        matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket)
            && self.pat_scan.follow(self.lx, self.tok) == follows
    }

    /// With the current token at a pattern that comes before its value in the source:
    /// skips the pattern, runs `value` (which parses and emits what follows it,
    /// leaving the value on the stack), then emits the pattern and continues after
    /// what `value` parsed.
    pub(super) fn deferred_pattern(
        &mut self,
        b: Binding,
        value: impl FnOnce(&mut Self) -> PResult<()>,
    ) -> PResult<()> {
        let cp = self.skip_pattern(b)?;
        value(self)?;
        self.pattern_at(cp, b)
    }

    /// Emits the pattern skipped at `cp` (its value on the stack) and returns to the
    /// current position.
    fn pattern_at(&mut self, cp: Checkpoint<'a>, b: Binding) -> PResult<()> {
        let end = self.checkpoint();
        self.rewind(cp);
        self.parse_pattern(b)?;
        self.rewind(end);
        Ok(())
    }

    /// Skips the pattern at the current `{`/`[` and returns where it starts.
    pub(super) fn skip_pattern(&mut self, b: Binding) -> PResult<Checkpoint<'a>> {
        let cp = self.checkpoint();
        match super::scan::skip_group(self.lx, self.tok) {
            Some((lx, end)) => {
                self.lx = lx;
                self.prev_end = end;
                self.tok = self.lx.next_token()?;
                Ok(cp)
            }
            None => {
                // It does not scan (unbalanced, a lexing error): parsing it in place
                // reports the real error.
                self.code().undefined();
                self.parse_pattern(b)?;
                syntax(cp.tok_start(), "invalid destructuring target")
            }
        }
    }

    /// Takes the bindings declared by patterns since `mark` and sets where their
    /// initialization ends (docs/compiler.md 5.6).
    pub(super) fn end_pattern_bindings(&mut self, mark: usize, init_end: u32) {
        for v in self.pattern_vars.drain(mark..) {
            self.f.vars[v as usize].init_end = init_end;
        }
    }

    /// A pattern at the current token (`{` or `[`), consuming the value on the stack.
    pub(super) fn parse_pattern(&mut self, b: Binding) -> PResult<()> {
        self.enter()?;
        let r = if self.is(TokenKind::LBrace) {
            self.parse_object_pattern(b)
        } else {
            self.parse_array_pattern(b)
        };
        self.leave();
        r
    }

    /// Declares a name of a binding pattern. `pos` is where qjs reports a conflict
    /// (the token after the name).
    fn declare_pattern_name(&mut self, b: Binding, name: Atom, pos: u32) -> PResult<()> {
        let v = match b {
            Binding::Var => {
                self.declare_var(name, pos)?;
                return Ok(());
            }
            Binding::Lexical(kind) => self.declare_lexical(name, kind, pos)?,
            Binding::Param => {
                if self.f.find_in_scope(BODY_SCOPE, name).is_some() {
                    return syntax(pos, "duplicate argument names not allowed in this context");
                }
                let v = self.f.add_var(name, VarKind::Param, BODY_SCOPE);
                self.f.params.push(v);
                v
            }
            Binding::Member => {
                self.ts_pattern_member(name);
                return Ok(());
            }
            Binding::Assign => unreachable!("assignment patterns declare nothing"),
        };
        self.pattern_vars.push(v);
        Ok(())
    }

    /// The target of an element or property value, with its references pushed.
    fn parse_target(&mut self, b: Binding) -> PResult<Target<'a>> {
        if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket)
            && (b != Binding::Assign
                || matches!(
                    self.pat_scan.follow(self.lx, self.tok),
                    Follow::Assign | Follow::ElemEnd
                ))
        {
            return Ok(Target::Pattern(self.skip_pattern(b)?));
        }
        if b == Binding::Assign {
            let e = self.parse_lhs(false)?;
            return Ok(Target::Ref(self.pattern_lvalue(e, false)?));
        }
        self.parse_binding_target(b)
    }

    /// A binding identifier as a target.
    fn parse_binding_target(&mut self, b: Binding) -> PResult<Target<'a>> {
        let pos = self.tok.start;
        if matches!(self.tok.kw, Kw::Eval | Kw::Arguments) && self.tok.kind == TokenKind::Ident {
            return syntax(pos, "invalid destructuring target");
        }
        let name = self.binding_name()?;
        self.next()?;
        self.declare_pattern_name(b, name, self.tok.start)?;
        Ok(Target::Bind { name, pos })
    }

    fn pattern_lvalue(&mut self, e: Expr, shorthand: bool) -> PResult<LValue> {
        if let Expr::Name {
            restricted: true, ..
        } = e
        {
            // `eval` / `arguments`, reported as qjs does.
            let msg = if shorthand {
                "invalid destructuring target"
            } else {
                "invalid lvalue in strict mode"
            };
            return syntax(self.tok.start, msg);
        }
        self.lvalue(e, "invalid destructuring target")
    }

    /// The name a default value's anonymous function gets (NamedEvaluation applies
    /// to identifier targets only).
    fn target_name(&self, t: &Target<'_>) -> Atom {
        match t {
            Target::Bind { name, .. } => *name,
            Target::Ref(LValue::Name { r, .. }) => self.f.refs[*r as usize].atom,
            _ => Atom::NULL,
        }
    }

    /// `= default` after a target, if present: replaces an `undefined` value.
    fn parse_pattern_default(&mut self, name: Atom) -> PResult<bool> {
        if !self.is(TokenKind::Assign) {
            return Ok(false);
        }
        self.next()?;
        let l = self.new_label();
        self.code().dup();
        self.code().is_undefined();
        self.code().jump_if_false(l);
        self.code().drop();
        if name != Atom::NULL {
            self.name_hint = Some((name, self.tok.start));
        }
        self.parse_assign(false)?;
        self.place(l);
        Ok(true)
    }

    /// Stores the value on top of the stack into the target.
    fn store_target(&mut self, b: Binding, t: Target<'a>) -> PResult<()> {
        match t {
            Target::Bind { name, pos } => {
                if matches!(b, Binding::Var | Binding::Member) {
                    let r = self.name_ref(name, pos);
                    self.mark(pos);
                    self.code().name_put(r);
                } else {
                    let r = self.init_ref(name, pos);
                    self.mark(pos);
                    self.code().name_init(r);
                }
            }
            Target::Ref(lv) => self.emit_store(lv, false),
            Target::Pattern(cp) => self.pattern_at(cp, b)?,
        }
        Ok(())
    }

    // ---- Object patterns -------------------------------------------------------------

    /// `{ a, b: c = 1, [k]: d, ...rest }`.
    fn parse_object_pattern(&mut self, b: Binding) -> PResult<()> {
        let open = self.tok;
        let rest = super::scan::object_has_rest(self.lx, open);
        self.next()?;
        self.mark(open.start);
        self.code().require_object_coercible();
        if rest {
            // The keys of the properties before the rest, to exclude from it.
            self.code().array_from(0);
        }
        let s = u8::from(rest);
        while !self.is(TokenKind::RBrace) {
            if self.is(TokenKind::Ellipsis) {
                let pos = self.tok.start;
                self.next()?;
                let t = if b == Binding::Assign {
                    let e = self.parse_lhs(false)?;
                    Target::Ref(self.pattern_lvalue(e, false)?)
                } else {
                    if !self.is(TokenKind::Ident) {
                        return syntax(self.tok.start, "invalid destructuring target");
                    }
                    self.parse_binding_target(b)?
                };
                let r = t.refs();
                self.code().pick(r + 1);
                self.code().pick(r + 1);
                self.mark(pos);
                self.code()
                    .call_intrinsic(intrinsic("CopyDataPropertiesExcluding"), 2);
                self.store_target(b, t)?;
                if !self.is(TokenKind::RBrace) {
                    return syntax(self.tok.start, "assignment rest property must be last");
                }
                break;
            }
            let key_tok = self.tok;
            let shorthand = key_tok.kind == TokenKind::Ident
                && matches!(
                    self.peek().kind,
                    TokenKind::Comma | TokenKind::RBrace | TokenKind::Assign
                );
            let (key, t) = if shorthand {
                let a = self.ident_atom()?;
                if rest {
                    self.code().push_atom(a);
                    self.code().array_push();
                }
                let t = if b == Binding::Assign {
                    let e = self.parse_lhs(false)?;
                    Target::Ref(self.pattern_lvalue(e, true)?)
                } else {
                    if matches!(key_tok.kw, Kw::Eval | Kw::Arguments) {
                        return syntax(self.peek().start, "invalid destructuring target");
                    }
                    self.parse_binding_target(b)?
                };
                (Some(a), t)
            } else {
                let key = self.parse_property_key()?;
                match key {
                    Some(a) if rest => {
                        self.code().push_atom(a);
                        self.code().array_push();
                    }
                    Some(_) => {}
                    None => {
                        // Converted once, before the target is evaluated.
                        self.mark(key_tok.start);
                        self.code().call_intrinsic(intrinsic("ToPropertyKey"), 1);
                        if rest {
                            // `v excl k` -> `v excl k`, with k appended to excl.
                            self.code().insert2();
                            self.code().array_push();
                            self.code().swap();
                        }
                    }
                }
                if !self.is(TokenKind::Colon) {
                    return syntax(self.tok.start, "expecting ':'");
                }
                self.next()?;
                (key, self.parse_target(b)?)
            };
            let r = t.refs();
            match key {
                Some(a) => {
                    self.code().pick(s + r);
                    self.mark(key_tok.start);
                    self.code().get_field(a, 0);
                }
                None => {
                    self.code().pick(s + 1 + r);
                    self.code().pick(r + 1);
                    self.mark(key_tok.start);
                    self.code().get_elem();
                }
            }
            let name = self.target_name(&t);
            self.parse_pattern_default(name)?;
            self.store_target(b, t)?;
            if key.is_none() {
                self.code().drop();
            }
            if !self.eat(TokenKind::Comma)? {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "}")?;
        if rest {
            self.code().drop();
        }
        self.code().drop();
        Ok(())
    }

    // ---- Array patterns --------------------------------------------------------------

    /// `[a, , b = 1, [c], ...d]`.
    fn parse_array_pattern(&mut self, b: Binding) -> PResult<()> {
        let open = self.tok;
        self.next()?;
        self.mark(open.start);
        self.code().call_intrinsic(intrinsic("GetIterator"), 1);
        self.code().dup();
        self.code().get_field(atoms::NEXT, 0);
        self.code().swap();
        // `next it`: the close handler's depth is the depth here.
        let l_base = self.new_label();
        self.place(l_base);
        self.open_iters
            .push(super::parser::OpenIter::Pattern(l_base));
        let mut regions = Regions {
            list: Vec::new(),
            start: l_base,
            start_pc: self.f.code.pos(),
        };
        // Only code that can throw needs the close handler: binding stores cannot
        // (except assignments to globals), defaults, references and nested patterns can.
        let mut may_throw = matches!(b, Binding::Assign | Binding::Member)
            || (b == Binding::Var && self.f.kind == FuncKind::Script);
        let mut first = true;
        loop {
            match self.tok.kind {
                TokenKind::RBracket => break,
                TokenKind::Comma => {
                    // An elision: one step, value dropped.
                    self.region_end(&mut regions);
                    self.emit_iter_step(0, first, open.start);
                    self.region_start(&mut regions);
                    self.code().drop();
                    first = false;
                    self.next()?;
                    continue;
                }
                TokenKind::Ellipsis => {
                    self.next()?;
                    let t = self.parse_target(b)?;
                    if !self.is(TokenKind::RBracket) {
                        return syntax(self.tok.start, "rest element must be the last one");
                    }
                    may_throw |= matches!(t, Target::Pattern(_));
                    self.region_end(&mut regions);
                    self.emit_iter_rest(t.refs(), first, open.start);
                    self.region_start(&mut regions);
                    self.store_target(b, t)?;
                    break;
                }
                _ => {
                    let t = self.parse_target(b)?;
                    may_throw |= matches!(t, Target::Pattern(_));
                    self.region_end(&mut regions);
                    self.emit_iter_step(t.refs(), first, open.start);
                    self.region_start(&mut regions);
                    let name = self.target_name(&t);
                    may_throw |= self.parse_pattern_default(name)?;
                    self.store_target(b, t)?;
                    first = false;
                }
            }
            match self.tok.kind {
                TokenKind::Comma => self.next()?,
                TokenKind::RBracket => break,
                _ => return syntax(self.tok.start, "expecting ','"),
            }
        }
        self.expect(TokenKind::RBracket, "]")?;
        self.open_iters.pop();
        self.region_end(&mut regions);
        // Close a not-done iterator: `next it` -> ``.
        let l_skip = self.new_label();
        self.code().nip();
        self.code().dup();
        self.code().is_undefined();
        self.code().jump_if_true(l_skip);
        self.mark(open.start);
        self.code().call_intrinsic(intrinsic("IteratorClose"), 1);
        self.place(l_skip);
        self.code().drop();
        if may_throw && !regions.list.is_empty() {
            // `next it exc`: close a not-done iterator, rethrow.
            let l_after = self.new_label();
            let l_handler = self.new_label();
            let l_rethrow = self.new_label();
            self.code().jump(l_after);
            self.place(l_handler);
            self.code().swap();
            self.code().dup();
            self.code().is_undefined();
            self.code().jump_if_true(l_rethrow);
            self.code()
                .call_intrinsic(intrinsic("IteratorCloseThrow"), 1);
            self.place(l_rethrow);
            self.code().drop();
            self.code().throw();
            self.place(l_after);
            for (start, end) in regions.list {
                self.f.handlers.push(HandlerLabels {
                    start,
                    end,
                    target: l_handler,
                    depth: 0,
                    depth_from: Some(l_base),
                });
            }
        }
        Ok(())
    }

    fn region_end(&mut self, regions: &mut Regions) {
        if self.f.code.pos() != regions.start_pc {
            let end = self.new_label();
            self.place(end);
            regions.list.push((regions.start, end));
        }
    }

    fn region_start(&mut self, regions: &mut Regions) {
        regions.start = self.new_label();
        self.place(regions.start);
        regions.start_pc = self.f.code.pos();
    }

    /// `next it refs -> next it' refs v`: the next value, or `undefined` once done
    /// (then `it'` is `undefined`). `refs` values sit on top of the iterator. `first`:
    /// the iterator cannot be done yet.
    fn emit_iter_step(&mut self, refs: u8, first: bool, pos: u32) {
        let l_end = self.new_label();
        let l_done = self.new_label();
        // `it next`, for `iter_next` (D1800).
        self.code().pick(refs);
        if !first {
            // A done iterator is `undefined`, which is also the value.
            self.code().dup();
            self.code().is_undefined();
            self.code().jump_if_true(l_end);
        }
        self.code().pick(refs + 2);
        self.mark(pos);
        let l_value = self.new_label();
        self.code().iter_next(l_value);
        self.code().iter_check_object();
        self.code().dup();
        self.code().get_field(atoms::DONE, 0);
        self.code().jump_if_true(l_done);
        self.code().get_field(atoms::VALUE, 0);
        self.place(l_value);
        self.code().jump(l_end);
        self.place(l_done);
        self.code().drop();
        self.code().undefined();
        self.code().poke(refs);
        self.code().undefined();
        self.place(l_end);
    }

    /// `next it refs -> next undefined refs arr`: the remaining values.
    fn emit_iter_rest(&mut self, refs: u8, first: bool, pos: u32) {
        let l_end = self.new_label();
        let l_empty = self.new_label();
        self.code().pick(refs);
        if !first {
            self.code().dup();
            self.code().is_undefined();
            self.code().jump_if_true(l_empty);
        }
        self.code().pick(refs + 2);
        self.mark(pos);
        self.code().call_intrinsic(intrinsic("IteratorRest"), 2);
        self.code().undefined();
        self.code().poke(refs + 1);
        if !first {
            self.code().jump(l_end);
            self.place(l_empty);
            self.code().drop();
            self.code().array_from(0);
            self.place(l_end);
        }
    }
}

/// The close handler's regions of an array pattern: everything but the steps.
struct Regions {
    list: Vec<(Label, Label)>,
    start: Label,
    start_pc: u32,
}
