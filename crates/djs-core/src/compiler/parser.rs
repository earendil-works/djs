//! The parser core: state, token helpers, errors, names, constants and declarations
//! (docs/compiler.md 2). Statements are in `stmt.rs`, expressions in `expr.rs`.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::func::{BODY_SCOPE, FuncKind, FuncState, NameRef, ScopeId, ScopeKind, VarId, VarKind};
use super::{CompileOptions, Goal, PErr, PResult};
use crate::bytecode::{CodeBuf, Label, Phase1};
use crate::heap::{Atom, Heap, Ref};
use crate::lexer::{Kw, Lexer, Token, TokenKind};
use crate::value::Value;

/// Nesting limit for the recursive descent (docs/compiler.md 2.4). The value is pinned
/// by the `depth_limit_fits_wasm_stack` test.
pub(super) const MAX_DEPTH: u32 = 500;

/// A break/continue target (QuickJS's `BlockEnv`, docs/compiler.md 4.3).
pub(super) struct BlockEnv {
    pub labels: Vec<Atom>,
    pub kind: BlockKind,
    pub break_label: Label,
    pub continue_label: Option<Label>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockKind {
    Loop,
    Switch,
    /// A labeled statement that is not a loop: only `break label` targets it.
    Labeled,
}

pub(super) struct Parser<'a, 'h> {
    pub lx: Lexer<'a>,
    pub tok: Token,
    pub prev_end: u32,
    pub src: &'a [u8],
    pub heap: &'h mut Heap,
    pub opts: &'a CompileOptions<'a>,
    pub f: FuncState,
    /// The innermost scope.
    pub scope: ScopeId,
    pub depth: u32,
    pub blocks: Vec<BlockEnv>,
    pub buf8: Vec<u8>,
    pub atom_undefined: Atom,
    pub atom_this: Atom,
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
        let mut p = Parser {
            lx,
            tok,
            prev_end: 0,
            src,
            heap,
            opts,
            f: FuncState::new(kind),
            scope: BODY_SCOPE,
            depth: 0,
            blocks: Vec::new(),
            buf8: Vec::new(),
            atom_undefined,
            atom_this,
        };
        if kind == FuncKind::AsyncBody {
            for (i, name) in opts.params.iter().enumerate() {
                let atom = p.heap.atom_str(name)?;
                if p.f.find_in_scope(BODY_SCOPE, atom).is_none() {
                    let v = p.f.add_var(atom, VarKind::Param, BODY_SCOPE);
                    p.f.vars[v as usize].param = i as u16;
                }
            }
            p.f.param_count = opts.params.len() as u16;
        }
        Ok(p)
    }

    /// Parses the whole source and builds the template.
    pub fn compile(mut self) -> PResult<Ref> {
        self.code().enter_scope(BODY_SCOPE as u16);
        while self.tok.kind != TokenKind::Eof {
            self.parse_statement_list_item()?;
        }
        self.code().leave_scope(BODY_SCOPE as u16);
        self.code().return_undef();
        super::resolve::finish(self)
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
            if let Some(v) = self.f.find_in_scope(s, name)
                && self.f.vars[v as usize].kind.is_lexical()
            {
                return syntax(pos, "invalid redefinition of lexical identifier");
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
                VarKind::Var => self.var_conflict_message(),
                VarKind::Param => "invalid redefinition of a variable",
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
            FuncKind::AsyncBody => "invalid redefinition of a variable",
        }
    }

    /// The `this` pseudo-binding of the current function, created on first use.
    pub fn this_ref(&mut self, pos: u32) -> u32 {
        if self.f.this_var.is_none() {
            let v = self.f.add_var(self.atom_this, VarKind::This, BODY_SCOPE);
            self.f.this_var = Some(v);
        }
        // Resolution looks `this` up by name from the body scope.
        self.f.refs.push(NameRef {
            atom: self.atom_this,
            scope: BODY_SCOPE,
            pos,
            init: false,
        });
        (self.f.refs.len() - 1) as u32
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
