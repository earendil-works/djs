//! Classes (0040, docs/compiler.md 7.9).
//!
//! A class compiles in the enclosing function, in two nested scopes: the class scope
//! with the inner class binding (in TDZ until the class is defined), and inside it the
//! body scope with the private names and the hidden bindings. The heritage is
//! evaluated in the class scope only, so it sees the outer class's private names.
//!
//! ```text
//! [heritage; dup; call_intrinsic %ClassHeritage 1 | undefined; undefined]
//! fclosure ctor               ; the constructor's child index is reserved first
//! define_class flags          ; -> ctor proto
//! <elements>                  ; methods on proto (static ones: swap; ...; swap),
//!                             ; computed field keys into hidden bindings
//! [fclosure fields; set_home_object | undefined]; name_init <fields>
//! drop; [dup; name_init C]    ; -> ctor
//! [fclosure static; set_home_object; pick 1; call 0; drop]
//! ```
//!
//! Field initializers are two functions per class (instance and static, kind
//! `FieldInit`), parked while the class body is parsed and resumed for each
//! initializer. Each starts with a jump to the end, where the private methods and
//! accessors are installed once all are known, and back: PrivateMethodOrAccessorAdd
//! comes before the fields. The instance one is the hidden binding `<fields N>` that
//! constructors call: a base constructor at entry, a derived one after `super()`.
//! Static blocks are functions of their own, called by the static initializer.

use alloc::format;
use alloc::vec::Vec;

use super::PResult;
use super::func::{FuncKind, FuncState, ScopeId, VarId, VarKind};
use super::parser::{Parser, syntax};
use crate::bytecode::Label;
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

