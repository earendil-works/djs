//! The bytecode hash of a function template: what a snapshot checks so that "pc N of
//! this function" means the same instruction on export and import.
//!
//! What goes in, and why:
//! - every instruction by opcode *name* (not byte), its byte offset, and its immediates;
//!   atom immediates by the atom's string, intrinsic immediates by the intrinsic's name
//!   and prelude function table slots by what they hold (D3711), since atom and
//!   intrinsic ids and the slots move whenever builtins come or go. The offsets
//!   are in because a frame's pc and its header's return pcs are byte offsets: a change
//!   in an instruction's encoded size must change the hash even if the sequence stays.
//! - the frame shape: parameter count, `length`, locals, operand stack depth, captures,
//!   the function flags: a suspended frame's registers are laid out by them.
//! - the capture descriptors (which parent slot each Box comes from) and the handler
//!   table (try regions, their targets and stack depths: a resumed `try` jumps by it).
//! - the constants: numbers by bits, strings by content (template-object caches are
//!   state, carried by the snapshot, not code).
//! - the function's name (`fn.name` is read from the template) and its children's
//!   source spans in order (`fclosure i` names child `i`).
//!
//! Left out: the line table and the local names (stack-trace text and messages, not the
//! meaning of a pc), inline cache slots' contents, the template's cell layout.

use super::wire::Fnv;
use crate::bytecode::decode::insns;
use crate::bytecode::imm::ImmKind;
use crate::bytecode::template;
use crate::heap::cells::FuncTemplate;
use crate::heap::{Atom, Heap};

pub(super) fn template_hash(h: &Heap, t: u32) -> u64 {
    let a = h.arena();
    let mut f = Fnv::new();
    f.str("djs-function");
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::PARAMS)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::LENGTH)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::LOCALS)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::STACK)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::FLAGS)));
    f.u32(u32::from(a.get::<u8>(t, FuncTemplate::NATIVE)));
    f.u32(u32::from(a.get::<u8>(t, FuncTemplate::FAST)));
    let name = a.get::<u32>(t, FuncTemplate::NAME);
    f.str(&h.atom_name(Atom::from_id(name)));
    let code = template::code(h, t);
    let consts = template::consts(h, t);
    f.u32(code.len() as u32);
    for insn in insns(code) {
        let Ok(insn) = insn else {
            f.str("<bad>");
            break;
        };
        f.u32(insn.pc);
        f.str(insn.op.name());
        if insn.op == crate::bytecode::Op::GetTemplateObject {
            // Its constant is the site's strings, then the template object the run made:
            // state, carried by the snapshot.
            continue;
        }
        for (i, k) in insn.info().imms.iter().enumerate() {
            match k.wide() {
                ImmKind::Atom => f.str(&h.atom_name(insn.atom(i))),
                // By value: the pool is the script's, so its numbering moves with the
                // other functions' constants (an index crossing 256 changes the
                // instruction's size, which the offsets catch).
                ImmKind::Const => constant(h, &mut f, consts.get(insn.imm(i) as usize).copied()),
                ImmKind::Intrinsic => f.str(crate::bytecode::intrinsics::intrinsic_name(
                    insn.imm(i) as u16,
                )),
                ImmKind::Builtin => builtin(h, &mut f, insn.imm(i)),
                _ => f.u32(insn.imm(i)),
            }
        }
    }
    let caps = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
    for i in 0..caps {
        f.u32(template::capture_desc(h, t, i));
    }
    f.bytes(template::handlers(h, t));
    let n = template::child_count(h, t);
    f.u32(n);
    for i in 0..n {
        match template::child(h, t, i).and_then(|r| r.get()) {
            Some(c) => {
                let d = template::debug_info(h, c);
                f.u32(d.src_start);
                f.u32(d.src_end);
            }
            None => f.u32(u32::MAX),
        }
    }
    f.0
}

/// A prelude function table slot (`get_builtin`) by what it holds (D3711): a known
/// builtin by its name (`Math.floor`), a prelude function by its function's name. The slot
/// numbers move whenever a builtin comes or goes.
fn builtin(h: &Heap, f: &mut Fnv, k: u32) {
    use crate::interp::realm::{Intrinsic, intrinsic};
    if let Some((g, m)) = crate::interp::known::name_of(h, k) {
        f.str("known");
        f.str(&h.atom_name(g));
        if let Some(m) = m {
            f.str(&h.atom_name(m));
        }
        return;
    }
    let t = intrinsic(h, Intrinsic::PreludeFunctions).as_internal();
    let fun = t
        .filter(|&t| k < h.value_array_len(t))
        .and_then(|t| h.value_array_get(t, k).as_object())
        .filter(|&o| h.object_class(o) == crate::heap::ObjectClass::Function);
    match fun {
        Some(o) => {
            let name = h.arena().get::<u32>(h.closure_template(o), FuncTemplate::NAME);
            f.str("prelude");
            f.str(&h.atom_name(Atom::from_id(name)));
        }
        None => f.u32(k),
    }
}

/// A constant by value: strings by content, numbers by bits; the rest (a regex literal's
/// program, a template-object cache) by kind only, being code or state.
fn constant(h: &Heap, f: &mut Fnv, c: Option<crate::value::Value>) {
    match c {
        Some(c) if c.as_string().is_some() => {
            f.str("s");
            for u in h.str_units(c.as_string().unwrap()) {
                f.bytes(&u.to_le_bytes());
            }
        }
        Some(c) if c.is_number() => {
            f.str("n");
            f.u64(c.to_bits());
        }
        Some(_) => f.str("k"),
        None => f.str("-"),
    }
}

/// The interface hash of a template: what a closure of it relies on when no frame of it is
/// stored. Its code may change; its closures keep working as long as they hold the same
/// Boxes in the same order (the captured bindings by name), the same home object slot,
/// and the same `length` and `name`.
pub(super) fn interface_hash(h: &Heap, t: u32) -> u64 {
    let a = h.arena();
    let mut f = Fnv::new();
    f.str("djs-interface");
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::PARAMS)));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::LENGTH)));
    let caps = a.get::<u16>(t, FuncTemplate::CAPTURES);
    f.u32(u32::from(caps));
    f.u32(u32::from(a.get::<u16>(t, FuncTemplate::FLAGS)));
    let name = a.get::<u32>(t, FuncTemplate::NAME);
    f.str(&h.atom_name(Atom::from_id(name)));
    let d = template::debug_info(h, t);
    let locals = u32::from(a.get::<u16>(t, FuncTemplate::LOCALS));
    match d.names.get() {
        Some(names) => {
            use crate::heap::cells::U32Array;
            let len: u32 = a.get(names, U32Array::LEN);
            for i in locals..(locals + u32::from(caps)).min(len) {
                let id: u32 = a.read(U32Array::DATA.at(names, i));
                f.str(&h.atom_name(Atom::from_id(id)));
            }
        }
        None => f.str("<no names>"),
    }
    f.0
}
