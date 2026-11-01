//! Functions: declarations, expressions, arrows, parameters and bodies
//! (docs/compiler.md 5.5, 5.7, 6.3).

use super::PResult;
use super::expr::Expr;
use super::func::{BODY_SCOPE, FuncKind, ScopeKind, VarKind};
use super::parser::{Parser, syntax};
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

impl Parser<'_, '_> {
    /// A binding name in a function context: parameters and function names have
    /// their own QuickJS messages for `eval`/`arguments`.
    fn function_binding_name(&mut self, restricted_msg: &'static str) -> PResult<Atom> {
        if matches!(self.tok.kw, Kw::Eval | Kw::Arguments) && self.tok.kind == TokenKind::Ident {
            return syntax(self.tok.start, restricted_msg);
        }
        self.binding_name()
    }

    /// `function name(params) { body }` as a declaration (hoisted).
    pub(super) fn parse_function_declaration(&mut self) -> PResult<()> {
        let start = self.tok.start;
        self.next()?;
        if self.is(TokenKind::Star) {
            return self.unsupported(start, "generators are");
        }
        if !self.is(TokenKind::Ident) {
            return syntax(self.tok.start, "function name expected");
        }
        let pos = self.tok.start;
        let name = self.function_binding_name("invalid function name in strict code")?;
        self.next()?;
        let child = self.parse_function_rest(FuncKind::Normal, name, start, false)?;
        self.declare_function(name, child, pos)
    }

    /// A function expression in primary position. `hint` names an anonymous one
    /// (`let f = function () {}`).
    pub(super) fn parse_function_expression(&mut self, hint: Atom) -> PResult<Expr> {
        let start = self.tok.start;
        self.next()?;
        if self.is(TokenKind::Star) {
            return self.unsupported(start, "generators are");
        }
        let (name, own) = if self.is(TokenKind::Ident) {
            let n = self.function_binding_name("invalid function name in strict code")?;
            self.next()?;
            (n, true)
        } else {
            (hint, false)
        };
        let child = self.parse_function_rest(FuncKind::Normal, name, start, own)?;
        self.code().fclosure(child);
        Ok(Expr::Value)
    }

    /// Parameters and body after the name. With `self_binding`, the name is bound
    /// inside the function to the function itself (named function expressions).
    fn parse_function_rest(
        &mut self,
        kind: FuncKind,
        name: Atom,
        start: u32,
        self_binding: bool,
    ) -> PResult<u32> {
        self.enter()?;
        self.begin_function(kind)?;
        self.f.name = name;
        self.f.src_start = start;
        if self_binding {
            let s = self.f.new_scope(BODY_SCOPE, ScopeKind::FuncName);
            // The name scope sits outside the body: parameters and body declarations
            // shadow it.
            self.f.scopes[s as usize].parent = None;
            self.f.scopes[BODY_SCOPE as usize].parent = Some(s);
            let v = self.f.add_var(name, VarKind::FuncName, s);
            self.f.func_name_var = Some(v);
        }
        self.expect(TokenKind::LParen, "(")?;
        self.parse_params()?;
        self.parse_function_body()?;
        self.f.src_end = self.prev_end;
        let child = self.end_function()?;
        self.leave();
        Ok(child)
    }

