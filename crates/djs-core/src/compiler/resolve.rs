//! Resolution and template creation (docs/compiler.md 5.3 to 5.8).
//!
//! After the whole unit is parsed: every name reference is classified (local binding or
//! global), TDZ checks are decided (5.6), bindings get storage, and one linear re-emit
//! turns the phase-1 code into final code. P0 has a single function, so there are no
//! captures yet; free names are globals. P1-1 adds the walk to enclosing functions
//! (D1081) in `classify`.

use alloc::borrow::Cow;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use super::func::{FuncKind, FuncState, ScopeKind, VarId, VarKind};
use super::parser::Parser;
use super::{PErr, PResult};
use crate::bytecode::global_decls::GlobalDecls;
use crate::bytecode::{
    CodeBuf, Label, LineTableBuilder, Op, TemplateError, TemplateParts, alloc_script,
    alloc_template, func_flags, insns, special,
};
use crate::heap::{Atom, Ref};
use crate::lexer::LineTable;

/// Where a binding lives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Storage {
    Local(u16),
    Arg(u16),
    /// Script top-level `var`/`let`/`const`.
    Global,
}

/// What a name reference resolved to.
#[derive(Clone, Copy, Debug)]
struct Resolved {
    var: Option<VarId>,
    /// A TDZ check is needed (5.6).
    check: bool,
}

fn internal(what: &str) -> PErr {
    PErr::Syntax {
        pos: 0,
        msg: Cow::Owned(format!("internal compiler error: {what}")),
    }
}

/// Resolves, re-emits and writes the template.
pub(super) fn finish(p: Parser<'_, '_>) -> PResult<Ref> {
    let Parser {
        heap,
        f,
        src,
        opts,
        atom_undefined,
        ..
    } = p;

    // 1. Classify references and decide TDZ checks.
    let resolved: Vec<Resolved> = f
        .refs
        .iter()
        .map(|r| {
            let var = f.lookup(r.scope, r.atom);
            let check = var.is_some_and(|v| needs_check(&f, v, r.pos));
            Resolved { var, check }
        })
        .collect();
    let mut needs_uninit = vec![false; f.vars.len()];
    for r in &resolved {
        if let (Some(v), true) = (r.var, r.check) {
            needs_uninit[v as usize] = true;
        }
    }

    // 2. Storage.
    let mut storage = Vec::with_capacity(f.vars.len());
    let mut names: Vec<Atom> = Vec::new();
    for v in &f.vars {
        storage.push(if v.global {
            Storage::Global
        } else if v.kind == VarKind::Param {
            Storage::Arg(v.param)
        } else {
            if names.len() >= u16::MAX as usize {
                return Err(PErr::Syntax {
                    pos: 0,
                    msg: Cow::Borrowed("too many local variables"),
                });
            }
            names.push(v.name);
            Storage::Local((names.len() - 1) as u16)
        });
    }

    // 3. Script declarations.
    let mut consts = f.consts.clone();
    let mut decls = GlobalDecls::default();
    for v in f.vars.iter().filter(|v| v.global) {
        match v.kind {
            VarKind::Var => decls.vars.push(v.name),
            VarKind::Let => decls.lets.push(v.name),
            VarKind::Const => decls.consts.push(v.name),
            _ => {}
        }
    }

    // 4. Re-emit.
    let phase1 = f.code.finish().map_err(|e| internal(&format!("{e:?}")))?;
    let mut out = CodeBuf::with_labels(phase1.label_count);
    if !decls.is_empty() {
        let idx = consts.len() as u32;
        consts.extend(decls.encode());
        out.global_decls(idx);
    }
    if let Some(this) = f.this_var
        && let Storage::Local(s) = storage[this as usize]
    {
        out.special_object(special::THIS);
        out.put_loc(s);
    }
    let mut lines: Vec<(u32, u32)> = Vec::new();
    let mut li = 0;
    for insn in insns(&phase1.code) {
        let insn = insn.map_err(|e| internal(&format!("{e:?}")))?;
        while li < f.lines.len() && f.lines[li].0 <= insn.pc {
            let entry = (out.pos(), f.lines[li].1);
            match lines.last_mut() {
                Some(last) if last.0 == entry.0 => *last = entry,
                _ => lines.push(entry),
            }
            li += 1;
        }
        match insn.op {
            Op::NameGet | Op::NamePut | Op::NameSet | Op::NameInit | Op::NameTypeof => {
                let r = insn.imm(0) as usize;
                let atom = f.refs[r].atom;
                let res = resolved[r];
                let var = res.var.map(|v| (&f.vars[v as usize], storage[v as usize]));
                emit_name(&mut out, insn.op, atom, var, res.check, atom_undefined);
            }
            Op::EnterScope => {
                let s = insn.imm(0) as usize;
                for &v in &f.scopes[s].vars {
                    if needs_uninit[v as usize]
                        && let Storage::Local(slot) = storage[v as usize]
                    {
                        out.set_loc_uninit(slot);
                    }
                }
            }
            Op::LeaveScope => {}
            Op::Label => out.place_label(Label(insn.imm(0))),
            _ => out.emit_insn(&insn),
        }
    }
    let code = out.finish().map_err(|e| internal(&format!("{e:?}")))?;

    // 5. Line table and template.
    let line_table = LineTable::new(src);
    let mut lt = LineTableBuilder::new();
    for (pc, pos) in lines {
        let (line, col) = line_table.line_col(src, pos);
        lt.add(pc, line, col);
    }
    let lines = lt.finish();
    let name = heap.alloc_str(opts.filename)?;
    let source = match core::str::from_utf8(src) {
        Ok(s) => Some(heap.alloc_str(s)?),
        Err(_) => None,
    };
    let script = alloc_script(heap, name, source)?;
    let flags = match f.kind {
        FuncKind::Script => func_flags::KIND_SCRIPT,
        FuncKind::AsyncBody => func_flags::KIND_ASYNC_BODY | func_flags::ASYNC,
    };
    let parts = TemplateParts {
        flags,
        name: Atom::NULL,
        params: f.param_count,
        length: f.param_count,
        locals: names.len() as u16,
        code: &code.code,
        consts: &consts,
        lines: &lines,
        names: &names,
        script,
        src_start: 0,
        src_end: src.len() as u32,
        ..TemplateParts::default()
    };
    alloc_template(heap, &parts).map_err(|e| match e {
        TemplateError::Heap(_) => PErr::Oom,
        e => internal(&format!("{e:?}")),
    })
}

