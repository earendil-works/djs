//! CreateDynamicFunction (20.2.1.1.1): the source of `Function`, `AsyncFunction`,
//! `GeneratorFunction` and `AsyncGeneratorFunction` compiled as one function (0059, D1590).
//!
//! The builtin builds the spec's sourceText, `<prefix> anonymous(<P>\n) {\n<body>\n}`, and
//! this parses exactly that one function at the top of a Script whose code returns the
//! closure. The spec parses P and the body separately, so neither can close the other's
//! brackets. Two checks give the same result on the joined text: the parameter list must
//! end at the `)` the builtin inserted (`params_close`), and the source must end right
//! after the function. So `Function("){}; f(); (function(", "")` and
//! `Function("/*", "*/){")` are SyntaxErrors, as in the spec (qjs evaluates the joined
//! text and runs such code).

use super::PResult;
use super::func::{BODY_SCOPE, FuncKind};
use super::parser::{Parser, syntax};
use crate::lexer::{Kw, TokenKind};

impl Parser<'_, '_> {
    /// Parses the dynamic function source and finishes the unit, like `compile`.
    pub(super) fn compile_dynamic(
        mut self,
        flavor: (bool, bool),
        params_close: u32,
    ) -> PResult<crate::heap::layout::Ref> {
        let start = self.tok.start;
        self.code().enter_scope(BODY_SCOPE as u16);
        if flavor.0 {
            if !self.tok.is_contextual(Kw::Async) {
                return syntax(self.tok.start, "expecting 'async'");
            }
            self.next()?;
        }
        self.expect(TokenKind::Function, "function")?;
        if flavor.1 {
            self.expect(TokenKind::Star, "*")?;
        }
        // `anonymous` (inserted by the builtin) names the function but is not bound
        // inside it.
        if self.tok.kind != TokenKind::Ident {
            return syntax(self.tok.start, "function name expected");
        }
        let name = self.ident_atom()?;
        self.next()?;
        self.enter()?;
        self.begin_function(FuncKind::Normal)?;
        self.start_flavor(flavor)?;
        self.f.name = name;
        self.f.src_start = start;
        self.expect(TokenKind::LParen, "(")?;
        self.parse_params()?;
        let paren = self.prev_end - 1;
        if paren != params_close {
            return syntax(paren, "parameters end before the parameter list");
        }
        self.parse_function_body()?;
        self.f.src_end = self.prev_end;
        let child = self.end_function()?;
        self.leave();
        if !self.is(TokenKind::Eof) {
            return syntax(self.tok.start, "unexpected token after the function body");
        }
        self.code().fclosure(child);
        self.code().leave_scope(BODY_SCOPE as u16);
        self.code().return_();
        Ok(self.finish_unit()?.0)
    }
}
