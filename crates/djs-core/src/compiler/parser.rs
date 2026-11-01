//! The parser core: state, token helpers, errors, names, constants and declarations
//! (docs/compiler.md 2). Statements are in `stmt.rs`, expressions in `expr.rs`.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::func::{
    BODY_SCOPE, FuncId, FuncKind, FuncState, NameRef, ScopeId, ScopeKind, VarId, VarKind,
};
use super::{CompileOptions, Goal, PErr, PResult};
use crate::bytecode::{CodeBuf, Label, Phase1};
use crate::heap::{Atom, Heap, Ref};
use crate::lexer::{Kw, Lexer, Token, TokenKind};
use crate::value::Value;

/// Nesting limit for the recursive descent (docs/compiler.md 2.4). The value is pinned
/// by the `depth_limit_fits_wasm_stack` test.
pub(super) const MAX_DEPTH: u32 = 1000;

/// A break/continue target (QuickJS's `BlockEnv`, docs/compiler.md 4.3).
pub(super) struct BlockEnv {
    pub labels: Vec<Atom>,
    pub kind: BlockKind,
    /// The break target (for `Finally`: the finally block, entered with `gosub`).
    pub break_label: Label,
    pub continue_label: Option<Label>,
    /// Operand stack depth at the statement: what a jump to this env's targets
    /// unwinds to (for `Finally`: the try statement's base depth).
    pub depth: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockKind {
    Loop,
    Switch,
    /// A labeled statement that is not a loop: only `break label` targets it.
    Labeled,
    /// A `try` with a `finally` block: `break`, `continue` and `return` that leave it
    /// run the finally block first (docs/compiler.md 7.4).
    Finally,
}

/// The state of an enclosing function while a nested one is parsed.
pub(super) struct Suspended {
    pub f: FuncState,
    pub scope: ScopeId,
    pub blocks: Vec<BlockEnv>,
    pub stmt_depth: u32,
}

pub(super) struct Parser<'a, 'h> {
    pub lx: Lexer<'a>,
    pub tok: Token,
    pub prev_end: u32,
    pub src: &'a [u8],
    pub heap: &'h mut Heap,
    pub opts: &'a CompileOptions<'a>,
    /// The function being parsed.
    pub f: FuncState,
    /// Its enclosing functions, innermost last.
    pub stack: Vec<Suspended>,
    /// Finished functions by id (the unit's top level is id 0, finished last).
    pub done: Vec<Option<FuncState>>,
    /// The innermost scope of `f`.
    pub scope: ScopeId,
    pub depth: u32,
    /// Break/continue targets of `f` (they never cross functions).
    pub blocks: Vec<BlockEnv>,
    /// Operand stack depth at statement boundaries of `f` (2 inside a finally block:
    /// the value slot and the return address).
    pub stmt_depth: u32,
    pub buf8: Vec<u8>,
    pub atom_undefined: Atom,
    pub atom_this: Atom,
    pub atom_arguments: Atom,
    pub atom_new_target: Atom,
    /// The name for an anonymous function or arrow that starts at the given offset
    /// (NamedEvaluation: `let f = () => 1` names it `f`).
    pub name_hint: Option<(Atom, u32)>,
}

pub(super) fn syntax<T>(pos: u32, msg: impl Into<Cow<'static, str>>) -> PResult<T> {
    Err(PErr::Syntax {
        pos,
        msg: msg.into(),
    })
}

impl<'a, 'h> Parser<'a, 'h> {
    pub fn new(
        heap: &'h mut Heap,
        src: &'a [u8],
        opts: &'a CompileOptions<'a>,
    ) -> PResult<Parser<'a, 'h>> {
        let mut lx = Lexer::new(src)?;
        let tok = lx.next_token()?;
        let kind = match opts.goal {
            Goal::Script => FuncKind::Script,
            Goal::AsyncBody => FuncKind::AsyncBody,
        };
        let atom_undefined = heap.atom_str("undefined")?;
        let atom_this = heap.atom_str("this")?;
        let atom_arguments = heap.atom_str("arguments")?;
        // Not an identifier, so it cannot collide with a user binding.
        let atom_new_target = heap.atom_str("new.target")?;
        let mut p = Parser {
            lx,
            tok,
            prev_end: 0,
            src,
            heap,
            opts,
            f: FuncState::new(0, kind, None),
            stack: Vec::new(),
            done: alloc::vec![None],
            scope: BODY_SCOPE,
            depth: 0,
            blocks: Vec::new(),
            stmt_depth: 0,
            buf8: Vec::new(),
            atom_undefined,
            atom_this,
            atom_arguments,
            atom_new_target,
            name_hint: None,
        };
        p.f.src_end = src.len() as u32;
        if kind == FuncKind::AsyncBody {
            for name in opts.params.iter() {
                let atom = p.heap.atom_str(name)?;
                let v = p.declare_param(atom, 0)?;
                p.f.vars[v as usize].init_end = 0;
            }
            p.f.length = p.f.param_count;
        }
        Ok(p)
    }

