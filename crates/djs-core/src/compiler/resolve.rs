//! Resolution and template creation (docs/compiler.md 5.3 to 5.9, D1081).
//!
//! After the whole unit is parsed:
//!
//! 1. every name reference of every function is classified, children first: a binding
//!    of the function itself, a capture (the walk through the enclosing functions marks
//!    the binding captured and adds capture slots to every function in between), or a
//!    global; TDZ checks are decided (5.6);
//! 2. every binding gets storage: an argument slot, a local slot, a local slot holding a
//!    Box (captured bindings, D1083), or global;
//! 3. each function's phase-1 code is re-emitted into final code and written as a
//!    template, children before parents.

use alloc::borrow::Cow;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use super::func::{Capture, FuncKind, FuncState, ScopeKind, VarId, VarKind};
use super::ts_decl::{DeclKind, EnumVal, TsDecl};
use super::{CompileOptions, PErr, PResult};
use crate::bytecode::global_decls::GlobalDecls;
use crate::bytecode::{
    CodeBuf, Label, LineTableBuilder, Op, TemplateError, TemplateParts, alloc_script,
    alloc_template_in, capture_desc, func_flags, special,
};
use crate::heap::{Atom, Heap, Ref};
use crate::lexer::LineTable;

/// Where a binding lives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Storage {
    /// Never referenced pseudo-bindings.
    None,
    Local(u16),
    /// A local slot holding the Box of a captured binding.
    Box(u16),
    Arg(u16),
    /// Script top-level declarations.
    Global,
}

/// What a name reference resolved to.
#[derive(Clone, Copy, Debug)]
enum Target {
    Var(VarId),
    Capture(u16),
    Global,
    /// A member of a TypeScript enum or namespace (compiler/ts_decl.rs): a property of
    /// the object that reference `.0` (to the closure's parameter) loads.
    Member(u32),
}

#[derive(Clone, Copy, Debug)]
struct Resolved {
    target: Target,
    /// The binding is immutable (`const`, a named function expression's name).
    immutable: bool,
    /// A TDZ check is needed (5.6).
    check: bool,
    /// `let`/`const`: initialization of a global one is `init_global_lex`.
    lexical: bool,
    /// The binding holds this TypeScript enum or namespace (const enum folding).
    decl: Option<u32>,
}

pub(super) fn internal(what: &str) -> PErr {
    PErr::syntax(0, Cow::Owned(format!("internal compiler error: {what}")))
}

/// Resolves all functions of the unit and writes their templates. `funcs` is indexed
/// by function id; id 0 is the top level.
pub(super) fn finish(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
    atom_undefined: Atom,
    mut funcs: Vec<FuncState>,
    decls: &[TsDecl],
) -> PResult<Ref> {
    // 1. Classify references, children first. (Member references add a reference to
    // their namespace parameter, classified in turn.)
    let mut resolved: Vec<Vec<Resolved>> = (0..funcs.len()).map(|_| Vec::new()).collect();
    for id in (0..funcs.len()).rev() {
        let mut r = 0;
        while r < funcs[id].refs.len() {
            let res = classify(&mut funcs, id, r, decls);
            resolved[id].push(res);
            r += 1;
        }
    }

    // 2. Storage.
    let storages: Vec<(Vec<Storage>, Vec<Atom>)> = funcs.iter().map(assign_storage).collect();
    for (s, _) in &storages {
        if s.len() > u16::MAX as usize {
            return Err(PErr::syntax(0, Cow::Borrowed("too many local variables")));
        }
    }

    // 3. Templates, children first.
    let script = {
        let name = heap.alloc_str(opts.filename)?;
        // Privileged code (the prelude) keeps no source text: its functions print as
        // native code, and the text would be a quarter of the base image (D1340).
        let source = match core::str::from_utf8(src) {
            Ok(s) if !opts.privileged => Some(heap.alloc_str(s)?),
            _ => None,
        };
        alloc_script(heap, name, source)?
    };
    let line_table = LineTable::new(src);
    let mut phase1 = Vec::with_capacity(funcs.len());
    for f in funcs.iter_mut() {
        let code = core::mem::take(&mut f.code)
            .finish()
            .map_err(|e| internal(&format!("{e:?}")))?;
        phase1.push(code);
    }
    let mut templates: Vec<Ref> = vec![Ref::NULL; funcs.len()];
    let mut scratch = Scratch::default();
    for id in (0..funcs.len()).rev() {
        let f = &funcs[id];
        let parent_storage = f.parent.map(|(p, _)| &storages[p as usize].0);
        let t = emit_function(
            heap,
            f,
            &mut phase1[id],
            &resolved[id],
            &storages[id],
            parent_storage,
            &templates,
            atom_undefined,
            decls,
            // Privileged code (the prelude) keeps no line tables or local names: its
            // frames show as `(native)` or not at all in stack traces, and it has no
            // TDZ errors to name (D1430).
            (!opts.privileged).then_some(&line_table),
            opts.privileged,
            src,
            script,
            &mut scratch,
        )?;
        templates[id] = t;
    }
    Ok(templates[0])
}

