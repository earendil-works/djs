//! Expressions (docs/compiler.md 2.3, 3.7, 4.5).

use super::PResult;
use super::func::FuncKind;
use super::parser::{Parser, syntax};
use crate::bytecode::{CodeBuf, Label, Phase1};
use crate::heap::Atom;
use crate::lexer::{Kw, TokenKind};

/// What an expression left on the stack, as far as the parser needs to know: whether
/// its last instruction loads a reference that an assignment, a call or `delete` can
/// turn into something else (docs/compiler.md 3.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Expr {
    /// A plain value.
    Value,
    /// `name_get r` was the last instruction. `restricted`: `eval` or `arguments`,
    /// which strict code cannot assign.
    Name {
        r: u32,
        pos: u32,
        restricted: bool,
    },
    /// `get_field atom` was the last instruction (`pos`: the `.` or `[`).
    Field {
        atom: Atom,
        pos: u32,
    },
    /// `get_elem` was the last instruction.
    Elem {
        pos: u32,
    },
    /// A string literal (`push_const idx` was the last instruction).
    Str {
        idx: u32,
    },
    ObjectLit,
    ArrayLit,
    /// An optional chain: never an assignment target.
    OptChain,
}

/// How call arguments ended up on the stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Args {
    Count(u16),
    /// One array (there was a spread).
    Array,
}

/// An assignment target, after lvalue conversion removed the load.
#[derive(Clone, Copy, Debug)]
pub(super) enum LValue {
    Name {
        r: u32,
        pos: u32,
    },
    /// The object is on the stack.
    Field {
        atom: Atom,
        pos: u32,
    },
    /// The object and key are on the stack.
    Elem {
        pos: u32,
    },
}

#[derive(Clone, Copy)]
enum BinOp {
    Nullish,
    Or,
    And,
    Pow,
    In,
    Simple(fn(&mut CodeBuf<Phase1>)),
}

/// `??` / `||` `&&` seen in an unparenthesized chain (they cannot be mixed).
const MIX_NULLISH: u8 = 1;
const MIX_ANDOR: u8 = 2;

fn binop(kind: TokenKind) -> Option<(u8, BinOp)> {
    use BinOp::Simple as S;
    use TokenKind::*;
    type B = CodeBuf<Phase1>;
    Some(match kind {
        QuestionQuestion => (1, BinOp::Nullish),
        PipePipe => (2, BinOp::Or),
        AmpAmp => (3, BinOp::And),
        Pipe => (4, S(B::bor)),
        Caret => (5, S(B::bxor)),
        Amp => (6, S(B::band)),
        Eq => (7, S(B::eq)),
        Ne => (7, S(B::ne)),
        StrictEq => (7, S(B::strict_eq)),
        StrictNe => (7, S(B::strict_ne)),
        Lt => (8, S(B::lt)),
        Gt => (8, S(B::gt)),
        Le => (8, S(B::le)),
        Ge => (8, S(B::ge)),
        Instanceof => (8, S(B::instanceof)),
        In => (8, BinOp::In),
        Shl => (9, S(B::shl)),
        Shr => (9, S(B::sar)),
        UShr => (9, S(B::shr)),
        Plus => (10, S(B::add)),
        Minus => (10, S(B::sub)),
        Star => (11, S(B::mul)),
        Slash => (11, S(B::div)),
        Percent => (11, S(B::mod_)),
        StarStar => (12, BinOp::Pow),
        _ => return None,
    })
}

/// The binary operator of a compound assignment.
fn compound_op(kind: TokenKind) -> Option<fn(&mut CodeBuf<Phase1>)> {
    use TokenKind::*;
    type B = CodeBuf<Phase1>;
    Some(match kind {
        PlusAssign => B::add,
        MinusAssign => B::sub,
        StarAssign => B::mul,
        SlashAssign => B::div,
        PercentAssign => B::mod_,
        StarStarAssign => B::pow,
        ShlAssign => B::shl,
        ShrAssign => B::sar,
        UShrAssign => B::shr,
        AmpAssign => B::band,
        PipeAssign => B::bor,
        CaretAssign => B::bxor,
        _ => return None,
    })
}

impl Parser<'_, '_> {
    /// Expression: comma lists.
    pub(super) fn parse_expr(&mut self, no_in: bool) -> PResult<Expr> {
        let mut e = self.parse_assign(no_in)?;
        while self.is(TokenKind::Comma) {
            self.next()?;
            self.code().drop();
            self.parse_assign(no_in)?;
            e = Expr::Value;
        }
        Ok(e)
    }

