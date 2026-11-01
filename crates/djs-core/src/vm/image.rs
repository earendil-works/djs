//! The base image's content (0127): what a shipped image needs and a VM that compiled
//! the prelude still has.

use alloc::vec;

use crate::bytecode::decode::insns;
use crate::bytecode::imm::ImmKind;
use crate::bytecode::template::{debug_info, func_flags};
use crate::heap::cells::{Bytes, FuncTemplate, PropEntry, PropTable, Shape, Symbol, U32Array};
use crate::heap::{CellKind, Heap, HeapError, Ref};

/// Drops the prelude's internal names (D2253): the names of privileged functions
/// (`ArrayPrototypeMap`) and the atoms of its locals and parameters, which compiling the
/// prelude interned and nothing refers to. User code can't observe either: a builtin's
/// `name` is the installed property (D1433), and stack traces skip prelude frames
/// (D1980). The atoms that stay are the ones the heap refers to: property keys (property
/// tables, shape descriptors), symbols, function and local names of templates, the atom
/// immediates of the bytecode, and the global declarations' atom ids in constant pools
/// (int32 values: every int32 constant counts, a few false keeps at most).
///
/// The forgotten strings are garbage for the collection that follows.
pub(super) fn drop_internal_names(heap: &mut Heap) -> Result<u32, HeapError> {
    let templates: alloc::vec::Vec<u32> = heap
        .cells()
        .filter(|c| c.kind == CellKind::FuncTemplate)
        .map(|c| c.offset)
        .collect();
    for &t in &templates {
        if heap.arena().get(t, FuncTemplate::FLAGS) & func_flags::PRIVILEGED != 0 {
            heap.arena_mut().set(t, FuncTemplate::NAME, 0u32);
        }
    }
    let mut keep = vec![false; heap.atom_count() as usize];
    let mut mark = |id: u32| {
        // String and symbol atoms (integer atoms have bit 31): their table index.
        if id & (1 << 31) == 0
            && let Some(k) = keep.get_mut((id & ((1 << 30) - 1)) as usize)
        {
            *k = true;
        }
    };
    let a = heap.arena();
    for c in heap.cells() {
        let at = c.offset;
        match c.kind {
            CellKind::Symbol => mark(a.get(at, Symbol::ATOM)),
            CellKind::PropTable => {
                for i in 0..a.get(at, PropTable::COUNT) {
                    mark(a.get(PropTable::ENTRIES.at(at, i), PropEntry::KEY));
                }
            }
            CellKind::Shape => {
                if let Some(d) = a.get::<Ref>(at, Shape::DESCS).get() {
                    for i in 0..a.read::<u32>(U32Array::DATA.at(d, 0)) {
                        mark(a.read(U32Array::DATA.at(d, 1 + 2 * i)));
                    }
                }
            }
            CellKind::FuncTemplate => {
                mark(a.get(at, FuncTemplate::NAME));
                if let Some(n) = debug_info(heap, at).names.get() {
                    for i in 0..a.get(n, U32Array::LEN) {
                        mark(a.read(U32Array::DATA.at(n, i)));
                    }
                }
                if let Some(code) = a.get::<Ref>(at, FuncTemplate::CODE).get() {
                    let bytes = a.slice(Bytes::DATA.at(code, 0), a.get(code, Bytes::LEN));
                    for insn in insns(bytes) {
                        let insn = insn.expect("the prelude's bytecode decodes");
                        for (k, kind) in insn.info().imms.iter().enumerate() {
                            if *kind == ImmKind::Atom {
                                mark(insn.imm(k));
                            }
                        }
                    }
                }
                if let Some(consts) = a.get::<Ref>(at, FuncTemplate::CONSTS).get() {
                    for i in 0..heap.value_array_len(consts) {
                        if let Some(n) = heap.value_array_get(consts, i).as_int() {
                            mark(n as u32);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    heap.forget_string_atoms(&keep)
}