/// A class being parsed: its private names, for the early errors.
pub(super) struct ClassCtx {
    privates: Vec<Private>,
    /// Private name references (`#x` and position) to check at the class end: each
    /// must be declared by this class or an enclosing one.
    refs: Vec<(Atom, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivKind {
    Field,
    Method,
    Getter,
    Setter,
    /// A getter and a setter.
    Accessor,
}

struct Private {
    name: Atom,
    kind: PrivKind,
    is_static: bool,
}

/// A private method or accessor to install: the private name and the hidden bindings
/// of its closures (method or getter, setter).
struct Install {
    name: Atom,
    kind: PrivKind,
    get: Option<VarId>,
    set: Option<VarId>,
}

/// A parked field initializer function.
struct Init {
    f: FuncState,
    /// Where the initializers continue after the private method installation.
    body: Label,
    /// The jump target at the end: the private method installation.
    methods: Label,
}

/// What `parse_class` collects while it parses the body.
struct ClassBuild {
    start: u32,
    name: Atom,
    derived: bool,
    scope: ScopeId,
    ctor_slot: u32,
    has_ctor: bool,
    fields_atom: Atom,
    inst: Option<Init>,
    stat: Option<Init>,
    inst_methods: Vec<Install>,
    stat_methods: Vec<Install>,
    keys: u32,
}

/// The name of an element.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Atom(Atom),
    Private(Atom),
    /// On the stack.
    Computed,
}

impl Parser<'_, '_> {
    /// A class declaration (`decl`, binding its name in the current scope) or
    /// expression (the class on the stack), with the current token at `class`.
    /// `hint` names an anonymous class expression.
    pub(super) fn parse_class(&mut self, decl: bool, hint: Atom) -> PResult<()> {
        self.enter()?;
        let start = self.tok.start;
        self.next()?;
        let mut name = Atom::NULL;
        let mut outer = None;
        if self.is(TokenKind::Ident) {
            let tok = self.tok;
            if tok.is_escaped_reserved()
                || tok.kw.is_strict_reserved()
                || tok.kw == Kw::Yield
                || (tok.kw == Kw::Await && self.await_reserved())
            {
                return syntax(tok.start, "class statement requires a name");
            }
            name = self.binding_name()?;
            self.next()?;
            if decl {
                outer = Some(self.declare_lexical(name, VarKind::Let, tok.start)?);
            }
        } else if decl {
            return syntax(self.tok.start, "class statement requires a name");
        }
        let class_scope = self.push_scope(super::func::ScopeKind::Class)?;
        let inner = if name.is_null() {
            None
        } else {
            Some(self.f.add_var(name, VarKind::Const, class_scope))
        };
        let derived = self.eat(TokenKind::Extends)?;
        if derived {
            let pos = self.tok.start;
            self.parse_lhs(false)?;
            self.code().dup();
            let id = crate::bytecode::intrinsics::intrinsic_id("ClassHeritage").expect("intrinsic");
            self.mark(pos);
            self.code().call_intrinsic(id, 1);
        } else {
            self.code().undefined();
            self.code().undefined();
        }
        let scope = self.push_scope(super::func::ScopeKind::Class)?;
        let fields_atom = self
            .heap
            .atom_str(&format!("<fields {}>", self.classes.len()))?;
        self.f.add_var(fields_atom, VarKind::Hidden, scope);
        self.classes.push(ClassCtx {
            privates: Vec::new(),
            refs: Vec::new(),
        });
        let ctor_slot = self.reserve_child();
        self.code().fclosure(ctor_slot);
        self.code().define_class(u8::from(derived));
        let mut b = ClassBuild {
            start,
            name: if name.is_null() { hint } else { name },
            derived,
            scope,
            ctor_slot,
            has_ctor: false,
            fields_atom,
            inst: None,
            stat: None,
            inst_methods: Vec::new(),
            stat_methods: Vec::new(),
            keys: 0,
        };
        self.expect(TokenKind::LBrace, "{")?;
        while !self.is(TokenKind::RBrace) {
            if self.eat(TokenKind::Semicolon)? {
                continue;
            }
            if self.is(TokenKind::Eof) {
                return syntax(self.tok.start, "expecting '}'");
            }
            self.parse_class_element(&mut b)?;
        }
        self.next()?;
        if !b.has_ctor {
            self.default_ctor(&b)?;
        }
        let end = self.prev_end;
        // The constructor's source text is the whole class.
        let ctor_id = self.f.children[b.ctor_slot as usize];
        if let Some(Some(c)) = self.done.get_mut(ctor_id as usize) {
            c.src_start = start;
            c.src_end = end;
        }

        // `ctor proto` -> the instance initializer, the inner binding, the statics.
        let methods = core::mem::take(&mut b.inst_methods);
        match b.inst.take() {
            Some(init) => {
                let child = self.finish_init(init, &methods)?;
                self.code().fclosure(child);
                self.code().set_home_object();
            }
            None => self.code().undefined(),
        }
        let r = self.init_ref(fields_atom, end);
        self.code().name_init(r);
        self.code().drop();
        if let Some(v) = inner {
            self.code().dup();
            let r = self.init_ref(name, end);
            self.code().name_init(r);
            self.f.vars[v as usize].init_end = end;
        }
        let methods = core::mem::take(&mut b.stat_methods);
        if let Some(init) = b.stat.take() {
            let child = self.finish_init(init, &methods)?;
            self.code().fclosure(child);
            self.code().set_home_object();
            self.code().pick(1);
            self.mark(start);
            self.code().call(0);
            self.code().drop();
        }
        self.pop_scope();
        self.pop_scope();
        self.check_private_refs()?;
        if let Some(v) = outer {
            let r = self.init_ref(name, start);
            self.mark(start);
            self.code().name_init(r);
            self.f.vars[v as usize].init_end = end;
        } else if name.is_null() {
            self.last_anon_fn = Some((start, end));
        }
        self.leave();
        Ok(())
    }

    /// Checks the private name references of the class just finished: those it does
    /// not declare go to the enclosing class, or are errors.
    fn check_private_refs(&mut self) -> PResult<()> {
        let ctx = self.classes.pop().expect("class context");
        for (atom, pos) in ctx.refs {
            if ctx.privates.iter().any(|p| p.name == atom) {
                continue;
            }
            match self.classes.last_mut() {
                Some(outer) => outer.refs.push((atom, pos)),
                None => {
                    let name = self.heap.atom_name(atom);
                    return syntax(pos, format!("undefined private field '{name}'"));
                }
            }
        }
        Ok(())
    }