    /// Parses the whole source and builds the templates.
    pub fn compile(mut self) -> PResult<Ref> {
        self.code().enter_scope(BODY_SCOPE as u16);
        while self.tok.kind != TokenKind::Eof {
            self.parse_statement_list_item()?;
        }
        self.code().leave_scope(BODY_SCOPE as u16);
        self.code().return_undef();
        let top = core::mem::replace(&mut self.f, FuncState::new(0, FuncKind::Script, None));
        self.done[0] = Some(top);
        let funcs = core::mem::take(&mut self.done)
            .into_iter()
            .map(|f| f.ok_or_else(|| super::resolve::internal("unfinished function")))
            .collect::<PResult<Vec<_>>>()?;
        super::resolve::finish(self.heap, self.src, self.opts, self.atom_undefined, funcs)
    }

    // ---- Functions ----------------------------------------------------------------

    /// Starts parsing a nested function: `f` becomes the new function.
    pub fn begin_function(&mut self, kind: FuncKind) -> PResult<()> {
        let id = self.done.len() as FuncId;
        self.done.push(None);
        let child = FuncState::new(id, kind, Some((self.f.id, self.scope)));
        let parent = core::mem::replace(&mut self.f, child);
        self.stack.push(Suspended {
            f: parent,
            scope: self.scope,
            blocks: core::mem::take(&mut self.blocks),
            stmt_depth: core::mem::take(&mut self.stmt_depth),
        });
        self.scope = BODY_SCOPE;
        Ok(())
    }

    /// Finishes the current function and returns to its parent. Returns the child
    /// index for `fclosure`.
    pub fn end_function(&mut self) -> PResult<u32> {
        let s = self
            .stack
            .pop()
            .ok_or_else(|| super::resolve::internal("no parent function"))?;
        let child = core::mem::replace(&mut self.f, s.f);
        self.scope = s.scope;
        self.blocks = s.blocks;
        self.stmt_depth = s.stmt_depth;
        let id = child.id;
        self.done[id as usize] = Some(child);
        self.f.children.push(id);
        Ok((self.f.children.len() - 1) as u32)
    }

    /// Declares the next parameter of the current function. Its `init_end` is set by
    /// the caller once the parameter (and its default) has been parsed.
    pub fn declare_param(&mut self, name: Atom, pos: u32) -> PResult<VarId> {
        if self.f.find_in_scope(BODY_SCOPE, name).is_some() {
            return syntax(pos, "duplicate argument names not allowed in this context");
        }
        if self.f.param_count == u16::MAX {
            return syntax(pos, "too many parameters");
        }
        let v = self.f.add_var(name, VarKind::Param, BODY_SCOPE);
        self.f.vars[v as usize].param = self.f.param_count;
        self.f.params.push(v);
        self.f.param_count += 1;
        Ok(v)
    }

    // ---- Tokens ---------------------------------------------------------------------

