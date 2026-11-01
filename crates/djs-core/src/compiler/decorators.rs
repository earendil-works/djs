//! Decorators (TC39 2023-11, 0065, D1650, docs/compiler.md 7.11), after qjs.
//!
//! A decorator list evaluates to an array of decorator and receiver pairs (`@a.b`
//! calls `b` with `this` = `a`). A class element's list goes into a hidden binding
//! before the element's key is evaluated; the class's list is evaluated before `class`
//! (`Parser::class_decs`). Once the body is defined (`ctor proto` on the stack), the
//! class's decoration state is created and the elements are decorated in qjs's order:
//! static methods and accessors, instance ones, static fields, instance fields, each by
//! `%ApplyDecorators` (builtins/decorators.ts). A public method, getter, setter or
//! auto-accessor is read from its home object and redefined there; a private one's
//! closures are hidden bindings (compiler/class.rs) that the result replaces before the
//! initializers install them. Fields and auto-accessors get a Record whose initializers
//! the field initializer runs on the value, and whose extra initializers it runs after
//! the definition. The class decorators come last, after the instance field
//! initializer exists and before the inner binding, the static extra initializers, the
//! static initializer (`this` = the new class) and the class extra initializers.
//!
//! The access object of a private element (`context.access`) has synthesized arrows
//! `(o) => o.#x`, `(o, v) => { o.#x = v; }` and `(o) => #x in o`; an auto-accessor
//! `accessor x` is a private storage name and a synthesized getter and setter
//! `get x() { return this.<storage>; }` (`emit_synth_fn`).

use alloc::format;

use super::PResult;
use super::func::{FuncKind, ScopeId, VarKind};
use super::parser::{Parser, syntax};
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

/// Element kinds and flags of `%ApplyDecorators` (builtins/decorators.ts).
pub(super) const DEC_CLASS: u8 = 0;
pub(super) const DEC_METHOD: u8 = 1;
pub(super) const DEC_GETTER: u8 = 2;
pub(super) const DEC_SETTER: u8 = 3;
pub(super) const DEC_ACCESSOR: u8 = 4;
pub(super) const DEC_FIELD: u8 = 5;
const DEC_STATIC: u8 = 8;
const DEC_PRIVATE: u8 = 16;

/// Slots of the decoration state and of an element Record.
const STATE_STATIC_EXTRAS: i8 = 1;
const STATE_INSTANCE_EXTRAS: i8 = 2;
const STATE_CLASS_EXTRAS: i8 = 3;
const REC_EXTRAS: i8 = 1;
const REC_GET: i8 = 2;
const REC_SET: i8 = 3;

/// The key of a decorated element.
#[derive(Clone, Copy)]
pub(super) enum DecKey {
    Atom(Atom),
    /// The hidden binding of the converted key.
    Computed(Atom),
    /// The private name `#x`.
    Private(Atom),
}

/// A decorated class element.
pub(super) struct DecElem {
    pub kind: u8,
    pub is_static: bool,
    pub key: DecKey,
    /// The hidden binding of the decorator array.
    pub decs: Atom,
    /// Fields and auto-accessors: the hidden binding of their Record.
    pub rec: Atom,
}

/// A synthesized function (`emit_synth_fn`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Synth {
    /// `get x() { return this.#s; }`
    AccessorGet,
    /// `set x(v) { this.#s = v; }`
    AccessorSet,
    /// `(o) => o.#x`
    AccessGet,
    /// `(o, v) => { o.#x = v; }`
    AccessSet,
    /// `(o) => #x in o`
    AccessHas,
}

fn intrinsic(name: &str) -> u16 {
    crate::bytecode::intrinsics::intrinsic_id(name).expect("intrinsic")
}

impl Parser<'_, '_> {
    /// One or more decorators at `@`: pushes the array of decorator and receiver pairs.
    pub(super) fn parse_decorators(&mut self) -> PResult<()> {
        let mut n: u32 = 0;
        while self.is(TokenKind::At) {
            self.parse_decorator()?;
            n += 2;
            if n > u16::MAX as u32 {
                return syntax(self.tok.start, "too many decorators");
            }
        }
        self.code().array_from(n as u16);
        Ok(())
    }