    /// A reference to the private name at the current token (`#x`): the binding of
    /// its private symbol, checked at the end of the class.
    pub(super) fn private_ref(&mut self, pos: u32) -> PResult<u32> {
        let atom = self.ident_atom()?;
        match self.classes.last_mut() {
            Some(c) => c.refs.push((atom, pos)),
            None => {
                let name = self.heap.atom_name(atom);
                return syntax(pos, format!("undefined private field '{name}'"));
            }
        }
        Ok(self.name_ref(atom, pos))
    }

    /// One class element (not `;`).
    fn parse_class_element(&mut self, b: &mut ClassBuild) -> PResult<()> {
        let mut is_static = false;
        if self.tok.kw == Kw::Static && self.tok.kind == TokenKind::Ident && !self.tok.has_escape()
        {
            let next = self.peek();
            if next.kind == TokenKind::LBrace {
                self.next()?;
                return self.parse_static_block(b);
            }
            if !matches!(
                next.kind,
                TokenKind::LParen | TokenKind::Assign | TokenKind::Semicolon | TokenKind::RBrace
            ) {
                self.next()?;
                is_static = true;
            }
        }
        let start = self.tok.start;
        let name_start = |k: crate::lexer::Token| {
            k.kind.is_identifier_name()
                || matches!(
                    k.kind,
                    TokenKind::String
                        | TokenKind::Number
                        | TokenKind::BigInt
                        | TokenKind::LBracket
                        | TokenKind::PrivateName
                )
        };
        let mut is_async = false;
        let mut is_gen = false;
        let mut accessor = None;
        let tok = self.tok;
        if tok.kind == TokenKind::Ident && !tok.has_escape() {
            let next = self.peek();
            if tok.kw == Kw::Async
                && !next.newline_before()
                && (name_start(next) || next.kind == TokenKind::Star)
            {
                self.next()?;
                is_async = true;
            } else if matches!(tok.kw, Kw::Get | Kw::Set) && name_start(next) {
                self.next()?;
                accessor = Some(if tok.kw == Kw::Get {
                    FuncKind::Getter
                } else {
                    FuncKind::Setter
                });
            }
        }
        if accessor.is_none() && self.is(TokenKind::Star) {
            self.next()?;
            is_gen = true;
        }
        if is_static {
            // Static elements work on the constructor: `ctor proto` -> `proto ctor`.
            self.code().swap();
        }
        let key_tok = self.tok;
        let key = if key_tok.kind == TokenKind::PrivateName {
            let a = self.ident_atom()?;
            self.next()?;
            Key::Private(a)
        } else {
            match self.parse_property_key()? {
                Some(a) => Key::Atom(a),
                None => Key::Computed,
            }
        };
        let is_method = self.is(TokenKind::LParen);
        let plain = !is_async && !is_gen && accessor.is_none();
        let ident_like =
            matches!(key_tok.kind, TokenKind::String) || key_tok.kind.is_identifier_name();
        if let Key::Atom(a) = key
            && ident_like
        {
            let text = self.heap.atom_name(a);
            if is_static && text == "constructor" && !is_method {
                return syntax(self.tok.start, "invalid field name");
            }
            if !is_static && text == "constructor" {
                if !is_method {
                    return syntax(self.tok.start, "invalid field name");
                }
                if !plain {
                    return syntax(self.tok.start, "invalid method name");
                }
                if b.has_ctor {
                    return syntax(
                        self.tok.start,
                        "property constructor appears more than once",
                    );
                }
                b.has_ctor = true;
                return self.parse_ctor(b);
            }
            if is_static && text == "prototype" {
                return syntax(self.tok.start, "invalid method name");
            }
        }
        if let Key::Private(a) = key
            && self.heap.atom_name(a) == "#constructor"
        {
            return syntax(self.tok.start, "invalid method name");
        }
        if is_method {
            self.parse_class_method(b, key, start, is_static, accessor, (is_async, is_gen))?;
        } else {
            if !plain {
                return syntax(self.tok.start, "expecting '('");
            }
            self.parse_field(b, key, is_static)?;
        }
        if is_static {
            self.code().swap();
        }
        Ok(())
    }