    pub fn next(&mut self) -> PResult<()> {
        self.prev_end = self.tok.end;
        self.tok = self.lx.next_token()?;
        Ok(())
    }

    /// The token after the current one (the lexer is `Copy`, so this is a scan on a
    /// copy). `Eof` on a lexing error: the real scan reports it.
    pub fn peek(&self) -> Token {
        let mut lx = self.lx;
        lx.next_token().unwrap_or(Token {
            kind: TokenKind::Eof,
            kw: Kw::None,
            flags: 0,
            start: self.tok.end,
            end: self.tok.end,
        })
    }

    pub fn is(&self, kind: TokenKind) -> bool {
        self.tok.kind == kind
    }

    pub fn eat(&mut self, kind: TokenKind) -> PResult<bool> {
        if self.tok.kind == kind {
            self.next()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Consumes `kind` or fails with `expecting '<text>'`.
    pub fn expect(&mut self, kind: TokenKind, text: &str) -> PResult<()> {
        if self.tok.kind == kind {
            self.next()
        } else {
            syntax(self.tok.start, format!("expecting '{text}'"))
        }
    }

    /// Automatic semicolon insertion (docs/compiler.md 3.5).
    pub fn expect_semicolon(&mut self) -> PResult<()> {
        if self.tok.kind == TokenKind::Semicolon {
            return self.next();
        }
        if matches!(self.tok.kind, TokenKind::RBrace | TokenKind::Eof) || self.tok.newline_before()
        {
            return Ok(());
        }
        syntax(self.tok.start, "expecting ';'")
    }

    /// In operator position, `>` merges with what follows (`>>`, `>=`, ...).
    pub fn operator_tok(&mut self) {
        if self.tok.kind == TokenKind::Gt {
            self.tok = self.lx.rescan_greater(self.tok);
        }
    }

    pub fn enter(&mut self) -> PResult<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return syntax(self.tok.start, "stack overflow");
        }
        Ok(())
    }

    pub fn leave(&mut self) {
        self.depth -= 1;
    }

    /// The token text for messages: the cooked name for identifiers and keywords,
    /// otherwise the source.
    pub fn tok_text(&mut self) -> String {
        if self.tok.kind == TokenKind::Ident && self.tok.has_escape() {
            let v = self.lx.ident_value(&self.tok, &mut self.buf8);
            return String::from_utf8_lossy(v).into_owned();
        }
        if self.tok.kind == TokenKind::Eof {
            return String::from("end of input");
        }
        String::from_utf8_lossy(self.lx.text(&self.tok)).into_owned()
    }

    pub fn unexpected<T>(&mut self) -> PResult<T> {
        let text = self.tok_text();
        syntax(
            self.tok.start,
            format!("unexpected token in expression: '{text}'"),
        )
    }

    pub fn unsupported<T>(&self, pos: u32, what: &str) -> PResult<T> {
        syntax(pos, format!("{what} not supported yet"))
    }

    // ---- Code ---------------------------------------------------------------------

    pub fn code(&mut self) -> &mut CodeBuf<Phase1> {
        &mut self.f.code
    }

    pub fn new_label(&mut self) -> Label {
        self.f.code.new_label()
    }

    pub fn place(&mut self, l: Label) {
        self.f.code.place_label(l);
    }

    /// Records that the next instruction comes from source offset `pos` (for runtime
    /// error positions).
    pub fn mark(&mut self, pos: u32) {
        let pc = self.f.code.pos();
        if let Some(last) = self.f.lines.last_mut() {
            if last.0 == pc {
                last.1 = pos;
                return;
            }
            if last.1 == pos {
                return;
            }
        }
        self.f.lines.push((pc, pos));
    }

    /// Removes the last instruction (lvalue conversion, docs/compiler.md 3.7).
    pub fn truncate_last(&mut self) {
        self.f.code.truncate_last();
        let pc = self.f.code.pos();
        while self.f.lines.last().is_some_and(|l| l.0 > pc) {
            self.f.lines.pop();
        }
    }