    /// Formal parameters up to and including the `)`. Simple lists keep the
    /// parameters in their argument slots; as soon as a default or a rest parameter
    /// appears, every parameter becomes a local initialized in order, with TDZ between
    /// them (docs/compiler.md 5.5).
    pub(super) fn parse_params(&mut self) -> PResult<()> {
        let close = TokenKind::RParen;
        let mut seen_default = false;
        while !self.is(close) {
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                return self.unsupported(self.tok.start, "destructuring parameters are");
            }
            let rest = self.eat(TokenKind::Ellipsis)?;
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                return self.unsupported(self.tok.start, "destructuring parameters are");
            }
            if self.tok.kind != TokenKind::Ident {
                return syntax(self.tok.start, "missing formal parameter");
            }
            let pos = self.tok.start;
            let name = self.function_binding_name("invalid argument name in strict code")?;
            self.next()?;
            // qjs reports duplicates at the token after the name.
            let v = self.declare_param(name, self.tok.start)?;
            let index = self.f.vars[v as usize].param;
            let has_default = !rest && self.is(TokenKind::Assign);
            if (rest || has_default) && self.f.simple_params {
                // The list turns non-simple: the parameters so far are copied from
                // their argument slots now, in order.
                self.f.simple_params = false;
                for i in 0..index {
                    let p = self.f.params[i as usize];
                    let pname = self.f.vars[p as usize].name;
                    self.code().get_arg(i);
                    let r = self.init_ref(pname, pos);
                    self.code().name_init(r);
                }
            }
            if rest {
                self.f.rest = Some(index);
                self.code().rest(index);
            } else if has_default {
                seen_default = true;
                self.next()?;
                let l = self.new_label();
                self.code().get_arg(index);
                self.code().dup();
                self.code().is_undefined();
                self.code().jump_if_false(l);
                self.code().drop();
                self.name_hint = Some((name, self.tok.start));
                self.parse_assign(false)?;
                self.place(l);
            } else if !self.f.simple_params {
                self.code().get_arg(index);
            }
            if !self.f.simple_params {
                let r = self.init_ref(name, pos);
                self.code().name_init(r);
            }
            self.f.vars[v as usize].init_end = self.prev_end;
            if !rest && !seen_default {
                self.f.length += 1;
            }
            if rest {
                break;
            }
            if !self.eat(TokenKind::Comma)? {
                break;
            }
        }
        self.expect(close, ")")
    }

    /// `{ body }` of a function: the directive prologue, then the statements.
    pub(super) fn parse_function_body(&mut self) -> PResult<()> {
        self.expect(TokenKind::LBrace, "{")?;
        self.code().enter_scope(BODY_SCOPE as u16);
        // Directive prologue: everything is strict already, but `"use strict"` with
        // non-simple parameters is an error.
        let mut directives = true;
        while !self.is(TokenKind::RBrace) {
            if self.is(TokenKind::Eof) {
                return syntax(self.tok.start, "expecting '}'");
            }
            if directives {
                if self.is(TokenKind::String) {
                    let tok = self.tok;
                    let next = self.peek();
                    let ends = matches!(
                        next.kind,
                        TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
                    ) || next.newline_before();
                    if ends
                        && !tok.has_escape()
                        && self.lx.string_content(&tok) == b"use strict"
                        && !self.f.simple_params
                    {
                        return syntax(
                            tok.start,
                            "\"use strict\" not allowed in function with default or destructuring parameter",
                        );
                    }
                    if !ends {
                        directives = false;
                    }
                } else {
                    directives = false;
                }
            }
            self.parse_statement_list_item()?;
        }
        self.next()?;
        self.code().leave_scope(BODY_SCOPE as u16);
        self.code().return_undef();
        Ok(())
    }

    /// An arrow function, with the current token at its parameters (an identifier or
    /// `(`). The caller checked that `=>` follows.
    pub(super) fn parse_arrow(&mut self, no_in: bool, hint: Atom) -> PResult<Expr> {
        let start = self.tok.start;
        self.enter()?;
        self.begin_function(FuncKind::Arrow)?;
        self.f.name = hint;
        self.f.src_start = start;
        if self.is(TokenKind::Ident) {
            let name = self.function_binding_name("invalid argument name in strict code")?;
            self.next()?;
            let v = self.declare_param(name, self.tok.start)?;
            self.f.vars[v as usize].init_end = self.prev_end;
            self.f.length = 1;
        } else {
            self.expect(TokenKind::LParen, "(")?;
            self.parse_params()?;
        }
        if !self.is(TokenKind::Arrow) || self.tok.newline_before() {
            return self.unexpected();
        }
        self.next()?;
        if self.is(TokenKind::LBrace) {
            self.parse_function_body()?;
        } else {
            self.code().enter_scope(BODY_SCOPE as u16);
            self.parse_assign(no_in)?;
            self.code().return_();
            self.code().leave_scope(BODY_SCOPE as u16);
        }
        self.f.src_end = self.prev_end;
        let child = self.end_function()?;
        self.leave();
        self.code().fclosure(child);
        Ok(Expr::Value)
    }
}