    /// AssignmentExpression.
    pub(super) fn parse_assign(&mut self, no_in: bool) -> PResult<Expr> {
        self.enter()?;
        let r = self.parse_assign_inner(no_in);
        self.leave();
        r
    }

    fn parse_assign_inner(&mut self, no_in: bool) -> PResult<Expr> {
        if let Some(arrow) = self.try_arrow(no_in)? {
            return Ok(arrow);
        }
        let lhs = self.parse_conditional(no_in)?;
        let op = self.tok;
        if op.kind == TokenKind::Assign {
            let lv = self.lvalue(lhs, "invalid assignment left-hand side")?;
            self.next()?;
            if let LValue::Name { r, .. } = lv {
                // NamedEvaluation: `f = function () {}` names the function `f`.
                self.name_hint = Some((self.f.refs[r as usize].atom, self.tok.start));
            }
            self.parse_assign(no_in)?;
            self.emit_store(lv, true);
            return Ok(Expr::Value);
        }
        if let Some(emit_op) = compound_op(op.kind) {
            let lv = self.lvalue(lhs, "invalid assignment left-hand side")?;
            self.emit_load(lv);
            self.next()?;
            self.parse_assign(no_in)?;
            self.mark(op.start);
            emit_op(self.code());
            self.emit_store(lv, true);
            return Ok(Expr::Value);
        }
        if matches!(
            op.kind,
            TokenKind::AmpAmpAssign | TokenKind::PipePipeAssign | TokenKind::QuestionQuestionAssign
        ) {
            // Short-circuit: the store only happens when the right side is evaluated.
            let lv = self.lvalue(lhs, "invalid assignment left-hand side")?;
            self.emit_load(lv);
            self.next()?;
            let l_keep = self.new_label();
            let l_end = self.new_label();
            self.code().dup();
            match op.kind {
                TokenKind::AmpAmpAssign => self.code().jump_if_false(l_keep),
                TokenKind::PipePipeAssign => self.code().jump_if_true(l_keep),
                _ => {
                    self.code().is_nullish();
                    self.code().jump_if_false(l_keep);
                }
            }
            self.code().drop();
            self.parse_assign(no_in)?;
            self.emit_store(lv, true);
            self.code().jump(l_end);
            self.place(l_keep);
            // Keep the current value; drop the object (and key) under it.
            match lv {
                LValue::Name { .. } => {}
                LValue::Field { .. } => self.code().nip(),
                LValue::Elem { .. } => {
                    self.code().nip();
                    self.code().nip();
                }
            }
            self.place(l_end);
            return Ok(Expr::Value);
        }
        Ok(lhs)
    }

    /// Arrow functions start here (docs/compiler.md 3.1). Returns the arrow if there
    /// is one; async arrows and functions are rejected with a clear message.
    fn try_arrow(&mut self, no_in: bool) -> PResult<Option<Expr>> {
        let tok = self.tok;
        let hint = match self.name_hint {
            Some((a, p)) if p == tok.start => a,
            _ => Atom::NULL,
        };
        match tok.kind {
            TokenKind::Ident => {
                let next = self.peek();
                if next.kind == TokenKind::Arrow && !next.newline_before() {
                    self.name_hint = None;
                    return self.parse_arrow(no_in, hint).map(Some);
                }
                if tok.is_contextual(Kw::Async) && !next.newline_before() {
                    match next.kind {
                        TokenKind::Function => {
                            return self.unsupported(tok.start, "async functions are");
                        }
                        TokenKind::Ident => {
                            return self.unsupported(tok.start, "async arrow functions are");
                        }
                        TokenKind::LParen => {
                            let mut lx = self.lx;
                            if let Ok(open) = lx.next_token()
                                && super::scan::is_arrow_head(lx, open)
                            {
                                return self.unsupported(tok.start, "async arrow functions are");
                            }
                        }
                        _ => {}
                    }
                }
            }
            TokenKind::LParen if super::scan::is_arrow_head(self.lx, tok) => {
                self.name_hint = None;
                return self.parse_arrow(no_in, hint).map(Some);
            }
            _ => {}
        }
        Ok(None)
    }

    /// Lvalue conversion: removes the load that `e` ended with.
    pub(super) fn lvalue(&mut self, e: Expr, msg: &'static str) -> PResult<LValue> {
        let pos = self.tok.start;
        let lv = match e {
            Expr::Name { r, pos, restricted } => {
                if restricted {
                    return syntax(pos, "invalid lvalue in strict mode");
                }
                LValue::Name { r, pos }
            }
            Expr::Field { atom, pos } => LValue::Field { atom, pos },
            Expr::Elem { pos } => LValue::Elem { pos },
            Expr::ObjectLit | Expr::ArrayLit => {
                return self.unsupported(pos, "destructuring assignment is");
            }
            _ => return syntax(pos, msg),
        };
        self.truncate_last();
        Ok(lv)
    }