    /// A name reference at `pos` in the current scope.
    pub fn name_ref(&mut self, atom: Atom, pos: u32) -> u32 {
        self.f.refs.push(NameRef {
            atom,
            scope: self.scope,
            pos,
            init: false,
        });
        (self.f.refs.len() - 1) as u32
    }

    /// A reference that initializes a declaration (`name_init`).
    pub fn init_ref(&mut self, atom: Atom, pos: u32) -> u32 {
        let r = self.name_ref(atom, pos);
        self.f.refs[r as usize].init = true;
        r
    }

    // ---- Constants -----------------------------------------------------------------

    /// Pushes a number: small integers inline, everything else through the pool.
    pub fn emit_number(&mut self, n: f64) {
        // `trunc` is not in core; an exact round trip through i32 says the same.
        let i = n as i32;
        if i as f64 == n && !(n == 0.0 && n.is_sign_negative()) {
            if let Ok(b) = i8::try_from(i) {
                self.code().push_i8(b);
            } else {
                self.code().push_i32(i);
            }
            return;
        }
        let bits = n.to_bits();
        let idx = match self.f.num_consts.get(&bits) {
            Some(&i) => i,
            None => {
                let i = self.add_const(Value::number(n));
                self.f.num_consts.insert(bits, i);
                i
            }
        };
        self.code().push_const(idx);
    }

    pub fn add_const(&mut self, v: Value) -> u32 {
        self.f.consts.push(v);
        (self.f.consts.len() - 1) as u32
    }

    /// The constant index of a string (deduplicated per function).
    pub fn string_const(&mut self, units: &[u16]) -> PResult<u32> {
        if let Some(&i) = self.f.str_consts.get(units) {
            return Ok(i);
        }
        let s = if units.iter().all(|&u| u <= 0xFF) {
            let bytes: Vec<u8> = units.iter().map(|&u| u as u8).collect();
            self.heap.alloc_latin1(&bytes)?
        } else {
            self.heap.alloc_utf16(units)?
        };
        let i = self.add_const(Value::string(s));
        self.f.str_consts.insert(units.to_vec(), i);
        Ok(i)
    }

    /// The cooked value of the current `String` token as UTF-16.
    pub fn cooked_string(&mut self) -> Vec<u16> {
        let mut out = Vec::new();
        self.lx.cook_string(&self.tok, &mut out);
        out
    }

    /// The atom of the current identifier-name token (identifier, keyword, private
    /// name without the `#`... P0 has no private names).
    pub fn ident_atom(&mut self) -> PResult<Atom> {
        let tok = self.tok;
        let bytes = self.lx.ident_value(&tok, &mut self.buf8);
        let s = core::str::from_utf8(bytes).unwrap_or("\u{FFFD}");
        Ok(self.heap.atom_str(s)?)
    }

    /// The atom of a string given as UTF-16 (property keys).
    pub fn atom_of_units(&mut self, units: &[u16]) -> PResult<Atom> {
        let s = if units.iter().all(|&u| u <= 0xFF) {
            let bytes: Vec<u8> = units.iter().map(|&u| u as u8).collect();
            self.heap.alloc_latin1(&bytes)?
        } else {
            self.heap.alloc_utf16(units)?
        };
        Ok(self.heap.intern(s)?)
    }

    /// The atom of a number used as a property key (`{ 1.5: x }`).
    pub fn atom_of_number(&mut self, n: f64) -> PResult<Atom> {
        let s = crate::num::to_string(n);
        Ok(self.heap.atom_str(s.as_str())?)
    }

    // ---- Names and declarations ----------------------------------------------------