/// Classifies reference `r` of function `id`, walking into enclosing functions for
/// free names (adding capture slots on the way).
fn classify(funcs: &mut [FuncState], id: usize, r: usize, decls: &[TsDecl]) -> Resolved {
    let nr = funcs[id].refs[r];
    // A member of the namespace of function `f`'s closure: a property of its
    // parameter, reached from the reference's own scope.
    let member = |funcs: &mut [FuncState], f: usize| -> Option<Resolved> {
        let decl = &decls[funcs[f].ns? as usize];
        let m = decl.members.iter().find(|m| m.name == nr.atom)?;
        funcs[id].refs.push(super::func::NameRef {
            atom: decl.param,
            scope: nr.scope,
            pos: nr.pos,
            init: false,
        });
        Some(Resolved {
            target: Target::Member((funcs[id].refs.len() - 1) as u32),
            immutable: false,
            check: false,
            lexical: false,
            decl: m.child,
        })
    };
    if let Some(v) = funcs[id].lookup(nr.scope, nr.atom) {
        let var = &mut funcs[id].vars[v as usize];
        var.used = true;
        let immutable = var.kind.is_immutable();
        let lexical = var.kind.is_lexical();
        let decl = var.ts_decl;
        if var.global {
            return Resolved {
                target: Target::Global,
                immutable,
                check: false,
                lexical,
                decl,
            };
        }
        let check = !nr.init && needs_check(&funcs[id], v, nr.pos);
        return Resolved {
            target: Target::Var(v),
            immutable,
            check,
            lexical,
            decl,
        };
    }
    if let Some(m) = member(funcs, id) {
        return m;
    }
    // A free name: find the enclosing function that binds it.
    let mut path = vec![id];
    let mut at = funcs[id].parent;
    while let Some((pf, ps)) = at {
        let pf = pf as usize;
        let Some(v) = funcs[pf].lookup(ps, nr.atom) else {
            if let Some(m) = member(funcs, pf) {
                return m;
            }
            path.push(pf);
            at = funcs[pf].parent;
            continue;
        };
        {
            let var = &mut funcs[pf].vars[v as usize];
            if var.global {
                return Resolved {
                    target: Target::Global,
                    immutable: var.kind.is_immutable(),
                    check: false,
                    lexical: var.kind.is_lexical(),
                    decl: var.ts_decl,
                };
            }
            let decl = var.ts_decl;
            var.captured = true;
            var.used = true;
            let immutable = var.kind.is_immutable();
            // Closures may run at any time: lexical bindings are always checked
            // through captures (and parameters of non-simple lists, whose defaults can
            // create closures before later parameters are initialized).
            let check =
                var.kind.is_lexical() || (var.kind == VarKind::Param && !funcs[pf].simple_params);
            // The function directly inside `pf` takes the parent's Box; each one below
            // takes its parent's capture.
            let mut cap = Capture::Local(v);
            let mut slot = 0;
            for &f in path.iter().rev() {
                slot = funcs[f].capture_slot(cap, nr.atom);
                cap = Capture::Outer(slot);
            }
            return Resolved {
                target: Target::Capture(slot),
                immutable,
                check,
                lexical: false,
                decl,
            };
        }
    }
    Resolved {
        target: Target::Global,
        immutable: false,
        check: false,
        lexical: false,
        decl: None,
    }
}