    /// `@` DecoratorMemberExpression, DecoratorCallExpression or `( Expression )`, as
    /// qjs parses it: a primary expression and then `.x`, `.#x` and calls (no `[`,
    /// `?.` or template). Pushes the decorator and its receiver.
    fn parse_decorator(&mut self) -> PResult<()> {
        self.enter()?;
        let at = self.tok.start;
        self.next()?;
        let mut e = match self.tok.kind {
            TokenKind::Ident => self.parse_ident_ref()?,
            TokenKind::LParen => self.parse_primary()?,
            _ => return syntax(at, "invalid decorator"),
        };
        loop {
            let tok = self.tok;
            match tok.kind {
                TokenKind::Dot => {
                    self.next()?;
                    e = self.parse_dot_name(tok.start)?;
                }
                TokenKind::LParen => {
                    self.prepare_call(e);
                    let args = self.parse_args()?;
                    self.mark(tok.start);
                    self.emit_call(args);
                    e = super::expr::Expr::Value;
                }
                _ => break,
            }
        }
        // `dec recv`, like a call's callee and `this`.
        self.prepare_call(e);
        self.leave();
        Ok(())
    }

    /// A statement starting with `@`: the decorators of a class declaration (or, in
    /// strip-types, of `export class`).
    pub(super) fn parse_decorated_statement(&mut self) -> PResult<()> {
        let start = self.tok.start;
        self.parse_decorators()?;
        self.expect_decorated_class(true)?;
        self.class_decs = Some(start);
        self.parse_statement_list_item()?;
        if self.class_decs.take().is_some() {
            return syntax(start, "decorators must be followed by a class");
        }
        Ok(())
    }

    /// After decorators: the current token must start the decorated class.
    pub(super) fn expect_decorated_class(&mut self, allow_export: bool) -> PResult<()> {
        let ok = self.is(TokenKind::Class)
            || (allow_export && self.strip && self.is(TokenKind::Export))
            || (allow_export
                && self.ts
                && self.tok.is_contextual(Kw::Abstract)
                && self.peek().kind == TokenKind::Class);
        if !ok {
            return syntax(self.tok.start, "decorators must be followed by a class");
        }
        Ok(())
    }

    /// A new hidden binding `<dec>N` of the class body scope, numbered like qjs's (an
    /// auto-accessor's storage name shows in its messages).
    pub(super) fn dec_hidden(&mut self, scope: ScopeId, n: &mut u32) -> PResult<Atom> {
        let atom = self.heap.atom_str(&format!("<dec>{n}"))?;
        *n += 1;
        self.f.add_var(atom, VarKind::Hidden, scope);
        Ok(atom)
    }

    /// Emits the closure of a synthesized function on the private name `name`.
    /// Accessors get the function name `fname`.
    pub(super) fn emit_synth_fn(
        &mut self,
        kind: Synth,
        name: Atom,
        fname: Atom,
        pos: u32,
    ) -> PResult<()> {
        let fk = match kind {
            Synth::AccessorGet => FuncKind::Getter,
            Synth::AccessorSet => FuncKind::Setter,
            _ => FuncKind::Arrow,
        };
        self.begin_function(fk)?;
        self.f.name = fname;
        self.f.src_start = pos;
        self.f.src_end = pos;
        let params = match kind {
            Synth::AccessorGet => 0,
            Synth::AccessorSet | Synth::AccessGet | Synth::AccessHas => 1,
            Synth::AccessSet => 2,
        };
        for i in 0..params {
            let a = self.heap.atom_str(&format!("<arg {i}>"))?;
            let v = self.declare_param(a, pos)?;
            self.f.vars[v as usize].init_end = 0;
            self.f.length += 1;
        }
        let method = matches!(kind, Synth::AccessorGet | Synth::AccessorSet);
        if kind == Synth::AccessHas {
            let r = self.name_ref(name, pos);
            self.code().name_get(r);
            self.code().get_arg(0);
            self.mark(pos);
            self.code().private_in();
            self.code().return_();
        } else {
            if method {
                self.emit_this(pos);
            } else {
                self.code().get_arg(0);
            }
            let r = self.name_ref(name, pos);
            self.code().name_get(r);
            self.mark(pos);
            match kind {
                Synth::AccessorGet | Synth::AccessGet => {
                    self.code().get_private_field();
                    self.code().return_();
                }
                _ => {
                    self.code().get_arg(params - 1);
                    self.mark(pos);
                    self.code().put_private_field();
                    self.code().return_undef();
                }
            }
        }
        let child = self.end_function()?;
        self.code().fclosure(child);
        Ok(())
    }