    /// A method, getter or setter after its key (computed keys are on the stack), with
    /// the target object (prototype or constructor) under it.
    #[allow(clippy::too_many_arguments)]
    fn parse_class_method(
        &mut self,
        b: &mut ClassBuild,
        key: Key,
        start: u32,
        is_static: bool,
        accessor: Option<FuncKind>,
        flavor: (bool, bool),
    ) -> PResult<()> {
        let kind = accessor.unwrap_or(FuncKind::Method);
        let (prefix, word) = match kind {
            FuncKind::Getter => (1u8, "get "),
            FuncKind::Setter => (2, "set "),
            _ => (0, ""),
        };
        let fname = match key {
            Key::Atom(a) | Key::Private(a) if prefix == 0 => a,
            Key::Atom(a) | Key::Private(a) => {
                let n = format!("{word}{}", self.heap.atom_name(a));
                self.heap.atom_str(&n)?
            }
            Key::Computed => Atom::NULL,
        };
        if let Key::Private(a) = key
            && accessor.is_some()
        {
            // qjs reports duplicate accessors at the `(`.
            let pk = if kind == FuncKind::Getter {
                PrivKind::Getter
            } else {
                PrivKind::Setter
            };
            self.declare_private(a, pk, is_static, self.tok.start)?;
        }
        let child = self.parse_method_function(kind, fname, start, flavor)?;
        self.code().fclosure(child);
        match key {
            Key::Atom(a) => self.code().define_class_method(a, prefix),
            Key::Computed => {
                self.code().set_function_name(prefix);
                self.code().define_class_method_elem(prefix);
            }
            Key::Private(a) => {
                if accessor.is_none() {
                    // qjs reports a duplicate method after its body.
                    self.declare_private(a, PrivKind::Method, is_static, self.tok.start)?;
                }
                // The closure goes into a hidden binding until the initializer
                // installs it (its home object is the prototype or constructor).
                let hidden = format!("<{word}{}>", self.heap.atom_name(a));
                let hidden = self.heap.atom_str(&hidden)?;
                let v = self.f.add_var(hidden, VarKind::Hidden, b.scope);
                self.code().set_home_object();
                let r = self.init_ref(hidden, start);
                self.code().name_init(r);
                // The initializer function installs it.
                let has_init = if is_static {
                    b.stat.is_some()
                } else {
                    b.inst.is_some()
                };
                if !has_init {
                    let init = self.new_init(b.start)?;
                    if is_static {
                        b.stat = Some(init);
                    } else {
                        b.inst = Some(init);
                    }
                }
                let list = if is_static {
                    &mut b.stat_methods
                } else {
                    &mut b.inst_methods
                };
                match list.iter_mut().find(|m| m.name == a) {
                    Some(m) => {
                        m.kind = PrivKind::Accessor;
                        if kind == FuncKind::Getter {
                            m.get = Some(v);
                        } else {
                            m.set = Some(v);
                        }
                    }
                    None => list.push(Install {
                        name: a,
                        kind: match kind {
                            FuncKind::Getter | FuncKind::Setter => PrivKind::Accessor,
                            _ => PrivKind::Method,
                        },
                        get: (kind != FuncKind::Setter).then_some(v),
                        set: (kind == FuncKind::Setter).then_some(v),
                    }),
                }
            }
        }
        Ok(())
    }

    /// Declares private name `name`: a getter and a setter (both static or both not)
    /// may share one; anything else is a duplicate.
    fn declare_private(
        &mut self,
        name: Atom,
        kind: PrivKind,
        is_static: bool,
        pos: u32,
    ) -> PResult<()> {
        let ctx = self.classes.last_mut().expect("class context");
        if let Some(p) = ctx.privates.iter_mut().find(|p| p.name == name) {
            let pair = matches!(
                (p.kind, kind),
                (PrivKind::Getter, PrivKind::Setter) | (PrivKind::Setter, PrivKind::Getter)
            );
            if !pair || p.is_static != is_static {
                return syntax(pos, "private class field is already defined");
            }
            p.kind = PrivKind::Accessor;
            if let Some(v) = self.f.find_in_scope(self.scope, name) {
                self.f.vars[v as usize].kind = VarKind::PrivateAccessor;
            }
            return Ok(());
        }
        ctx.privates.push(Private {
            name,
            kind,
            is_static,
        });
        let vk = match kind {
            PrivKind::Field => VarKind::PrivateName,
            PrivKind::Method | PrivKind::Getter => VarKind::PrivateMethod,
            _ => VarKind::PrivateAccessor,
        };
        let scope = self.scope;
        self.f.add_var(name, vk, scope);
        Ok(())
    }

