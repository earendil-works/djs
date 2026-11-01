//! Module syntax for `strip_types` only (0048): the TypeScript corpus of the
//! strip-types oracle is modules, so `import` and `export` declarations parse in that
//! mode (`Parser::strip`) and compile to nothing meaningful; djs itself has no modules.
//! Type-only imports and exports (`import type`, `export type`, `type` specifiers) are
//! erased like ts-blank-space erases them.

use super::PResult;
use super::parser::{Parser, syntax};
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

impl Parser<'_, '_> {
    /// At `import` or `export`: a declaration (not `import(...)` or `import.meta`)?
    pub(super) fn strip_at_module_item(&self) -> bool {
        !(self.is(TokenKind::Import)
            && matches!(self.peek().kind, TokenKind::LParen | TokenKind::Dot))
    }

    pub(super) fn strip_module_item(&mut self) -> PResult<()> {
        let start = self.tok.start;
        let type_only = if self.is(TokenKind::Import) {
            self.strip_import(start)?
        } else {
            self.strip_export(start)?
        };
        if !type_only {
            self.ts_note_stmt_end();
        }
        self.ts_type_only = type_only;
        Ok(())
    }

    /// A module specifier string.
    fn strip_from(&mut self) -> PResult<()> {
        if !self.tok.is_contextual(Kw::From) {
            return syntax(self.tok.start, "expecting 'from'");
        }
        self.next()?;
        self.strip_module_name()
    }

    fn strip_module_name(&mut self) -> PResult<()> {
        if !self.is(TokenKind::String) {
            return syntax(self.tok.start, "expecting string");
        }
        self.next()?;
        // Import attributes: `with { type: "json" }`.
        if (self.is(TokenKind::With)
            || (self.lx.text(&self.tok) == b"assert" && self.is(TokenKind::Ident)))
            && !self.tok.newline_before()
        {
            self.next()?;
            if !self.is(TokenKind::LBrace) {
                return syntax(self.tok.start, "expecting '{'");
            }
            self.ts_skip_balanced()?;
        }
        Ok(())
    }

    /// A name in an import or export list: an identifier name or a string.
    fn strip_module_export_name(&mut self) -> PResult<()> {
        if !self.tok.kind.is_identifier_name() && !self.is(TokenKind::String) {
            return syntax(self.tok.start, "identifier expected");
        }
        self.next()
    }