/// Whether an access at `pos` to binding `v` (in the same function) needs a TDZ check
/// (docs/compiler.md 5.6).
fn needs_check(f: &FuncState, v: VarId, pos: u32) -> bool {
    let var = &f.vars[v as usize];
    if !var.kind.is_lexical() || var.global {
        // Globals are checked at run time by the global ops.
        return false;
    }
    pos < var.init_end || f.scopes[var.scope as usize].kind == ScopeKind::Switch
}

/// The resolved form of one `name_*` op.
fn emit_name(
    out: &mut CodeBuf,
    op: Op,
    atom: Atom,
    var: Option<(&super::func::Var, Storage)>,
    check: bool,
    atom_undefined: Atom,
) {
    let storage = var.map_or(Storage::Global, |(_, s)| s);
    let is_const = var.is_some_and(|(v, _)| v.kind == VarKind::Const);
    if is_const && matches!(op, Op::NamePut | Op::NameSet) {
        // Assignment to a const: TDZ first (ReferenceError), then TypeError.
        match storage {
            Storage::Local(s) if check => {
                out.get_loc_chk(s);
                out.drop();
            }
            Storage::Global => {
                out.get_global(atom);
                out.drop();
            }
            _ => {}
        }
        out.throw_const_assign(atom);
        return;
    }
    match (op, storage) {
        (Op::NameGet | Op::NameTypeof, Storage::Local(s)) => {
            if check {
                out.get_loc_chk(s)
            } else {
                out.get_loc(s)
            }
        }
        (Op::NameGet | Op::NameTypeof, Storage::Arg(a)) => out.get_arg(a),
        (Op::NameGet, Storage::Global) => {
            // The global `undefined` is non-writable and non-configurable.
            if atom == atom_undefined && var.is_none() {
                out.undefined()
            } else {
                out.get_global(atom)
            }
        }
        (Op::NameTypeof, Storage::Global) => {
            if atom == atom_undefined && var.is_none() {
                out.undefined()
            } else {
                out.get_global_typeof(atom)
            }
        }
        (Op::NamePut, Storage::Local(s)) => {
            if check {
                out.put_loc_chk(s)
            } else {
                out.put_loc(s)
            }
        }
        (Op::NameSet, Storage::Local(s)) => {
            if check {
                out.set_loc_chk(s)
            } else {
                out.set_loc(s)
            }
        }
        (Op::NameInit, Storage::Local(s)) => out.put_loc(s),
        (Op::NamePut | Op::NameInit, Storage::Arg(a)) => out.put_arg(a),
        (Op::NameSet, Storage::Arg(a)) => out.set_arg(a),
        (Op::NamePut, Storage::Global) => out.put_global(atom),
        (Op::NameSet, Storage::Global) => {
            out.dup();
            out.put_global(atom);
        }
        (Op::NameInit, Storage::Global) => {
            if var.is_some_and(|(v, _)| v.kind.is_lexical()) {
                out.init_global_lex(atom)
            } else {
                out.put_global(atom)
            }
        }
        _ => out.nop(),
    }
}