    /// A field after its key: the initializer goes into the instance or static
    /// initializer function. A computed key is converted now and kept in a hidden
    /// binding.
    fn parse_field(&mut self, b: &mut ClassBuild, key: Key, is_static: bool) -> PResult<()> {
        let pos = self.tok.start;
        let key_var = match key {
            Key::Computed => {
                let id =
                    crate::bytecode::intrinsics::intrinsic_id("ToPropertyKey").expect("intrinsic");
                self.code().call_intrinsic(id, 1);
                let atom = self.heap.atom_str(&format!("<key {}>", b.keys))?;
                b.keys += 1;
                self.f.add_var(atom, VarKind::Hidden, b.scope);
                let r = self.init_ref(atom, pos);
                self.code().name_init(r);
                Some(atom)
            }
            Key::Private(a) => {
                self.declare_private(a, PrivKind::Field, is_static, pos)?;
                None
            }
            Key::Atom(_) => None,
        };
        let init = match if is_static {
            b.stat.take()
        } else {
            b.inst.take()
        } {
            Some(i) => i,
            None => self.new_init(b.start)?,
        };
        self.resume_function(init.f);
        self.emit_this(pos);
        match key {
            Key::Atom(a) => self.code().push_atom(a),
            Key::Private(a) => {
                let r = self.name_ref(a, pos);
                self.code().name_get(r);
            }
            Key::Computed => {
                let r = self.name_ref(key_var.expect("key binding"), pos);
                self.code().name_get(r);
            }
        }
        if self.eat(TokenKind::Assign)? {
            let value_start = self.tok.start;
            if let Key::Atom(a) | Key::Private(a) = key {
                self.name_hint = Some((a, value_start));
            }
            self.parse_assign(false)?;
            if key == Key::Computed && self.last_anon_fn == Some((value_start, self.prev_end)) {
                self.code().set_function_name(0);
            }
        } else {
            self.code().undefined();
        }
        self.mark(pos);
        match key {
            Key::Private(_) => self.code().define_private_field(0),
            _ => self.code().define_class_field(),
        }
        self.code().drop();
        let f = self.park_function()?;
        let init = Init {
            f,
            body: init.body,
            methods: init.methods,
        };
        if is_static {
            b.stat = Some(init);
        } else {
            b.inst = Some(init);
        }
        self.expect_semicolon()
    }

    /// A new parked field initializer function (`FieldInit`): it starts by jumping to
    /// the private method installation, which `finish_init` emits at its end.
    fn new_init(&mut self, start: u32) -> PResult<Init> {
        self.begin_function(FuncKind::FieldInit)?;
        self.f.src_start = start;
        let body = self.new_label();
        let methods = self.new_label();
        self.code().jump(methods);
        self.place(body);
        let f = self.park_function()?;
        Ok(Init { f, body, methods })
    }

    /// Ends a field initializer function: returns, then the private method
    /// installation (jumped to at the start), then back to the initializers. Returns
    /// its child index.
    fn finish_init(&mut self, init: Init, methods: &[Install]) -> PResult<u32> {
        let pos = self.prev_end;
        self.resume_function(init.f);
        self.f.src_end = pos;
        self.code().return_undef();
        self.place(init.methods);
        for m in methods {
            self.emit_this(pos);
            let r = self.name_ref(m.name, pos);
            self.code().name_get(r);
            let names: Vec<Atom> = [m.get, m.set]
                .iter()
                .map(|v| {
                    v.map_or(Atom::NULL, |v| {
                        self.stack.last().unwrap().f.vars[v as usize].name
                    })
                })
                .collect();
            let load = |p: &mut Self, a: Atom| {
                if a.is_null() {
                    p.code().undefined();
                } else {
                    let r = p.name_ref(a, pos);
                    p.code().name_get(r);
                }
            };
            if m.kind == PrivKind::Method {
                load(self, names[0]);
                self.code().define_private_field(1);
            } else {
                load(self, names[0]);
                load(self, names[1]);
                self.code().define_private_accessor();
            }
            self.code().drop();
        }
        self.code().jump(init.body);
        self.end_function()
    }