    /// Validates the current token as a binding identifier and returns its atom
    /// (docs/compiler.md 2.5).
    pub fn binding_name(&mut self) -> PResult<Atom> {
        let tok = self.tok;
        if tok.kind != TokenKind::Ident
            || tok.is_escaped_reserved()
            || tok.kw.is_strict_reserved()
            || (tok.kw == Kw::Await && self.f.kind == FuncKind::AsyncBody)
        {
            return syntax(tok.start, "variable name expected");
        }
        if matches!(tok.kw, Kw::Eval | Kw::Arguments) {
            return syntax(tok.start, "invalid variable name in strict mode");
        }
        self.ident_atom()
    }

    /// Declares `var name` from the current scope: it binds in the body scope.
    pub fn declare_var(&mut self, name: Atom, pos: u32) -> PResult<VarId> {
        // A `var` must not cross a scope with a lexical binding of the same name.
        let mut s = self.scope;
        loop {
            if let Some(v) = self.f.find_in_scope(s, name) {
                let kind = self.f.vars[v as usize].kind;
                // Block-level function declarations are lexical too.
                if kind.is_lexical() || (kind == VarKind::Func && s != BODY_SCOPE) {
                    return syntax(pos, "invalid redefinition of lexical identifier");
                }
            }
            match self.f.scopes[s as usize].parent {
                Some(p) => s = p,
                None => break,
            }
        }
        self.f.var_decls.push((name, self.scope));
        if let Some(v) = self.f.find_in_scope(BODY_SCOPE, name) {
            // `var` after `var`, or a parameter: the same binding.
            return Ok(v);
        }
        let v = self.f.add_var(name, VarKind::Var, BODY_SCOPE);
        self.f.vars[v as usize].global = self.f.kind == FuncKind::Script;
        Ok(v)
    }

    /// Declares a `let`/`const` in the current scope.
    pub fn declare_lexical(&mut self, name: Atom, kind: VarKind, pos: u32) -> PResult<VarId> {
        if let Some(v) = self.f.find_in_scope(self.scope, name) {
            let msg = match self.f.vars[v as usize].kind {
                // Function declarations at the function level behave like `var`.
                VarKind::Var => self.var_conflict_message(),
                VarKind::Func if self.scope == BODY_SCOPE => self.var_conflict_message(),
                VarKind::Param => "invalid redefinition of parameter name",
                _ => "invalid redefinition of lexical identifier",
            };
            return syntax(pos, msg);
        }
        let scope = self.scope;
        if self
            .f
            .var_decls
            .iter()
            .any(|&(n, d)| n == name && self.f.scope_within(d, scope))
        {
            return syntax(pos, self.var_conflict_message());
        }
        let v = self.f.add_var(name, kind, scope);
        self.f.vars[v as usize].global = self.f.kind == FuncKind::Script && scope == BODY_SCOPE;
        Ok(v)
    }

    fn var_conflict_message(&self) -> &'static str {
        match self.f.kind {
            FuncKind::Script => "invalid redefinition of global identifier",
            _ => "invalid redefinition of a variable",
        }
    }

    /// Declares a function declaration's name in the current scope (hoisted to the
    /// scope's entry, docs/compiler.md 5.5) and registers the child that initializes it.
    pub fn declare_function(&mut self, name: Atom, child: u32, pos: u32) -> PResult<()> {
        let scope = self.scope;
        let v = if scope == BODY_SCOPE {
            // Like `var` at the function level: it may share a binding with a `var`,
            // a parameter or an earlier function declaration (the last one wins).
            match self.f.find_in_scope(BODY_SCOPE, name) {
                Some(v) if self.f.vars[v as usize].kind.is_lexical() => {
                    return syntax(pos, "invalid redefinition of lexical identifier");
                }
                Some(v) => v,
                None => {
                    let v = self.f.add_var(name, VarKind::Func, BODY_SCOPE);
                    self.f.vars[v as usize].global = self.f.kind == FuncKind::Script;
                    v
                }
            }
        } else {
            // In a block: lexical (strict mode, no Annex B).
            if self.f.find_in_scope(scope, name).is_some()
                || self
                    .f
                    .var_decls
                    .iter()
                    .any(|&(n, d)| n == name && self.f.scope_within(d, scope))
            {
                return syntax(pos, "invalid redefinition of lexical identifier");
            }
            self.f.add_var(name, VarKind::Func, scope)
        };
        let var = &mut self.f.vars[v as usize];
        if var.kind == VarKind::Var {
            // `var f; function f() {}`: the function now initializes it.
            var.kind = VarKind::Func;
        }
        self.f.scopes[scope as usize].funcs.push((v, child));
        Ok(())
    }

