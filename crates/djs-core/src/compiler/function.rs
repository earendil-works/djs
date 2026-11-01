//! Functions: declarations, expressions, arrows, parameters and bodies
//! (docs/compiler.md 5.5, 5.7, 6.3).

use super::PResult;
use super::expr::Expr;
use super::func::{BODY_SCOPE, FuncKind, ScopeKind, VarKind};
use super::parser::{Parser, syntax};
use super::pattern::Binding;
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
        if !own {
            self.last_anon_fn = Some((start, self.prev_end));
        }
        Ok(Expr::Value)
    }

    /// A method, getter or setter of an object literal: parameters and body after the
    /// key. Getters take no parameters, setters exactly one.
    pub(super) fn parse_method_function(
        &mut self,
        kind: FuncKind,
        name: Atom,
        start: u32,
    ) -> PResult<u32> {
        self.enter()?;
        self.begin_function(kind)?;
        self.f.name = name;
        self.f.src_start = start;
        self.expect(TokenKind::LParen, "(")?;
        self.parse_params()?;
        // qjs reports a wrong accessor arity at the `)`.
        let paren = self.prev_end - 1;
        let ok = match kind {
            FuncKind::Getter => self.f.param_count == 0,
            FuncKind::Setter => self.f.param_count == 1 && self.f.rest.is_none(),
            _ => true,
        };
        if !ok {
            return syntax(paren, "invalid number of arguments for getter or setter");
        }
        self.parse_function_body()?;
        self.f.src_end = self.prev_end;
        let child = self.end_function()?;
        self.leave();
        Ok(child)
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
            let rest = self.eat(TokenKind::Ellipsis)?;
            if matches!(self.tok.kind, TokenKind::LBrace | TokenKind::LBracket) {
                self.parse_pattern_param(rest, &mut seen_default)?;
                if rest {
                    break;
                }
                if !self.eat(TokenKind::Comma)? {
                    break;
                }
                continue;
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
            if rest || has_default {
                self.make_params_non_simple(index, pos);
            }
            if rest {
                self.f.rest = Some(index);
                self.code().rest(index);
            } else if has_default {
                seen_default = true;
                self.next()?;
                self.code().get_arg(index);
                self.name_hint = Some((name, self.tok.start));
                self.parse_param_default()?;
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

    /// The list turns non-simple at parameter `index`: the parameters before it are
    /// copied from their argument slots now, in order.
    fn make_params_non_simple(&mut self, index: u16, pos: u32) {
        if !self.f.simple_params {
            return;
        }
        self.f.simple_params = false;
        for i in 0..index {
            let p = self.f.params[i as usize];
            let pname = self.f.vars[p as usize].name;
            self.code().get_arg(i);
            let r = self.init_ref(pname, pos);
            self.code().name_init(r);
        }
    }

    /// `= default` of a parameter whose argument is on the stack.
    fn parse_param_default(&mut self) -> PResult<()> {
        let l = self.new_label();
        self.code().dup();
        self.code().is_undefined();
        self.code().jump_if_false(l);
        self.code().drop();
        self.parse_assign(false)?;
        self.place(l);
        Ok(())
    }

    /// A destructuring parameter (`{a, b}`, `[x] = []`, `...[y]`): an argument slot
    /// without a binding of its own; the pattern binds parameters.
    fn parse_pattern_param(&mut self, rest: bool, seen_default: &mut bool) -> PResult<()> {
        if self.f.param_count == u16::MAX {
            return syntax(self.tok.start, "too many parameters");
        }
        let index = self.f.param_count;
        self.f.param_count += 1;
        let pos = self.tok.start;
        self.make_params_non_simple(index, pos);
        let mark = self.pattern_vars.len();
        if rest {
            self.f.rest = Some(index);
            self.code().rest(index);
            self.parse_pattern(Binding::Param)?;
        } else {
            let cp = self.skip_pattern(Binding::Param)?;
            self.code().get_arg(index);
            if self.eat(TokenKind::Assign)? {
                *seen_default = true;
                self.parse_param_default()?;
            }
            let end = self.checkpoint();
            self.rewind(cp);
            self.parse_pattern(Binding::Param)?;
            self.rewind(end);
            if !*seen_default {
                self.f.length += 1;
            }
        }
        let end = self.prev_end;
        self.end_pattern_bindings(mark, end);
        Ok(())
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
        self.last_anon_fn = Some((start, self.prev_end));
        Ok(Expr::Value)
    }
}