    /// Loads the current value of a target for a compound operation, keeping the target
    /// on the stack under it.
    fn emit_load(&mut self, lv: LValue) {
        match lv {
            LValue::Name { r, pos } => {
                self.mark(pos);
                self.code().name_get(r);
            }
            LValue::Field { atom, pos } => {
                self.code().dup();
                self.mark(pos);
                self.code().get_field(atom);
            }
            LValue::Elem { pos } => {
                self.code().dup2();
                self.mark(pos);
                self.code().get_elem();
            }
        }
    }

    /// Stores the value on top of the stack into the target. With `keep`, the value
    /// stays on the stack.
    fn emit_store(&mut self, lv: LValue, keep: bool) {
        match lv {
            LValue::Name { r, pos } => {
                self.mark(pos);
                if keep {
                    self.code().name_set(r);
                } else {
                    self.code().name_put(r);
                }
            }
            LValue::Field { atom, pos } => {
                if keep {
                    self.code().insert2();
                }
                self.mark(pos);
                self.code().put_field(atom);
            }
            LValue::Elem { pos } => {
                if keep {
                    self.code().insert3();
                }
                self.mark(pos);
                self.code().put_elem();
            }
        }
    }

    fn parse_conditional(&mut self, no_in: bool) -> PResult<Expr> {
        let (e, _) = self.parse_binary(1, no_in)?;
        if !self.is(TokenKind::Question) {
            return Ok(e);
        }
        self.next()?;
        let l_else = self.new_label();
        let l_end = self.new_label();
        self.code().jump_if_false(l_else);
        self.parse_assign(false)?;
        self.expect(TokenKind::Colon, ":")?;
        self.code().jump(l_end);
        self.place(l_else);
        self.parse_assign(no_in)?;
        self.place(l_end);
        Ok(Expr::Value)
    }

    /// Precedence climbing over the binary operators (docs/compiler.md 2.3). Returns
    /// the expression and which logical operators its unparenthesized chain contains.
    fn parse_binary(&mut self, min: u8, no_in: bool) -> PResult<(Expr, u8)> {
        let (mut lhs, mut lhs_unary) = self.parse_unary()?;
        let mut mix = 0u8;
        loop {
            self.operator_tok();
            let Some((prec, op)) = binop(self.tok.kind) else {
                break;
            };
            if prec < min || (matches!(op, BinOp::In) && no_in) {
                break;
            }
            let op_tok = self.tok;
            if matches!(op, BinOp::Pow) && lhs_unary {
                return syntax(
                    op_tok.start,
                    "unparenthesized unary expression can't appear on the left-hand side of '**'",
                );
            }
            self.next()?;
            let l_short = self.new_label();
            match op {
                BinOp::Nullish => {
                    self.code().dup();
                    self.code().is_nullish();
                    self.code().jump_if_false(l_short);
                    self.code().drop();
                }
                BinOp::Or => {
                    self.code().dup();
                    self.code().jump_if_true(l_short);
                    self.code().drop();
                }
                BinOp::And => {
                    self.code().dup();
                    self.code().jump_if_false(l_short);
                    self.code().drop();
                }
                _ => {}
            }
            // `**` is right-associative.
            let next_min = if matches!(op, BinOp::Pow) {
                prec
            } else {
                prec + 1
            };
            let (_, rmix) = self.parse_binary(next_min, no_in)?;
            let this = match op {
                BinOp::Nullish => MIX_NULLISH,
                BinOp::Or | BinOp::And => MIX_ANDOR,
                _ => 0,
            };
            let all = mix | rmix;
            if (this == MIX_NULLISH && all & MIX_ANDOR != 0)
                || (this == MIX_ANDOR && all & MIX_NULLISH != 0)
            {
                return syntax(op_tok.start, "cannot mix ?? with && or ||");
            }
            mix = all | this;
            match op {
                BinOp::Nullish | BinOp::Or | BinOp::And => self.place(l_short),
                BinOp::Pow => {
                    self.mark(op_tok.start);
                    self.code().pow();
                }
                BinOp::In => {
                    self.mark(op_tok.start);
                    self.code().in_();
                }
                BinOp::Simple(emit) => {
                    self.mark(op_tok.start);
                    emit(self.code());
                }
            }
            lhs = Expr::Value;
            lhs_unary = false;
        }
        Ok((lhs, mix))
    }