    /// The access object of a private element of kind `kind`.
    fn emit_private_access(&mut self, kind: u8, name: Atom, pos: u32) -> PResult<()> {
        self.code().object();
        if kind != DEC_SETTER {
            self.emit_synth_fn(Synth::AccessGet, name, Atom::NULL, pos)?;
            let a = self.heap.atom_str("get")?;
            self.code().define_field(a);
        }
        if matches!(kind, DEC_SETTER | DEC_FIELD | DEC_ACCESSOR) {
            self.emit_synth_fn(Synth::AccessSet, name, Atom::NULL, pos)?;
            let a = self.heap.atom_str("set")?;
            self.code().define_field(a);
        }
        self.emit_synth_fn(Synth::AccessHas, name, Atom::NULL, pos)?;
        let a = self.heap.atom_str("has")?;
        self.code().define_field(a);
        Ok(())
    }

    /// The hidden binding of a private method's closure (compiler/class.rs).
    pub(super) fn private_closure_atom(&mut self, word: &str, name: Atom) -> PResult<Atom> {
        let hidden = format!("<{word}{}>", self.heap.atom_name(name));
        Ok(self.heap.atom_str(&hidden)?)
    }

    /// With `ctor proto` on the stack: creates the decoration state (in the hidden
    /// binding `state`) and decorates the elements.
    pub(super) fn emit_decorate_elements(
        &mut self,
        elems: &[DecElem],
        state: Atom,
        derived: bool,
        pos: u32,
    ) -> PResult<()> {
        self.code().pick(1);
        if derived {
            self.code().push_true();
        } else {
            self.code().push_false();
        }
        self.mark(pos);
        self.code().call_intrinsic(intrinsic("DecoratorStart"), 2);
        let r = self.init_ref(state, pos);
        self.code().name_init(r);
        for pass in 0..4 {
            for d in elems {
                if d.is_static != (pass & 1 == 0) || (d.kind == DEC_FIELD) != (pass >= 2) {
                    continue;
                }
                self.emit_decorate(d, state, pos)?;
            }
        }
        Ok(())
    }

    /// Decorates one element (`ctor proto` on the stack).
    fn emit_decorate(&mut self, d: &DecElem, state: Atom, pos: u32) -> PResult<()> {
        let (word_get, word_set) = ("get ", "set ");
        // Target.
        match d.key {
            DecKey::Private(a) => self.emit_private_access(d.kind, a, pos)?,
            _ if d.is_static => self.code().pick(1),
            _ => self.code().dup(),
        }
        // Key.
        match d.key {
            DecKey::Atom(a) | DecKey::Private(a) => self.code().push_atom(a),
            DecKey::Computed(v) => {
                let r = self.name_ref(v, pos);
                self.code().name_get(r);
            }
        }
        // Value.
        let mut hidden = Atom::NULL;
        let mut hidden_set = Atom::NULL;
        match (d.key, d.kind) {
            (DecKey::Private(a), DEC_METHOD) => hidden = self.private_closure_atom("", a)?,
            (DecKey::Private(a), DEC_GETTER) => hidden = self.private_closure_atom(word_get, a)?,
            (DecKey::Private(a), DEC_SETTER) => hidden = self.private_closure_atom(word_set, a)?,
            (DecKey::Private(a), DEC_ACCESSOR) => {
                hidden = self.private_closure_atom(word_get, a)?;
                hidden_set = self.private_closure_atom(word_set, a)?;
            }
            _ => {}
        }
        if hidden.is_null() {
            self.code().undefined();
        } else if hidden_set.is_null() {
            let r = self.name_ref(hidden, pos);
            self.code().name_get(r);
        } else {
            self.code().object();
            for (h, w) in [(hidden, "get"), (hidden_set, "set")] {
                let r = self.name_ref(h, pos);
                self.code().name_get(r);
                let a = self.heap.atom_str(w)?;
                self.code().define_field(a);
            }
        }
        let r = self.name_ref(d.decs, pos);
        self.code().name_get(r);
        let r = self.name_ref(state, pos);
        self.code().name_get(r);
        let mut flags = d.kind;
        if d.is_static {
            flags |= DEC_STATIC;
        }
        if matches!(d.key, DecKey::Private(_)) {
            flags |= DEC_PRIVATE;
        }
        self.code().push_i8(flags as i8);
        self.mark(pos);
        self.code().call_intrinsic(intrinsic("ApplyDecorators"), 6);
        // Result.
        if d.kind == DEC_FIELD || d.kind == DEC_ACCESSOR {
            if !hidden_set.is_null() {
                for (h, slot) in [(hidden, REC_GET), (hidden_set, REC_SET)] {
                    self.code().dup();
                    self.code().push_i8(slot);
                    self.code().call_intrinsic(intrinsic("RecordGet"), 2);
                    let r = self.init_ref(h, pos);
                    self.code().name_init(r);
                }
            }
            let r = self.init_ref(d.rec, pos);
            self.code().name_init(r);
        } else if !hidden.is_null() {
            let r = self.init_ref(hidden, pos);
            self.code().name_init(r);
        } else {
            self.code().drop();
        }
        Ok(())
    }

