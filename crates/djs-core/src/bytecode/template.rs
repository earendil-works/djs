//! Heap helpers for compiled code cells (`Bytes`, `RefArray`, `FuncTemplate`) and the
//! template disassembler. The cell layouts live in `heap/cells.rs`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use super::disasm::{DisasmContext, disasm};
use super::handlers::HandlerTable;
use crate::heap::cells::{Bytes, CellKind, FuncTemplate, RefArray, U32Array, ValueArray};
use crate::heap::{Atom, Heap, HeapError, Ref};
use crate::value::Value;

/// Allocates a `Bytes` cell holding `data`.
pub fn alloc_bytes(heap: &mut Heap, data: &[u8]) -> Result<Ref, HeapError> {
    let len = u32::try_from(data.len()).map_err(|_| HeapError::OutOfMemory)?;
    let size = Bytes::DATA.cell_size(len).ok_or(HeapError::OutOfMemory)?;
    let off = heap.alloc_cell(CellKind::Bytes, size)?;
    let a = heap.arena_mut();
    a.set(off, Bytes::LEN, len);
    a.slice_mut(Bytes::DATA.at(off, 0), len)
        .copy_from_slice(data);
    Ok(Ref(off))
}

/// The contents of a `Bytes` cell; empty for null.
pub fn bytes_of(heap: &Heap, r: Ref) -> &[u8] {
    match r.get() {
        None => &[],
        Some(off) => {
            debug_assert_eq!(heap.cell_kind(off), CellKind::Bytes);
            let len = heap.arena().get(off, Bytes::LEN);
            heap.arena().slice(Bytes::DATA.at(off, 0), len)
        }
    }
}

/// Allocates a `RefArray` holding `refs`.
pub fn alloc_ref_array(heap: &mut Heap, refs: &[Ref]) -> Result<Ref, HeapError> {
    let len = u32::try_from(refs.len()).map_err(|_| HeapError::OutOfMemory)?;
    let size = RefArray::REFS
        .cell_size(len)
        .ok_or(HeapError::OutOfMemory)?;
    let off = heap.alloc_cell(CellKind::RefArray, size)?;
    let a = heap.arena_mut();
    a.set(off, RefArray::LEN, len);
    for (i, &r) in refs.iter().enumerate() {
        a.write(RefArray::REFS.at(off, i as u32), r);
    }
    Ok(Ref(off))
}

/// Entry `i` of a `RefArray` (`None` for null arrays and out of range).
pub fn ref_array_get(heap: &Heap, r: Ref, i: u32) -> Option<Ref> {
    let off = r.get()?;
    debug_assert_eq!(heap.cell_kind(off), CellKind::RefArray);
    (i < heap.arena().get(off, RefArray::LEN)).then(|| heap.arena().read(RefArray::REFS.at(off, i)))
}

pub fn ref_array_len(heap: &Heap, r: Ref) -> u32 {
    r.get()
        .map_or(0, |off| heap.arena().get(off, RefArray::LEN))
}

fn value_array(heap: &Heap, r: Ref) -> Vec<Value> {
    let Some(off) = r.get() else {
        return Vec::new();
    };
    let len = heap.arena().get(off, ValueArray::LEN);
    (0..len)
        .map(|i| heap.arena().read(ValueArray::VALUES.at(off, i)))
        .collect()
}

fn u32_array(heap: &Heap, r: Ref) -> Vec<u32> {
    let Some(off) = r.get() else {
        return Vec::new();
    };
    let len = heap.arena().get(off, U32Array::LEN);
    (0..len)
        .map(|i| heap.arena().read(U32Array::DATA.at(off, i)))
        .collect()
}

/// Disassembles a function template and, after it, all its descendants: a header line
/// per function (name, sizes), the handler table, then the code with names and
/// constants resolved.
pub fn disasm_template(heap: &Heap, template: Ref) -> String {
    let mut out = String::new();
    // Explicit work list rather than recursion: templates nest as deep as the source.
    let mut work = alloc::vec![(template, String::from("0"))];
    while let Some((t, path)) = work.pop() {
        let Some(off) = t.get() else { continue };
        let a = heap.arena();
        let name = Atom::from_id(a.get(off, FuncTemplate::NAME));
        let locals = a.get(off, FuncTemplate::LOCALS);
        let consts = value_array(heap, a.get(off, FuncTemplate::CONSTS));
        let names: Vec<Atom> = u32_array(heap, a.get(off, FuncTemplate::NAMES))
            .into_iter()
            .map(Atom::from_id)
            .collect();
        let shown = if name.is_null() {
            String::from("<anonymous>")
        } else {
            heap.atom_name(name)
        };
        let _ = writeln!(
            out,
            "function {path} {shown}: params={} locals={locals} stack={} captures={}",
            a.get(off, FuncTemplate::PARAMS),
            a.get(off, FuncTemplate::STACK),
            a.get(off, FuncTemplate::CAPTURES),
        );
        if let Ok(h) = HandlerTable::new(bytes_of(heap, a.get(off, FuncTemplate::HANDLERS))) {
            for h in h.iter() {
                let _ = writeln!(
                    out,
                    "  handler [{}, {}) -> {} depth {}",
                    h.start, h.end, h.target, h.depth
                );
            }
        }
        let cx = DisasmContext {
            heap: Some(heap),
            consts: &consts,
            names: &names,
            locals: locals as usize,
            phase1: false,
        };
        out.push_str(&disasm(bytes_of(heap, a.get(off, FuncTemplate::CODE)), &cx));
        let children = a.get(off, FuncTemplate::CHILDREN);
        let n = ref_array_len(heap, children);
        for i in (0..n).rev() {
            if let Some(c) = ref_array_get(heap, children, i) {
                work.push((c, alloc::format!("{path}.{i}")));
            }
        }
    }
    out
}
