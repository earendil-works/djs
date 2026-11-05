//! Compiling a lazy function at its first call (D2990, work/research/0218).
//!
//! The unit's compile parsed the whole source, reported every early error, classified
//! every name and wrote a lazy template for each lazy function (`resolve::lazy_roots`):
//! its flags, name, parameters, capture descriptors and names, and a record of where its
//! parameters start and the context its parse needs. Here its source text is parsed
//! again on its own: a stand-in for the enclosing function binds one variable per
//! capture (with the binding's attributes from the descriptors), the function's capture
//! slots are those variables in the template's order, and `resolve::finish` emits the
//! function and the eager functions inside it. Positions are relative to the function's
//! start (`Unit::base`): only its text is copied, so a first call costs its size, not the
//! script's.

use alloc::vec::Vec;

use super::func::{BODY_SCOPE, Capture, FuncKind, FuncState, VarKind};
use super::parser::Parser;
use super::resolve::Unit;
use super::{CompileError, CompileOptions, PResult, error_at};
use crate::bytecode::capture_desc;
use crate::bytecode::lines::line_entries;
use crate::bytecode::template::{bytes_of, debug_info, func_flags};
use crate::heap::cells::{FuncTemplate, Script, U32Array};
use crate::heap::{Atom, Heap, Ref};
use crate::value::Str;

/// `FuncState::owner` as the lazy record stores it (`FuncKind as u32`).
const KINDS: [FuncKind; 13] = [
    FuncKind::Script,
    FuncKind::AsyncBody,
    FuncKind::Eval,
    FuncKind::Normal,
    FuncKind::Arrow,
    FuncKind::Method,
    FuncKind::Getter,
    FuncKind::Setter,
    FuncKind::ClassCtor,
    FuncKind::DerivedCtor,
    FuncKind::FieldInit,
    FuncKind::StaticBlock,
    FuncKind::Normal,
];

/// Compiles lazy template `t` and returns its compiled template.
pub fn compile_lazy(heap: &mut Heap, t: Ref) -> Result<Ref, CompileError> {
    let a = heap.arena();
    let flags = u32::from(a.get::<u16>(t.0, FuncTemplate::FLAGS));
    let name = Atom::from_id(a.get(t.0, FuncTemplate::NAME));
    let n_caps = u32::from(a.get::<u16>(t.0, FuncTemplate::CAPTURES));
    let code_len: u32 = a.get(t.0, FuncTemplate::CODE_LEN);
    let words: Vec<u32> = (0..n_caps)
        .map(|i| a.read(FuncTemplate::DATA.at(t.0, code_len + 4 * i)))
        .collect();
    let info = debug_info(heap, t.0);
    let names: Vec<Atom> = match info.names.get() {
        Some(n) => (0..n_caps)
            .map(|i| Atom::from_id(a.read(U32Array::DATA.at(n, i))))
            .collect(),
        None => Vec::new(),
    };
    let mut record = [(1, 1); 3];
    for (i, e) in line_entries(bytes_of(heap, info.lines)).take(3).enumerate() {
        record[i] = (e.line, e.col);
    }
    let [(line, col), (_, head), (_, ctx)] = record;
    let script = info.script;
    let text = |r: Ref| {
        r.get()
            .map(|s| heap.str_to_string(Str(s)))
            .unwrap_or_default()
    };
    let (src, filename, params, bits) = match script.get() {
        Some(s) => (
            text(a.get(s, Script::SOURCE)),
            text(a.get(s, Script::NAME)),
            text(a.get(s, Script::PARAMS)),
            a.get::<u32>(s, Script::OPTIONS),
        ),
        None => Default::default(),
    };
    // (Batch 1 converts the whole source; the span is what is parsed.)
    let span = src
        .as_bytes()
        .get(info.src_start as usize..info.src_end as usize)
        .unwrap_or_default();
    let params: Vec<&str> = if params.is_empty() {
        Vec::new()
    } else {
        params.split(',').collect()
    };
    let opts = CompileOptions::from_bits(&filename, &params, bits);
    let unit = Unit {
        lazy: true,
        root: true,
        script,
        base: info.src_start,
        line,
        col,
    };
    let lazy = Lazy {
        flags,
        name,
        names: &names,
        words: &words,
        head,
        ctx,
    };
    let compiled = Parser::new(heap, span, &opts)
        .and_then(|p| lazy.compile(p, unit))
        .map_err(|e| error_at(span, e))?;
    // The capture descriptors say where closures of `t` take their Boxes from, which
    // only the enclosing unit knew; the name may come from the enclosing context too.
    let a = heap.arena_mut();
    let n = u32::from(a.get::<u16>(compiled.0, FuncTemplate::CAPTURES));
    if n != n_caps {
        return Err(error_at(
            span,
            super::resolve::internal("lazy captures differ"),
        ));
    }
    let code_len: u32 = a.get(compiled.0, FuncTemplate::CODE_LEN);
    for (i, &w) in words.iter().enumerate() {
        a.write(
            FuncTemplate::DATA.at(compiled.0, code_len + 4 * i as u32),
            w,
        );
    }
    a.set(compiled.0, FuncTemplate::NAME, name.id());
    Ok(compiled)
}