    /// With the class on the stack (it stays): applies the class decorators (in the
    /// hidden binding `decs`, if any), leaving `class new_class`.
    pub(super) fn emit_decorate_class(
        &mut self,
        decs: Option<Atom>,
        name: Atom,
        state: Atom,
        pos: u32,
    ) -> PResult<()> {
        self.code().dup();
        if name.is_null() {
            self.code().undefined();
        } else {
            self.code().push_atom(name);
        }
        self.code().undefined();
        match decs {
            Some(d) => {
                let r = self.name_ref(d, pos);
                self.code().name_get(r);
            }
            None => self.code().undefined(),
        }
        let r = self.name_ref(state, pos);
        self.code().name_get(r);
        self.code().push_i8(DEC_CLASS as i8);
        self.mark(pos);
        self.code().call_intrinsic(intrinsic("ApplyDecorators"), 6);
        Ok(())
    }

    /// Runs the extra initializers in slot `slot` of the state with `this` = the value
    /// on the top of the stack (it stays).
    pub(super) fn emit_state_extras(&mut self, state: Atom, slot: u8, pos: u32) {
        let r = self.name_ref(state, pos);
        self.code().name_get(r);
        self.code().push_i8(slot as i8);
        self.code().pick(2);
        self.mark(pos);
        self.code()
            .call_intrinsic(intrinsic("DecoratorRunExtras"), 3);
        self.code().drop();
    }

    /// In a field initializer, the field value on the stack: runs the initializers of
    /// the element Record `rec` on it.
    pub(super) fn emit_run_inits(&mut self, rec: Atom, pos: u32) {
        let r = self.name_ref(rec, pos);
        self.code().name_get(r);
        self.emit_this(pos);
        self.mark(pos);
        self.code()
            .call_intrinsic(intrinsic("DecoratorRunInits"), 3);
    }

    /// In a field initializer, after the definition: the extra initializers of `rec`.
    pub(super) fn emit_run_rec_extras(&mut self, rec: Atom, pos: u32) {
        let r = self.name_ref(rec, pos);
        self.code().name_get(r);
        self.code().push_i8(REC_EXTRAS);
        self.emit_this(pos);
        self.mark(pos);
        self.code()
            .call_intrinsic(intrinsic("DecoratorRunExtras"), 3);
        self.code().drop();
    }

    /// In the instance field initializer, after the private methods: the instance
    /// method extra initializers.
    pub(super) fn emit_instance_extras(&mut self, state: Atom, pos: u32) {
        let r = self.name_ref(state, pos);
        self.code().name_get(r);
        self.code().push_i8(STATE_INSTANCE_EXTRAS);
        self.emit_this(pos);
        self.mark(pos);
        self.code()
            .call_intrinsic(intrinsic("DecoratorRunExtras"), 3);
        self.code().drop();
    }
}

pub(super) const SLOT_STATIC_EXTRAS: u8 = STATE_STATIC_EXTRAS as u8;
pub(super) const SLOT_CLASS_EXTRAS: u8 = STATE_CLASS_EXTRAS as u8;