    /// UnaryExpression. Returns the expression and whether it was a unary operator
    /// (for the `**` rule).
    fn parse_unary(&mut self) -> PResult<(Expr, bool)> {
        self.enter()?;
        let r = self.parse_unary_inner();
        self.leave();
        r
    }

    fn parse_unary_inner(&mut self) -> PResult<(Expr, bool)> {
        let tok = self.tok;
        match tok.kind {
            TokenKind::Delete => {
                self.next()?;
                let (e, _) = self.parse_unary()?;
                match e {
                    Expr::Name { .. } => {
                        return syntax(
                            tok.start,
                            "cannot delete a direct reference in strict mode",
                        );
                    }
                    Expr::Field { atom, pos } => {
                        self.truncate_last();
                        self.code().push_atom(atom);
                        self.mark(pos);
                        self.code().delete();
                    }
                    Expr::Elem { pos } => {
                        self.truncate_last();
                        self.mark(pos);
                        self.code().delete();
                    }
                    Expr::OptChain => {
                        return self.unsupported(tok.start, "delete of an optional chain is");
                    }
                    _ => {
                        self.code().drop();
                        self.code().push_true();
                    }
                }
            }
            TokenKind::Void => {
                self.next()?;
                self.parse_unary()?;
                self.code().drop();
                self.code().undefined();
            }
            TokenKind::Typeof => {
                self.next()?;
                let (e, _) = self.parse_unary()?;
                if let Expr::Name { r, pos, .. } = e {
                    self.truncate_last();
                    self.mark(pos);
                    self.code().name_typeof(r);
                }
                self.code().typeof_();
            }
            TokenKind::Plus | TokenKind::Minus | TokenKind::Tilde | TokenKind::Bang => {
                self.next()?;
                self.parse_unary()?;
                self.mark(tok.start);
                match tok.kind {
                    TokenKind::Plus => self.code().plus(),
                    TokenKind::Minus => self.code().neg(),
                    TokenKind::Tilde => self.code().bnot(),
                    _ => self.code().not(),
                }
            }
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                self.next()?;
                let (e, _) = self.parse_unary()?;
                let lv = self.lvalue(e, "invalid increment/decrement operand")?;
                self.emit_load(lv);
                self.mark(tok.start);
                if tok.kind == TokenKind::PlusPlus {
                    self.code().inc();
                } else {
                    self.code().dec();
                }
                self.emit_store(lv, true);
                // An UpdateExpression, not a UnaryExpression: `++a ** 2` is valid.
                return Ok((Expr::Value, false));
            }
            TokenKind::Ident
                if tok.is_contextual(Kw::Await) && self.f.kind == FuncKind::AsyncBody =>
            {
                return self.unsupported(tok.start, "await is");
            }
            _ => return Ok((self.parse_postfix()?, false)),
        }
        Ok((Expr::Value, true))
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let e = self.parse_lhs()?;
        let tok = self.tok;
        if !matches!(tok.kind, TokenKind::PlusPlus | TokenKind::MinusMinus) || tok.newline_before()
        {
            return Ok(e);
        }
        let lv = self.lvalue(e, "invalid increment/decrement operand")?;
        self.next()?;
        // Leaves the old (numeric) value: `x++` is `old = +x; x = old + 1; old`.
        self.emit_load(lv);
        self.code().to_numeric();
        match lv {
            LValue::Name { .. } => self.code().dup(),
            LValue::Field { .. } => self.code().insert2(),
            LValue::Elem { .. } => self.code().insert3(),
        }
        self.mark(tok.start);
        if tok.kind == TokenKind::PlusPlus {
            self.code().inc();
        } else {
            self.code().dec();
        }
        self.emit_store(lv, false);
        Ok(Expr::Value)
    }

    /// LeftHandSideExpression: `new`, member access, calls, optional chains.
    fn parse_lhs(&mut self) -> PResult<Expr> {
        let mut e = if self.is(TokenKind::New) {
            self.parse_new()?
        } else {
            self.parse_primary()?
        };
        // Short-circuit pads of an optional chain, indexed by stack depth - 1.
        let mut pads: [Option<Label>; 2] = [None, None];
        loop {
            let tok = self.tok;
            match tok.kind {
                TokenKind::Dot => {
                    self.next()?;
                    e = self.parse_dot_name(tok.start)?;
                }
                TokenKind::LBracket => {
                    self.next()?;
                    e = self.parse_index(tok.start)?;
                }
                TokenKind::LParen => {
                    self.prepare_call(e);
                    let args = self.parse_args()?;
                    self.mark(tok.start);
                    self.emit_call(args);
                    e = Expr::Value;
                }
                TokenKind::QuestionDot => {
                    self.next()?;
                    match self.tok.kind {
                        TokenKind::LParen => {
                            // `f?.()` / `o.m?.()`: test the callee, keep `this`.
                            self.prepare_call(e);
                            let pad = *pads[1].get_or_insert_with(|| self.f.code.new_label());
                            self.code().swap();
                            self.code().dup();
                            self.code().is_nullish();
                            self.code().jump_if_true(pad);
                            self.code().swap();
                            let paren = self.tok.start;
                            let args = self.parse_args()?;
                            self.mark(paren);
                            self.emit_call(args);
                            e = Expr::Value;
                        }
                        _ => {
                            let pad = *pads[0].get_or_insert_with(|| self.f.code.new_label());
                            self.code().dup();
                            self.code().is_nullish();
                            self.code().jump_if_true(pad);
                            if self.is(TokenKind::LBracket) {
                                let pos = self.tok.start;
                                self.next()?;
                                e = self.parse_index(pos)?;
                            } else {
                                e = self.parse_dot_name(tok.start)?;
                            }
                        }
                    }
                }
                TokenKind::NoSubstTemplate | TokenKind::TemplateHead => {
                    if pads.iter().any(|p| p.is_some()) {
                        return syntax(
                            tok.start,
                            "template literal cannot appear in an optional chain",
                        );
                    }
                    return self.unsupported(tok.start, "tagged templates are");
                }
                _ => break,
            }
        }
        if pads.iter().any(|p| p.is_some()) {
            // Short-circuited: drop what the chain had pushed and produce `undefined`.
            let l_end = self.new_label();
            self.code().jump(l_end);
            if let Some(p2) = pads[1] {
                self.place(p2);
                self.code().drop();
            }
            let p1 = pads[0].unwrap_or_else(|| self.f.code.new_label());
            self.place(p1);
            self.code().drop();
            self.code().undefined();
            self.place(l_end);
            return Ok(Expr::OptChain);
        }
        Ok(e)
    }

    /// Before the arguments of a call: the callee and `this` on the stack.
    fn prepare_call(&mut self, e: Expr) {
        match e {
            Expr::Field { atom, pos } => {
                self.truncate_last();
                self.mark(pos);
                self.code().get_method(atom);
            }
            Expr::Elem { pos } => {
                self.truncate_last();
                self.mark(pos);
                self.code().get_elem_method();
            }
            _ => self.code().undefined(),
        }
    }

    /// The property name after `.` (or `?.`).
    fn parse_dot_name(&mut self, pos: u32) -> PResult<Expr> {
        if self.is(TokenKind::PrivateName) {
            return self.unsupported(self.tok.start, "private names are");
        }
        if !self.tok.kind.is_identifier_name() {
            return syntax(self.tok.start, "expecting field name");
        }
        let atom = self.ident_atom()?;
        self.next()?;
        self.mark(pos);
        self.code().get_field(atom);
        Ok(Expr::Field { atom, pos })
    }

    /// `[expr]` after the `[`. A string literal key becomes a `get_field`.
    fn parse_index(&mut self, pos: u32) -> PResult<Expr> {
        let key = self.parse_expr(false)?;
        self.expect(TokenKind::RBracket, "]")?;
        if let Expr::Str { idx } = key {
            let s = self.f.consts[idx as usize].as_string();
            if let Some(s) = s {
                self.truncate_last();
                let atom = self.heap.intern(s)?;
                self.mark(pos);
                self.code().get_field(atom);
                return Ok(Expr::Field { atom, pos });
            }
        }
        self.mark(pos);
        self.code().get_elem();
        Ok(Expr::Elem { pos })
    }

    /// `( args )`: the arguments are pushed one by one, or, from the first spread
    /// on, collected into an array (`f(a, ...b)` -> `array_from 1; array_spread`).
    fn parse_args(&mut self) -> PResult<Args> {
        self.expect(TokenKind::LParen, "(")?;
        let mut argc: u32 = 0;
        let mut array = false;
        while !self.is(TokenKind::RParen) {
            if self.is(TokenKind::Ellipsis) {
                if !array {
                    self.code().array_from(argc as u16);
                    array = true;
                }
                self.next()?;
                self.parse_assign(false)?;
                self.code().array_spread();
            } else {
                self.parse_assign(false)?;
                if array {
                    self.code().array_push();
                } else {
                    argc += 1;
                    if argc > u16::MAX as u32 {
                        return syntax(self.tok.start, "Too many call arguments");
                    }
                }
            }
            if !self.eat(TokenKind::Comma)? {
                break;
            }
        }
        self.expect(TokenKind::RParen, ")")?;
        Ok(if array {
            Args::Array
        } else {
            Args::Count(argc as u16)
        })
    }

    /// The call after `prepare_call`.
    fn emit_call(&mut self, args: Args) {
        match args {
            Args::Count(n) => self.code().call(n),
            Args::Array => self.code().call_spread(),
        }
    }

    /// `new X(args)` / `new X`.
    fn parse_new(&mut self) -> PResult<Expr> {
        let pos = self.tok.start;
        self.next()?;
        if self.is(TokenKind::Dot) {
            self.next()?;
            if !self.tok.is_contextual(Kw::Target) {
                return syntax(self.tok.start, "expecting target");
            }
            self.next()?;
            let Some(r) = self.new_target_ref(pos) else {
                return syntax(pos, "new.target only allowed within functions");
            };
            self.code().name_get(r);
            return Ok(Expr::Value);
        }
        self.enter()?;
        if self.is(TokenKind::New) {
            self.parse_new()?;
        } else {
            self.parse_primary()?;
        }
        self.leave();
        // MemberExpression: no calls before the arguments of `new`.
        loop {
            let tok = self.tok;
            match tok.kind {
                TokenKind::Dot => {
                    self.next()?;
                    self.parse_dot_name(tok.start)?;
                }
                TokenKind::LBracket => {
                    self.next()?;
                    self.parse_index(tok.start)?;
                }
                TokenKind::QuestionDot => {
                    return syntax(
                        tok.start,
                        "new keyword cannot be used with an optional chain",
                    );
                }
                TokenKind::NoSubstTemplate | TokenKind::TemplateHead => {
                    return self.unsupported(tok.start, "tagged templates are");
                }
                _ => break,
            }
        }
        let args = if self.is(TokenKind::LParen) {
            self.parse_args()?
        } else {
            Args::Count(0)
        };
        self.mark(pos);
        match args {
            Args::Count(n) => self.code().construct(n),
            Args::Array => self.code().construct_spread(),
        }
        Ok(Expr::Value)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        self.enter()?;
        let r = self.parse_primary_inner();
        self.leave();
        r
    }

    fn parse_primary_inner(&mut self) -> PResult<Expr> {
        let tok = self.tok;
        match tok.kind {
            TokenKind::Number => {
                let lit = self.lx.number_literal(&tok);
                let n = crate::num::literal_digits_value(lit.base.radix(), lit.digits);
                self.next()?;
                self.emit_number(n);
                Ok(Expr::Value)
            }
            TokenKind::BigInt => self.unsupported(tok.start, "BigInt literals are"),
            TokenKind::String => {
                let units = self.cooked_string();
                let idx = self.string_const(&units)?;
                self.next()?;
                self.code().push_const(idx);
                Ok(Expr::Str { idx })
            }
            TokenKind::NoSubstTemplate | TokenKind::TemplateHead => self.parse_template(),
            TokenKind::Slash | TokenKind::SlashAssign => {
                self.unsupported(tok.start, "regular expressions are")
            }
            TokenKind::Ident => self.parse_ident_ref(),
            TokenKind::This => {
                let r = self.this_ref(tok.start);
                self.next()?;
                self.code().name_get(r);
                Ok(Expr::Value)
            }
            TokenKind::Null => {
                self.next()?;
                self.code().null();
                Ok(Expr::Value)
            }
            TokenKind::True => {
                self.next()?;
                self.code().push_true();
                Ok(Expr::Value)
            }
            TokenKind::False => {
                self.next()?;
                self.code().push_false();
                Ok(Expr::Value)
            }
            TokenKind::LParen => {
                self.next()?;
                if self.is(TokenKind::RParen) {
                    return self.unexpected();
                }
                let e = self.parse_expr(false)?;
                self.expect(TokenKind::RParen, ")")?;
                // Parentheses keep references (`(a.b)()`, `(x) = 1`) but not
                // literals as patterns (`({a}) = 1` is an error).
                Ok(match e {
                    Expr::ObjectLit | Expr::ArrayLit | Expr::Str { .. } => Expr::Value,
                    e => e,
                })
            }
            TokenKind::LBracket => self.parse_array(),
            TokenKind::LBrace => self.parse_object(),
            TokenKind::Function => {
                let hint = self.take_name_hint(tok.start);
                self.parse_function_expression(hint)
            }
            TokenKind::Class => self.unsupported(tok.start, "classes are"),
            TokenKind::PrivateName => self.unsupported(tok.start, "private names are"),
            TokenKind::Super => syntax(tok.start, "'super' is only valid in a method"),
            TokenKind::Import => self.unsupported(tok.start, "import() is"),
            TokenKind::Percent if self.opts.privileged => self.parse_intrinsic_call(),
            _ => self.unexpected(),
        }
    }

    /// `%Name(args)` in privileged code: `call_intrinsic id, argc` (D1270).
    fn parse_intrinsic_call(&mut self) -> PResult<Expr> {
        let pos = self.tok.start;
        self.next()?;
        if !self.is(TokenKind::Ident) {
            return syntax(self.tok.start, "intrinsic name expected");
        }
        let name = self.lx.text(&self.tok);
        let name = core::str::from_utf8(name).unwrap_or("");
        let Some(id) = crate::bytecode::intrinsics::intrinsic_id(name) else {
            return syntax(self.tok.start, alloc::format!("unknown intrinsic %{name}"));
        };
        let info = crate::bytecode::intrinsics::intrinsic(id).expect("intrinsic id");
        self.next()?;
        let paren = self.tok.start;
        let argc = match self.parse_args()? {
            Args::Count(n) => n,
            Args::Array => return syntax(paren, "spread arguments in an intrinsic call"),
        };
        let ok = if info.variadic {
            argc >= u16::from(info.params)
        } else {
            argc == u16::from(info.params)
        };
        if !ok {
            return syntax(
                paren,
                alloc::format!("wrong number of arguments for %{}", info.name),
            );
        }
        self.mark(pos);
        self.code().call_intrinsic(id, argc);
        Ok(Expr::Value)
    }

    /// An identifier reference.
    fn parse_ident_ref(&mut self) -> PResult<Expr> {
        let tok = self.tok;
        if tok.kw == Kw::Yield {
            return syntax(tok.start, "unexpected 'yield' keyword");
        }
        if tok.is_escaped_reserved() || tok.kw.is_strict_reserved() {
            return self.unexpected();
        }
        let atom = self.ident_atom()?;
        self.next()?;
        if tok.kw == Kw::Arguments {
            self.note_arguments();
        }
        let r = self.name_ref(atom, tok.start);
        self.mark(tok.start);
        self.code().name_get(r);
        Ok(Expr::Name {
            r,
            pos: tok.start,
            restricted: matches!(tok.kw, Kw::Eval | Kw::Arguments),
        })
    }

    /// An untagged template literal: the cooked parts and `to_string` of each
    /// substitution, joined by one `concat`.
    fn parse_template(&mut self) -> PResult<Expr> {
        let mut parts: u32 = 0;
        let mut units = alloc::vec::Vec::new();
        loop {
            let tok = self.tok;
            units.clear();
            self.lx.cook_template(&tok, &mut units)?;
            if !units.is_empty() {
                let idx = self.string_const(&units)?;
                self.code().push_const(idx);
                parts = self.template_part(parts);
            }
            if matches!(
                tok.kind,
                TokenKind::NoSubstTemplate | TokenKind::TemplateTail
            ) {
                self.next()?;
                break;
            }
            self.next()?;
            self.parse_expr(false)?;
            self.mark(tok.end - 2);
            self.code().to_string();
            parts = self.template_part(parts);
            if !self.is(TokenKind::RBrace) {
                return syntax(self.tok.start, "expected '}' after template expression");
            }
            self.tok = self.lx.rescan_template_continuation(self.tok)?;
        }
        match parts {
            0 => {
                let idx = self.string_const(&[])?;
                self.code().push_const(idx);
            }
            1 => {}
            n => self.code().concat(n as u16),
        }
        Ok(Expr::Value)
    }

    /// Counts a pushed template part, joining early when the count would not fit.
    fn template_part(&mut self, parts: u32) -> u32 {
        let parts = parts + 1;
        if parts == u16::MAX as u32 {
            self.code().concat(u16::MAX);
            return 1;
        }
        parts
    }

    /// `[a, , ...b]`: up to `STACK_ELEMS` leading elements go on the stack for one
    /// `array_from`; holes, spreads and the rest are appended one by one.
    fn parse_array(&mut self) -> PResult<Expr> {
        const STACK_ELEMS: u16 = 256;
        self.next()?;
        let mut on_stack: u16 = 0;
        let mut appending = false;
        loop {
            match self.tok.kind {
                TokenKind::RBracket => break,
                TokenKind::Comma => {
                    self.start_appending(&mut appending, on_stack);
                    self.code().array_hole();
                    self.next()?;
                    continue;
                }
                TokenKind::Ellipsis => {
                    self.start_appending(&mut appending, on_stack);
                    self.next()?;
                    self.parse_assign(false)?;
                    self.code().array_spread();
                }
                _ => {
                    self.parse_assign(false)?;
                    if appending {
                        self.code().array_push();
                    } else {
                        on_stack += 1;
                        if on_stack == STACK_ELEMS {
                            self.start_appending(&mut appending, on_stack);
                        }
                    }
                }
            }
            if !self.eat(TokenKind::Comma)? {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "]")?;
        if !appending {
            self.code().array_from(on_stack);
        }
        Ok(Expr::ArrayLit)
    }

    fn start_appending(&mut self, appending: &mut bool, on_stack: u16) {
        if !*appending {
            self.code().array_from(on_stack);
            *appending = true;
        }
    }

    /// `{ a: 1, b, [k]: v, ...o }`. Methods and accessors need functions (P1).
    fn parse_object(&mut self) -> PResult<Expr> {
        self.next()?;
        self.code().object();
        while !self.is(TokenKind::RBrace) {
            let tok = self.tok;
            if tok.kind == TokenKind::Ellipsis {
                self.next()?;
                self.parse_assign(false)?;
                self.code().copy_data_props(0);
            } else {
                let mut computed = false;
                let mut shorthand_ok = false;
                let atom = match tok.kind {
                    TokenKind::LBracket => {
                        self.next()?;
                        self.parse_assign(false)?;
                        self.expect(TokenKind::RBracket, "]")?;
                        computed = true;
                        None
                    }
                    TokenKind::String => {
                        let units = self.cooked_string();
                        let a = self.atom_of_units(&units)?;
                        self.next()?;
                        Some(a)
                    }
                    TokenKind::Number => {
                        let lit = self.lx.number_literal(&tok);
                        let n = crate::num::literal_digits_value(lit.base.radix(), lit.digits);
                        let a = self.atom_of_number(n)?;
                        self.next()?;
                        Some(a)
                    }
                    TokenKind::Star => {
                        return self.unsupported(tok.start, "methods are");
                    }
                    TokenKind::BigInt => {
                        return self.unsupported(tok.start, "BigInt literals are");
                    }
                    k if k.is_identifier_name() => {
                        let a = self.ident_atom()?;
                        self.next()?;
                        if matches!(tok.kw, Kw::Get | Kw::Set | Kw::Async)
                            && !tok.has_escape()
                            && !matches!(
                                self.tok.kind,
                                TokenKind::Colon
                                    | TokenKind::Comma
                                    | TokenKind::RBrace
                                    | TokenKind::LParen
                                    | TokenKind::Assign
                            )
                        {
                            return self.unsupported(tok.start, "getters, setters and methods are");
                        }
                        shorthand_ok = tok.kind == TokenKind::Ident;
                        Some(a)
                    }
                    TokenKind::PrivateName => return syntax(tok.start, "invalid property name"),
                    _ => return syntax(tok.start, "invalid property name"),
                };
                match self.tok.kind {
                    TokenKind::Colon => {
                        if !computed && self.lx.text(&tok) == b"__proto__" {
                            return self.unsupported(tok.start, "__proto__ in object literals is");
                        }
                        self.next()?;
                        if let Some(a) = atom {
                            self.name_hint = Some((a, self.tok.start));
                        }
                        self.parse_assign(false)?;
                        match atom {
                            Some(a) => self.code().define_field(a),
                            None => self.code().define_elem(),
                        }
                    }
                    TokenKind::LParen => {
                        return self.unsupported(tok.start, "methods are");
                    }
                    TokenKind::Comma | TokenKind::RBrace if shorthand_ok => {
                        if tok.kw == Kw::Yield {
                            return syntax(tok.start, "unexpected 'yield' keyword");
                        }
                        if tok.is_escaped_reserved() || tok.kw.is_strict_reserved() {
                            return syntax(tok.start, "invalid property name");
                        }
                        if tok.is_contextual(Kw::Await) && self.f.kind == FuncKind::AsyncBody {
                            return self.unsupported(tok.start, "await is");
                        }
                        let a = atom.unwrap_or(Atom::NULL);
                        if tok.kw == Kw::Arguments {
                            self.note_arguments();
                        }
                        let r = self.name_ref(a, tok.start);
                        self.mark(tok.start);
                        self.code().name_get(r);
                        self.code().define_field(a);
                    }
                    TokenKind::Assign if shorthand_ok => {
                        // `{ a = 1 }` is only valid as a destructuring pattern.
                        if super::scan::closes_before_assign(self.lx, b'{') {
                            return self.unsupported(tok.start, "destructuring assignment is");
                        }
                        return syntax(self.tok.start, "expecting '}'");
                    }
                    _ => return syntax(self.tok.start, "expecting '}'"),
                }
            }
            if !self.eat(TokenKind::Comma)? {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "}")?;
        Ok(Expr::ObjectLit)
    }
}