    /// `{ a, b as c, type T }` of an import or export: `type` specifiers are erased with
    /// their comma.
    fn strip_specifiers(&mut self) -> PResult<()> {
        self.expect(TokenKind::LBrace, "{")?;
        while !self.is(TokenKind::RBrace) {
            let start = self.tok.start;
            // `type` is a modifier before a name, also before one named `as`
            // (`{ type as }`), but not in `{ type as x }`.
            let is_type = self.tok.is_contextual(Kw::Type) && {
                let mut lx = self.lx;
                let n = lx.next_token().ok();
                let n2 = lx.next_token().ok();
                match n {
                    Some(n) if n.is_contextual(Kw::As) => {
                        n2.is_some_and(|t| matches!(t.kind, TokenKind::Comma | TokenKind::RBrace))
                    }
                    Some(n) => n.kind.is_identifier_name() || n.kind == TokenKind::String,
                    None => false,
                }
            };
            if is_type {
                self.next()?;
            }
            self.strip_module_export_name()?;
            if self.tok.is_contextual(Kw::As) {
                self.next()?;
                self.strip_module_export_name()?;
            }
            let more = self.eat(TokenKind::Comma)?;
            if is_type {
                self.ts_erase(start);
            }
            if !more {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "}")
    }

    /// An import declaration at `import`. Returns whether it is type-only (erased).
    fn strip_import(&mut self, start: u32) -> PResult<bool> {
        self.next()?;
        if self.is(TokenKind::String) {
            self.strip_module_name()?;
            self.expect_semicolon()?;
            return Ok(false);
        }
        // `import type X from` (not `import type from "m"`, the default import `type`);
        // `import defer * as ns from` likewise.
        let modifier = |p: &Self| {
            let mut lx = p.lx;
            let n = lx.next_token().ok();
            let n2 = lx.next_token().ok();
            match n {
                Some(n) if matches!(n.kind, TokenKind::LBrace | TokenKind::Star) => true,
                Some(n) if n.is_contextual(Kw::From) => {
                    n2.is_some_and(|t| t.is_contextual(Kw::From))
                }
                Some(n) => n.kind == TokenKind::Ident,
                None => false,
            }
        };
        let type_only = self.tok.is_contextual(Kw::Type) && modifier(self);
        if type_only || (self.lx.text(&self.tok) == b"defer" && modifier(self)) {
            self.next()?;
        }
        if self.is(TokenKind::Ident) {
            self.next()?;
            if !self.eat(TokenKind::Comma)? {
                self.strip_from()?;
                return self.strip_import_end(start, type_only);
            }
        }
        if self.is(TokenKind::Star) {
            self.next()?;
            if !self.tok.is_contextual(Kw::As) {
                return syntax(self.tok.start, "expecting 'as'");
            }
            self.next()?;
            self.strip_module_export_name()?;
        } else {
            self.strip_specifiers()?;
        }
        self.strip_from()?;
        self.strip_import_end(start, type_only)
    }

    fn strip_import_end(&mut self, start: u32, type_only: bool) -> PResult<bool> {
        self.expect_semicolon()?;
        if type_only {
            self.ts_erase_stmt(start);
        }
        Ok(type_only)
    }

    /// An export declaration at `export`. Returns whether it is type-only (erased).
    fn strip_export(&mut self, start: u32) -> PResult<bool> {
        self.next()?;
        let tok = self.tok;
        match tok.kind {
            TokenKind::Star => {
                self.next()?;
                if self.tok.is_contextual(Kw::As) {
                    self.next()?;
                    self.strip_module_export_name()?;
                }
                self.strip_from()?;
                self.expect_semicolon()?;
                return Ok(false);
            }
            TokenKind::LBrace => {
                self.strip_specifiers()?;
                if self.tok.is_contextual(Kw::From) {
                    self.strip_from()?;
                }
                self.expect_semicolon()?;
                return Ok(false);
            }
            TokenKind::Default => {
                self.next()?;
                return self.strip_export_default(start);
            }
            TokenKind::Assign => {
                return syntax(tok.start, "'export =' is not supported");
            }
            TokenKind::Ident
                if tok.is_contextual(Kw::Type)
                    && matches!(self.peek().kind, TokenKind::LBrace | TokenKind::Star) =>
            {
                // `export type { T } [from "m"]`, `export type * from "m"`.
                self.next()?;
                if self.is(TokenKind::Star) {
                    self.next()?;
                    if self.tok.is_contextual(Kw::As) {
                        self.next()?;
                        self.strip_module_export_name()?;
                    }
                    self.strip_from()?;
                } else {
                    self.strip_specifiers()?;
                    if self.tok.is_contextual(Kw::From) {
                        self.strip_from()?;
                    }
                }
                self.expect_semicolon()?;
                self.ts_erase_stmt(start);
                return Ok(true);
            }
            TokenKind::Ident if tok.is_contextual(Kw::As) => {
                // `export as namespace N;` (declaration files): kept, like ts-blank-space.
                self.next()?;
                self.next()?;
                self.strip_module_export_name()?;
                self.expect_semicolon()?;
                return Ok(false);
            }
            _ => {}
        }
        // `export <declaration>`: a type-only one is erased from the `export`.
        let mark = self.erased.as_ref().map_or(0, |e| e.len());
        self.parse_statement_list_item()?;
        let type_only = self.ts_type_only;
        if type_only && let Some(e) = &mut self.erased {
            for r in &mut e[mark..] {
                if r.0 == tok.start {
                    r.0 = start;
                    break;
                }
            }
        }
        Ok(type_only)
    }

    /// `export default ...` after `default`.
    fn strip_export_default(&mut self, start: u32) -> PResult<bool> {
        let tok = self.tok;
        if tok.is_contextual(Kw::Interface) && self.peek().kind == TokenKind::Ident {
            self.ts_interface()?;
            self.ts_erase_stmt(start);
            return Ok(true);
        }
        if tok.is_contextual(Kw::Abstract) && self.peek().kind == TokenKind::Class {
            self.next()?;
            self.ts_erase(tok.start);
        }
        match self.tok.kind {
            TokenKind::Function => {
                if self.ts_lookahead(|p| p.ts_skip_signature()) {
                    // An overload signature of the default export.
                    let s = self.tok.start;
                    self.ts_statement()?;
                    if let Some(e) = &mut self.erased
                        && let Some(r) = e.iter_mut().rev().find(|r| r.0 == s)
                    {
                        r.0 = start;
                    }
                    return Ok(true);
                }
                let s = self.tok.start;
                self.parse_function_expression(Atom::NULL, s, false)?;
                self.code().drop();
            }
            TokenKind::Ident if self.at_async_function() => {
                let s = self.tok.start;
                self.next()?;
                self.parse_function_expression(Atom::NULL, s, true)?;
                self.code().drop();
            }
            TokenKind::Class => {
                self.parse_class(false, Atom::NULL)?;
                self.code().drop();
            }
            _ => {
                self.parse_assign(false)?;
                self.code().drop();
                self.expect_semicolon()?;
            }
        }
        Ok(false)
    }
}