    /// `static { ... }` after `static`: a function called by the static initializer
    /// with `this` = the class.
    fn parse_static_block(&mut self, b: &mut ClassBuild) -> PResult<()> {
        let start = self.tok.start;
        let init = match b.stat.take() {
            Some(i) => i,
            None => self.new_init(b.start)?,
        };
        self.resume_function(init.f);
        self.emit_this(start);
        self.enter()?;
        self.begin_function(FuncKind::StaticBlock)?;
        self.f.src_start = start;
        self.parse_function_body()?;
        self.f.src_end = self.prev_end;
        let child = self.end_function()?;
        self.leave();
        self.code().fclosure(child);
        self.code().set_home_object();
        self.code().swap();
        self.mark(start);
        self.code().call(0);
        self.code().drop();
        let f = self.park_function()?;
        b.stat = Some(Init {
            f,
            body: init.body,
            methods: init.methods,
        });
        Ok(())
    }

    /// `constructor(params) { body }` after the key: the reserved child.
    fn parse_ctor(&mut self, b: &mut ClassBuild) -> PResult<()> {
        self.enter()?;
        self.begin_ctor(b)?;
        self.expect(TokenKind::LParen, "(")?;
        self.parse_params()?;
        self.parse_function_body()?;
        self.emit_ctor_epilogue();
        self.f.src_end = self.prev_end;
        self.end_function_at(Some(b.ctor_slot))?;
        self.leave();
        Ok(())
    }

    /// Starts a class constructor: the `new` check, and for a base class the field
    /// initializer, which runs before the parameters (OrdinaryCallEvaluateBody after
    /// InitializeInstanceElements, 10.2.2).
    fn begin_ctor(&mut self, b: &ClassBuild) -> PResult<()> {
        let kind = if b.derived {
            FuncKind::DerivedCtor
        } else {
            FuncKind::ClassCtor
        };
        let pos = self.tok.start;
        self.begin_function(kind)?;
        self.f.name = b.name;
        self.f.fields_atom = b.fields_atom;
        self.f.src_start = b.start;
        self.mark(pos);
        self.code().check_ctor();
        if b.derived {
            let ret = self.heap.atom_str("<return>")?;
            self.f
                .add_var(ret, VarKind::Internal, super::func::BODY_SCOPE);
            let l = self.new_label();
            self.f.ctor_return = Some((l, ret));
        } else {
            self.emit_this(pos);
            self.emit_init_fields(b.fields_atom, pos);
            self.code().drop();
        }
        Ok(())
    }

    /// The default constructor: `constructor() {}`, or for a derived class
    /// `constructor(...args) { super(...args) }` (without iterating `args`).
    fn default_ctor(&mut self, b: &ClassBuild) -> PResult<()> {
        let pos = b.start;
        self.begin_ctor(b)?;
        if b.derived {
            self.declare_pseudo(VarKind::ActiveFunc);
            let active = self.atom_active;
            let r = self.name_ref(active, pos);
            self.code().name_get(r);
            self.code().get_super();
            self.code().rest(0);
            let nt = self
                .new_target_ref(pos)
                .ok_or_else(|| super::resolve::internal("no new.target"))?;
            self.code().name_get(nt);
            self.mark(pos);
            self.code().super_call_spread();
            self.emit_bind_this(pos);
            self.code().return_();
            self.f.ctor_return = None;
        } else {
            self.code().return_undef();
        }
        self.end_function_at(Some(b.ctor_slot))?;
        Ok(())
    }
}