/// Whether an access at `pos` to binding `v` in its own function needs a TDZ check
/// (docs/compiler.md 5.6).
fn needs_check(f: &FuncState, v: VarId, pos: u32) -> bool {
    let var = &f.vars[v as usize];
    let tdz = var.kind.is_lexical() || (var.kind == VarKind::Param && !f.simple_params);
    if !tdz || var.global {
        // Globals are checked at run time by the global ops.
        return false;
    }
    pos < var.init_end || f.scopes[var.scope as usize].kind == ScopeKind::Switch
}

/// Storage for every binding of `f`, and the local slot names.
fn assign_storage(f: &FuncState) -> (Vec<Storage>, Vec<Atom>) {
    let mut storage = Vec::with_capacity(f.vars.len());
    let mut names = Vec::new();
    for v in &f.vars {
        let pseudo = v.kind.is_pseudo();
        storage.push(if v.global {
            Storage::Global
        } else if pseudo && !v.used {
            Storage::None
        } else if v.kind == VarKind::Param && f.simple_params && !v.captured {
            Storage::Arg(v.param)
        } else {
            names.push(v.name);
            let slot = (names.len() - 1) as u16;
            if v.captured {
                Storage::Box(slot)
            } else {
                Storage::Local(slot)
            }
        });
    }
    (storage, names)
}

/// Handlers whose depth is relative to a label (`HandlerLabels::depth_from`) get their
/// absolute depth from the final code. A region may sit in code that only another such
/// handler reaches (a pattern in a catch block of a pattern default...), so this
/// repeats until nothing changes.
fn resolve_relative_depths(
    f: &FuncState,
    code: &crate::bytecode::Code,
    handlers: &mut [crate::bytecode::Handler],
) -> PResult<()> {
    let mut pending: Vec<usize> = (0..handlers.len())
        .filter(|&i| f.handlers[i].depth_from.is_some())
        .collect();
    while !pending.is_empty() {
        let depths = crate::bytecode::stack_depths(&code.code, handlers)
            .map_err(|e| internal(&format!("{e:?}")))?;
        let before = pending.len();
        pending.retain(|&i| {
            let h = &f.handlers[i];
            let at = h.depth_from.and_then(|l| code.label_pos(l));
            match at.map(|pc| depths.get(pc as usize).copied()) {
                Some(Some(d)) if d != crate::bytecode::UNKNOWN_DEPTH => {
                    handlers[i].depth = d + h.depth;
                    false
                }
                _ => true,
            }
        });
        if pending.len() == before {
            return Err(internal("unreachable relative handler depth"));
        }
    }
    Ok(())
}