    /// The innermost function with its own `this`/`arguments`/`new.target` (not an
    /// arrow): `None` = the current one, `Some(i)` = `stack[i]`. `Err` if none.
    fn own_this_function(&self) -> Result<Option<usize>, ()> {
        if self.f.kind.has_own_this() {
            return Ok(None);
        }
        for (i, s) in self.stack.iter().enumerate().rev() {
            if s.f.kind.has_own_this() {
                return Ok(Some(i));
            }
        }
        Err(())
    }

    /// Declares a pseudo-binding (`this`, `arguments`, `new.target`) in the function
    /// that owns it. Returns false if there is none (top-level code has a `this` but no
    /// `arguments` or `new.target`).
    fn declare_pseudo(&mut self, kind: VarKind) -> bool {
        let Ok(owner) = self.own_this_function() else {
            return false;
        };
        let atom = match kind {
            VarKind::This => self.atom_this,
            VarKind::Arguments => self.atom_arguments,
            _ => self.atom_new_target,
        };
        let f = match owner {
            None => &mut self.f,
            Some(i) => &mut self.stack[i].f,
        };
        if f.kind.is_top_level() && kind != VarKind::This {
            return false;
        }
        let existing = match kind {
            VarKind::This => f.this_var,
            VarKind::Arguments => f.arguments_var,
            _ => f.new_target_var,
        };
        if existing.is_none() {
            let v = f.add_var(atom, kind, BODY_SCOPE);
            match kind {
                VarKind::This => f.this_var = Some(v),
                VarKind::Arguments => f.arguments_var = Some(v),
                _ => f.new_target_var = Some(v),
            }
        }
        true
    }

    /// The hinted name if the function or arrow at `pos` is the hinted expression.
    pub fn take_name_hint(&mut self, pos: u32) -> Atom {
        match self.name_hint.take() {
            Some((a, p)) if p == pos => a,
            _ => Atom::NULL,
        }
    }

    /// A reference to `this` (docs/compiler.md 5.7). In arrows it resolves to the
    /// enclosing function's binding through a capture.
    pub fn this_ref(&mut self, pos: u32) -> u32 {
        self.declare_pseudo(VarKind::This);
        let atom = self.atom_this;
        self.name_ref(atom, pos)
    }

    /// Before a reference to the name `arguments`: makes sure the owning function has
    /// the binding. In top-level code `arguments` is an ordinary (global) name.
    pub fn note_arguments(&mut self) {
        self.declare_pseudo(VarKind::Arguments);
    }

    /// A reference to `new.target`, or `None` outside functions.
    pub fn new_target_ref(&mut self, pos: u32) -> Option<u32> {
        if !self.declare_pseudo(VarKind::NewTarget) {
            return None;
        }
        let atom = self.atom_new_target;
        Some(self.name_ref(atom, pos))
    }

    // ---- Scopes ----------------------------------------------------------------------

    pub fn push_scope(&mut self, kind: ScopeKind) -> PResult<ScopeId> {
        if self.f.scopes.len() > u16::MAX as usize {
            return syntax(self.tok.start, "too many nested scopes");
        }
        let s = self.f.new_scope(self.scope, kind);
        self.scope = s;
        self.f.code.enter_scope(s as u16);
        Ok(s)
    }

    pub fn pop_scope(&mut self) {
        let s = self.scope;
        self.f.code.leave_scope(s as u16);
        self.scope = self.f.scopes[s as usize].parent.unwrap_or(BODY_SCOPE);
    }
}