/// What a lazy compile reads from the template.
struct Lazy<'a> {
    flags: u32,
    name: Atom,
    names: &'a [Atom],
    words: &'a [u32],
    head: u32,
    ctx: u32,
}

impl Lazy<'_> {
    fn compile(&self, mut p: Parser<'_, '_>, unit: Unit) -> PResult<Ref> {
        let kind = func_flags::kind(self.flags);
        // The stand-in for the enclosing function: for an arrow, the kind of the function
        // that owns its `this` (`super`, `arguments`, `new.target` resolve as there).
        let owner = if kind == func_flags::KIND_ARROW {
            KINDS[(self.ctx >> 2) as usize % KINDS.len()]
        } else {
            FuncKind::Normal
        };
        let mut outer = FuncState::new(0, owner, None);
        outer.src_end = p.src.len() as u32;
        for (&n, &w) in self.names.iter().zip(self.words) {
            let kind = match (
                w & capture_desc::IMMUTABLE != 0,
                w & capture_desc::CHECKED != 0,
            ) {
                (true, true) => VarKind::Const,
                (true, false) => VarKind::FuncName,
                (false, true) => VarKind::Let,
                (false, false) => VarKind::Var,
            };
            let v = outer.add_var(n, kind, BODY_SCOPE);
            outer.vars[v as usize].init_end = 0;
            let slot = if n == p.atom_this {
                &mut outer.this_var
            } else if n == p.atom_arguments {
                &mut outer.arguments_var
            } else if n == p.atom_new_target {
                &mut outer.new_target_var
            } else if n == p.atom_home {
                &mut outer.home_var
            } else if n == p.atom_active {
                &mut outer.active_var
            } else {
                continue;
            };
            *slot = Some(v);
        }
        p.f = outer;
        p.completion = None;
        p.unit = unit;
        // Private names resolve through the captures; the unit checked them.
        p.classes.push(Default::default());
        p.lx.seek(self.head);
        p.tok = p.lx.next_token()?;
        p.prev_end = self.head;
        let flavor = (
            self.flags & func_flags::ASYNC != 0,
            self.flags & func_flags::GENERATOR != 0,
        );
        match kind {
            func_flags::KIND_ARROW => {
                p.parse_arrow(false, Atom::NULL, self.ctx & 2 != 0)?;
            }
            func_flags::KIND_NORMAL => {
                p.parse_function_rest(FuncKind::Normal, self.name, 0, self.ctx & 1 != 0, flavor)?;
            }
            _ => {
                let kind = match kind {
                    func_flags::KIND_GETTER => FuncKind::Getter,
                    func_flags::KIND_SETTER => FuncKind::Setter,
                    _ => FuncKind::Method,
                };
                p.parse_method_function(kind, self.name, 0, flavor)?;
            }
        }
        // The function's capture slots are the stand-in's variables, in order.
        if let Some(Some(f)) = p.done.get_mut(1) {
            f.captures = (0..self.names.len() as u32).map(Capture::Local).collect();
            f.capture_names = self.names.to_vec();
        }
        let (t, ..) = p.finish_unit()?;
        Ok(t)
    }
}