/// Stores the value on top of the stack into a binding being initialized.
fn emit_init(out: &mut CodeBuf, storage: Storage, atom: Atom, lexical: bool) {
    match storage {
        Storage::Local(s) => out.put_loc(s),
        Storage::Box(s) => out.put_box(s),
        Storage::Arg(a) => out.put_arg(a),
        Storage::Global if lexical => out.init_global_lex(atom),
        Storage::Global => out.put_global(atom, 0),
        Storage::None => out.drop(),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_function(
    heap: &mut Heap,
    f: &FuncState,
    phase1: &mut crate::bytecode::Phase1Code,
    resolved: &[Resolved],
    (storage, names): &(Vec<Storage>, Vec<Atom>),
    parent_storage: Option<&Vec<Storage>>,
    templates: &[Ref],
    atom_undefined: Atom,
    decls: &[TsDecl],
    line_table: Option<&LineTable>,
    privileged: bool,
    src: &[u8],
    script: Ref,
    scratch: &mut Scratch,
) -> PResult<Ref> {
    let mut consts = f.consts.clone();
    let mut needs_uninit = vec![false; f.vars.len()];
    for r in resolved {
        if let (Target::Var(v), true) = (r.target, r.check) {
            needs_uninit[v as usize] = true;
        }
    }

    let all = core::mem::take(&mut phase1.insns);
    let estimate = all.len() * 3;
    let mut label_count = phase1.label_count;
    let all = super::peephole::simplify(all, &mut label_count);
    let mut out = CodeBuf::with_labels(label_count);
    out.reserve(estimate + 16);

    // ---- Prologue (docs/compiler.md 5.5, 5.7).
    if f.kind == FuncKind::Script {
        let mut decls = GlobalDecls::default();
        for v in f.vars.iter().filter(|v| v.global) {
            match v.kind {
                VarKind::Var => decls.vars.push(v.name),
                VarKind::Func => decls.funcs.push(v.name),
                VarKind::Let => decls.lets.push(v.name),
                VarKind::Const => decls.consts.push(v.name),
                _ => {}
            }
        }
        if !decls.is_empty() {
            let idx = consts.len() as u32;
            consts.extend(decls.encode());
            out.global_decls(idx);
        }
    }
    for &p in &f.params {
        match storage[p as usize] {
            Storage::Box(s) if f.simple_params => {
                out.get_arg(f.vars[p as usize].param);
                out.new_box_init(s);
            }
            // Non-simple lists: the phase-1 code initializes them in order.
            Storage::Box(s) => out.new_box(s),
            Storage::Local(s) if needs_uninit[p as usize] => out.set_loc_uninit(s),
            _ => {}
        }
    }
    let pseudo = [
        (f.this_var, special::THIS),
        (f.arguments_var, special::ARGUMENTS),
        (f.new_target_var, special::NEW_TARGET),
        (f.func_name_var, special::ACTIVE_FUNCTION),
        (f.home_var, special::HOME_OBJECT),
        (f.active_var, special::ACTIVE_FUNCTION),
    ];
    for (var, kind) in pseudo {
        if let Some(v) = var {
            let st = storage[v as usize];
            match st {
                Storage::Local(s) => {
                    out.special_object(kind);
                    out.put_loc(s);
                }
                Storage::Box(s) => {
                    out.special_object(kind);
                    out.new_box_init(s);
                }
                _ => {}
            }
        }
    }
    for (i, v) in f.vars.iter().enumerate() {
        // Captured `var`s of the function scope start as `undefined` in their Box.
        if v.kind == VarKind::Var
            && let Storage::Box(s) = storage[i]
        {
            out.undefined();
            out.new_box_init(s);
        }
    }

    // ---- Re-emit.
    let mut lines = core::mem::take(&mut scratch.lines);
    lines.clear();
    let mut li = 0;
    // The line table entries of phase-1 instructions up to `pc` start at the current
    // position of `out` (the last one wins).
    let flush = |out: &CodeBuf, li: &mut usize, lines: &mut Vec<(u32, u32)>, pc: u32| {
        while *li < f.lines.len() && f.lines[*li].0 <= pc {
            let entry = (out.pos(), f.lines[*li].1);
            match lines.last_mut() {
                Some(last) if last.0 == entry.0 => *last = entry,
                _ => lines.push(entry),
            }
            *li += 1;
        }
    };
    let mut i = 0;
    while i < all.len() {
        let insn = all[i];
        i += 1;
        flush(&out, &mut li, &mut lines, insn.pc);
        match insn.op {
            Op::NameGet | Op::NamePut | Op::NameSet | Op::NameInit | Op::NameTypeof => {
                let r = insn.imm(0) as usize;
                let atom = f.refs[r].atom;
                // `i++;` / `++i;` on an unchecked local: `get_loc; inc; put_loc` in place.
                if insn.op == Op::NameGet
                    && let [upd, put, ..] = all[i..]
                    && matches!(upd.op, Op::Inc | Op::Dec)
                    && matches!(put.op, Op::NamePut | Op::NameSet)
                    && let Some(s) = unchecked_local(storage, resolved[r])
                    && unchecked_local(storage, resolved[put.imm(0) as usize]) == Some(s)
                    && !resolved[put.imm(0) as usize].immutable
                {
                    // Errors point at the update (the operator), as for `inc`.
                    flush(&out, &mut li, &mut lines, upd.pc);
                    if upd.op == Op::Inc {
                        out.inc_loc(s);
                    } else {
                        out.dec_loc(s);
                    }
                    if put.op == Op::NameSet {
                        out.get_loc(s);
                    }
                    i += 2;
                    continue;
                }
                // `x = v; ... x`, `const p = f(); if (p ...)`: keep the stored value
                // (unless the read starts an `inc_loc`).
                if matches!(insn.op, Op::NamePut | Op::NameInit)
                    && let Some(get) = all.get(i)
                    && get.op == Op::NameGet
                    && !all
                        .get(i + 1)
                        .is_some_and(|u| matches!(u.op, Op::Inc | Op::Dec))
                    && (!resolved[r].immutable || insn.op == Op::NameInit)
                    && let Some(s) = unchecked_local(storage, resolved[r])
                    && unchecked_local(storage, resolved[get.imm(0) as usize]) == Some(s)
                {
                    flush(&out, &mut li, &mut lines, get.pc);
                    out.set_loc(s);
                    i += 1;
                    continue;
                }
                // `v === undefined`
                if insn.op == Op::NameGet
                    && atom == atom_undefined
                    && matches!(resolved[r].target, Target::Global)
                    && let Some(cmp) = all.get(i)
                    && matches!(cmp.op, Op::StrictEq | Op::StrictNe)
                {
                    let jump = all.get(i + 1).filter(|j| {
                        matches!(j.op, Op::JumpIfFalse | Op::JumpIfTrue) && cmp.op == Op::StrictNe
                    });
                    if cmp.op == Op::StrictEq || jump.is_some() {
                        flush(&out, &mut li, &mut lines, cmp.pc);
                        out.is_undefined();
                        i += 1;
                        if let Some(j) = jump {
                            // `!==`: the inverted jump.
                            let inv = if j.op == Op::JumpIfFalse {
                                Op::JumpIfTrue
                            } else {
                                Op::JumpIfFalse
                            };
                            out.emit_insn(&j.with_op(inv));
                            i += 1;
                        }
                        continue;
                    }
                }
                if insn.op == Op::NameGet
                    && let Some(d) = resolved[r].decl
                    && let Some((v, n)) = fold_const_enum(decls, d, &all[i..])
                {
                    // `CE.A`, `N.CE.A` of a const enum: the member's value (as tsc).
                    match v {
                        EnumVal::Num(x) => emit_number(&mut out, &mut consts, *x),
                        EnumVal::Str(units) => {
                            let s = if units.iter().all(|&u| u <= 0xFF) {
                                let b: Vec<u8> = units.iter().map(|&u| u as u8).collect();
                                heap.alloc_latin1(&b)?
                            } else {
                                heap.alloc_utf16(units)?
                            };
                            consts.push(crate::value::Value::string(s));
                            out.push_const((consts.len() - 1) as u32);
                        }
                    }
                    i += n;
                    continue;
                }
                if let Target::Member(obj) = resolved[r].target {
                    let obj = obj as usize;
                    let load = |out: &mut CodeBuf| {
                        let a = f.refs[obj].atom;
                        emit_name(out, storage, Op::NameGet, a, resolved[obj], atom_undefined);
                    };
                    match insn.op {
                        Op::NameGet | Op::NameTypeof => {
                            load(&mut out);
                            out.get_field(atom, 0);
                        }
                        Op::NameSet => {
                            out.dup();
                            load(&mut out);
                            out.swap();
                            out.put_field(atom, 0);
                        }
                        _ => {
                            load(&mut out);
                            out.swap();
                            out.put_field(atom, 0);
                        }
                    }
                    continue;
                }
                emit_name(
                    &mut out,
                    storage,
                    insn.op,
                    atom,
                    resolved[r],
                    atom_undefined,
                );
            }
            Op::EnterScope => {
                let s = insn.imm(0) as usize;
                let scope = &f.scopes[s];
                for &v in &scope.vars {
                    let var = &f.vars[v as usize];
                    if matches!(
                        var.kind,
                        VarKind::PrivateName | VarKind::PrivateMethod | VarKind::PrivateAccessor
                    ) {
                        // A new private symbol per class evaluation (D1400).
                        use crate::heap::cells::flags::{SYM_PRIVATE_METHOD, SYM_PRIVATE_READONLY};
                        let method = match var.kind {
                            VarKind::PrivateName => 0,
                            VarKind::PrivateMethod => SYM_PRIVATE_METHOD | SYM_PRIVATE_READONLY,
                            _ => SYM_PRIVATE_METHOD,
                        } as u8;
                        match storage[v as usize] {
                            Storage::Box(slot) => {
                                out.private_symbol(var.name, method);
                                out.new_box_init(slot);
                            }
                            Storage::Local(slot) => {
                                out.private_symbol(var.name, method);
                                out.put_loc(slot);
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if var.kind == VarKind::Hidden {
                        if let Storage::Box(slot) = storage[v as usize] {
                            out.new_box(slot);
                        }
                        continue;
                    }
                    // Parameters, `var`s and the pseudo-bindings are set up in the prologue.
                    if !matches!(
                        var.kind,
                        VarKind::Let | VarKind::Const | VarKind::Func | VarKind::CatchParam
                    ) || var.global
                    {
                        continue;
                    }
                    match storage[v as usize] {
                        Storage::Box(slot)
                            if var.kind.is_lexical() || var.kind == VarKind::CatchParam =>
                        {
                            out.new_box(slot)
                        }
                        Storage::Box(slot) => {
                            out.undefined();
                            out.new_box_init(slot);
                        }
                        Storage::Local(slot) if needs_uninit[v as usize] => {
                            out.set_loc_uninit(slot)
                        }
                        _ => {}
                    }
                }
                // `var`s named like a parameter (a separate var scope, D1820).
                for &(v, p) in &scope.copies {
                    match storage[p as usize] {
                        Storage::Local(s) => out.get_loc(s),
                        Storage::Box(s) => out.get_box(s),
                        Storage::Arg(a) => out.get_arg(a),
                        _ => out.undefined(),
                    }
                    emit_init(
                        &mut out,
                        storage[v as usize],
                        f.vars[v as usize].name,
                        false,
                    );
                }
                // Hoisted function declarations (docs/compiler.md 5.5).
                for &(v, child) in &scope.funcs {
                    out.fclosure(child);
                    let var = &f.vars[v as usize];
                    emit_init(&mut out, storage[v as usize], var.name, false);
                }
            }
            Op::IterScope => {
                // CreatePerIterationEnvironment: fresh Boxes for captured bindings.
                let s = insn.imm(0) as usize;
                for &v in &f.scopes[s].vars {
                    if let Storage::Box(slot) = storage[v as usize] {
                        out.rebox(slot);
                    }
                }
            }
            // The generator/async state local (docs/compiler.md 7.7): phase-1 code holds
            // a placeholder.
            Op::GeneratorStart
            | Op::Yield
            | Op::AsyncStart
            | Op::Await
            | Op::AsyncReturn
            | Op::AsyncReject => {
                let slot = match f.state_var.map(|v| storage[v as usize]) {
                    Some(Storage::Local(s)) => s,
                    _ => return Err(internal("state op without a state local")),
                };
                match insn.op {
                    Op::GeneratorStart => out.generator_start(slot),
                    Op::Yield => out.yield_(slot),
                    Op::AsyncStart => out.async_start(slot),
                    Op::Await => out.await_(slot),
                    Op::AsyncReturn => out.async_return(slot),
                    _ => out.async_reject(slot),
                }
            }
            Op::LeaveScope => {}
            Op::Label => out.place_label(Label(insn.imm(0))),
            _ => out.emit_insn(&insn),
        }
    }
    let code = out.finish().map_err(|e| internal(&format!("{e:?}")))?;
    let mut handlers = Vec::with_capacity(f.handlers.len());
    for h in &f.handlers {
        let pos = |l| {
            code.label_pos(l)
                .ok_or_else(|| internal("unplaced handler label"))
        };
        handlers.push(crate::bytecode::Handler {
            start: pos(h.start)?,
            end: pos(h.end)?,
            target: pos(h.target)?,
            depth: if h.depth_from.is_some() {
                crate::bytecode::UNKNOWN_DEPTH
            } else {
                h.depth
            },
        });
    }
    resolve_relative_depths(f, &code, &mut handlers)?;

    // ---- Captures, children, line table, template.
    let mut capture_words = Vec::with_capacity(f.captures.len());
    for &c in &f.captures {
        capture_words.push(match c {
            Capture::Outer(i) => capture_desc::outer(i),
            Capture::Local(v) => match parent_storage.map(|p| p[v as usize]) {
                Some(Storage::Box(slot)) => capture_desc::local(slot),
                other => return Err(internal(&format!("capture of a non-Box binding {other:?}"))),
            },
        });
    }
    let children: Vec<Ref> = f.children.iter().map(|&c| templates[c as usize]).collect();
    let (line_bytes, all_names) = match line_table {
        Some(line_table) => {
            let mut lt = LineTableBuilder::new();
            let mut cursor = crate::lexer::LineCursor::default();
            for &(pc, pos) in &lines {
                let (line, col) = line_table.line_col_from(src, pos, &mut cursor);
                lt.add(pc, line, col);
            }
            let mut all_names = names.clone();
            all_names.extend_from_slice(&f.capture_names);
            (lt.finish(), all_names)
        }
        None => (Vec::new(), Vec::new()),
    };
    let kind = match f.kind {
        // Eval code runs like a Script's top level (the embedder supplies `this`).
        FuncKind::Script | FuncKind::Eval => func_flags::KIND_SCRIPT,
        FuncKind::AsyncBody => func_flags::KIND_ASYNC_BODY,
        FuncKind::Normal => func_flags::KIND_NORMAL,
        FuncKind::Arrow => func_flags::KIND_ARROW,
        FuncKind::Method => func_flags::KIND_METHOD,
        FuncKind::Getter => func_flags::KIND_GETTER,
        FuncKind::Setter => func_flags::KIND_SETTER,
        FuncKind::ClassCtor => func_flags::KIND_CLASS_CTOR,
        FuncKind::DerivedCtor => func_flags::KIND_DERIVED_CTOR,
        FuncKind::FieldInit | FuncKind::StaticBlock => func_flags::KIND_FIELD_INIT,
    };
    let mut flags = kind;
    if privileged {
        flags |= func_flags::PRIVILEGED;
    }
    if f.is_async {
        flags |= func_flags::ASYNC;
    }
    if f.is_generator {
        flags |= func_flags::GENERATOR;
    }
    if f.sloppy_this {
        flags |= func_flags::SLOPPY_THIS;
    }
    // Functions that use `super.x` (themselves or in arrows) get a home object slot.
    if f.home_var.is_some() {
        flags |= func_flags::HOME;
    }
    let params = f.param_count - u16::from(f.rest.is_some());
    let parts = TemplateParts {
        flags,
        name: f.name,
        params,
        length: f.length,
        locals: names.len() as u16,
        captures: &capture_words,
        code: &code.code,
        consts: &consts,
        children: &children,
        handlers: &handlers,
        lines: &line_bytes,
        names: &all_names,
        script,
        src_start: f.src_start,
        src_end: f.src_end,
    };
    let t = alloc_template_in(heap, &parts, &mut scratch.stack).map_err(|e| match e {
        TemplateError::Heap(e) => e.into(),
        e => internal(&format!("{e:?}")),
    });
    scratch.lines = lines;
    t
}

/// Buffers reused across the functions of a unit (0096: per-function allocations were
/// a tenth of a compile).
#[derive(Default)]
struct Scratch {
    lines: Vec<(u32, u32)>,
    stack: crate::bytecode::StackScratch,
}

/// A const enum member read through the binding of declaration `d`: the
/// `get_field` chain at the start of `next` (through namespaces) and its value, with
/// the number of `get_field`s it replaces.
fn fold_const_enum<'d>(
    decls: &'d [TsDecl],
    d: u32,
    next: &[crate::bytecode::Insn],
) -> Option<(&'d EnumVal, usize)> {
    let mut cur = d as usize;
    for (n, insn) in next.iter().enumerate() {
        if insn.op != Op::GetField {
            return None;
        }
        let decl = &decls[cur];
        let m = decl.members.iter().find(|m| m.name == insn.atom(0))?;
        match decl.kind {
            DeclKind::ConstEnum => return m.value.as_ref().map(|v| (v, n + 1)),
            DeclKind::Namespace => cur = m.child? as usize,
            DeclKind::Enum => return None,
        }
    }
    None
}

/// The local slot of a reference that resolved to a local without a TDZ check.
fn unchecked_local(storage: &[Storage], res: Resolved) -> Option<u16> {
    match res.target {
        Target::Var(v) if !res.check => match storage[v as usize] {
            Storage::Local(s) => Some(s),
            _ => None,
        },
        _ => None,
    }
}

/// Pushes a number constant (small integers inline).
fn emit_number(out: &mut CodeBuf, consts: &mut Vec<crate::value::Value>, n: f64) {
    let i = n as i32;
    if i as f64 == n && !(n == 0.0 && n.is_sign_negative()) {
        match i8::try_from(i) {
            Ok(b) => out.push_i8(b),
            Err(_) => out.push_i32(i),
        }
        return;
    }
    consts.push(crate::value::Value::number(n));
    out.push_const((consts.len() - 1) as u32);
}

/// The resolved form of one `name_*` op.
fn emit_name(
    out: &mut CodeBuf,
    storage: &[Storage],
    op: Op,
    atom: Atom,
    res: Resolved,
    atom_undefined: Atom,
) {
    #[derive(Clone, Copy)]
    enum At {
        Loc(u16),
        Box(u16),
        Arg(u16),
        Cap(u16),
        Global,
    }
    let at = match res.target {
        Target::Var(v) => match storage[v as usize] {
            Storage::Local(s) => At::Loc(s),
            Storage::Box(s) => At::Box(s),
            Storage::Arg(a) => At::Arg(a),
            Storage::Global | Storage::None => At::Global,
        },
        Target::Capture(c) => At::Cap(c),
        // Handled by the caller (a property access).
        Target::Global | Target::Member(_) => At::Global,
    };
    let lexical = res.lexical;
    let chk = res.check;
    // Reads, with or without the TDZ check.
    let get = |out: &mut CodeBuf, typeof_: bool| match at {
        At::Loc(s) if chk => out.get_loc_chk(s),
        At::Loc(s) => out.get_loc(s),
        At::Box(s) if chk => out.get_box_chk(s),
        At::Box(s) => out.get_box(s),
        At::Arg(a) => out.get_arg(a),
        At::Cap(c) if chk => out.get_cap_chk(c),
        At::Cap(c) => out.get_cap(c),
        At::Global => {
            // The global `undefined` is non-writable and non-configurable.
            if atom == atom_undefined && matches!(res.target, Target::Global) {
                out.undefined()
            } else if typeof_ {
                out.get_global_typeof(atom, 0)
            } else {
                out.get_global(atom, 0)
            }
        }
    };
    if res.immutable && matches!(op, Op::NamePut | Op::NameSet) {
        // Assignment to a const: TDZ first (ReferenceError), then TypeError.
        if chk || matches!(at, At::Global) {
            get(out, false);
            out.drop();
        }
        out.throw_const_assign(atom);
        return;
    }
    match op {
        Op::NameGet => get(out, false),
        Op::NameTypeof => get(out, true),
        Op::NamePut | Op::NameSet => {
            let keep = op == Op::NameSet;
            match at {
                At::Loc(s) => match (chk, keep) {
                    (true, true) => out.set_loc_chk(s),
                    (true, false) => out.put_loc_chk(s),
                    (false, true) => out.set_loc(s),
                    (false, false) => out.put_loc(s),
                },
                At::Box(s) => match (chk, keep) {
                    (true, true) => out.set_box_chk(s),
                    (true, false) => out.put_box_chk(s),
                    (false, true) => out.set_box(s),
                    (false, false) => out.put_box(s),
                },
                At::Arg(a) => {
                    if keep {
                        out.set_arg(a)
                    } else {
                        out.put_arg(a)
                    }
                }
                At::Cap(c) => match (chk, keep) {
                    (true, true) => out.set_cap_chk(c),
                    (true, false) => out.put_cap_chk(c),
                    (false, true) => out.set_cap(c),
                    (false, false) => out.put_cap(c),
                },
                At::Global => {
                    if keep {
                        out.dup();
                    }
                    out.put_global(atom, 0);
                }
            }
        }
        Op::NameInit => match at {
            At::Loc(s) => out.put_loc(s),
            At::Box(s) => out.put_box(s),
            At::Arg(a) => out.put_arg(a),
            At::Cap(c) => out.put_cap(c),
            At::Global if lexical => out.init_global_lex(atom),
            At::Global => out.put_global(atom, 0),
        },
        _ => out.nop(),
    }
}
