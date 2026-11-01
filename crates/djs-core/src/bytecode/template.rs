//! Heap helpers for compiled code cells (`Bytes`, `RefArray`, `FuncTemplate`) and the
//! template disassembler. The cell layouts live in `heap/cells.rs`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use super::disasm::{DisasmContext, disasm};
use super::handlers::{Handler, HandlerTable, encode_handlers};
use super::stack::{StackError, StackScratch, check_stack_in};
use crate::heap::cells::{Bytes, CellKind, FuncTemplate, RefArray, Script, U32Array, ValueArray};
use crate::heap::{Atom, Heap, HeapError, Ref};
use crate::value::{Str, Value};

/// `FuncTemplate::FLAGS`: the function kind in the low 4 bits, then flags.
pub mod func_flags {
    pub const KIND_MASK: u32 = 0xF;
    /// A Script's top level (D004): called with `this` = the global object.
    pub const KIND_SCRIPT: u32 = 0;
    /// A codemode body (D008): `(async (params) => { body })`, called with `this` = the
    /// global object and the host's arguments.
    pub const KIND_ASYNC_BODY: u32 = 1;
    pub const KIND_NORMAL: u32 = 2;
    pub const KIND_ARROW: u32 = 3;
    pub const KIND_METHOD: u32 = 4;
    pub const KIND_GETTER: u32 = 5;
    pub const KIND_SETTER: u32 = 6;
    pub const KIND_CLASS_CTOR: u32 = 7;
    pub const KIND_DERIVED_CTOR: u32 = 8;
    pub const KIND_FIELD_INIT: u32 = 9;
    pub const ASYNC: u32 = 1 << 4;
    pub const GENERATOR: u32 = 1 << 5;
    /// The closure has a home object slot after its captures (methods that use `super`
    /// properties, directly or in arrows; class constructors and field initializers).
    pub const HOME: u32 = 1 << 6;
    /// Compiled in privileged mode (the prelude): closures get no `length`, `name` or
    /// `prototype`, since the `%Install*` intrinsics define them on the builtins that
    /// user code can see (D1433).
    pub const PRIVILEGED: u32 = 1 << 8;
    /// D015: a function from a Function constructor whose body has no "use strict":
    /// OrdinaryCallBindThis is sloppy (undefined/null become the global object,
    /// primitives their wrappers). Nothing else of sloppy mode.
    pub const SLOPPY_THIS: u32 = 1 << 9;
    /// 0063: a `@native` builtin; `FuncTemplate::NATIVE` holds its index into the
    /// generated table (`builtins::gen`), set by `%MarkNative`. With
    /// `ExecOptions::native_builtins` its calls get native frames.
    pub const NATIVE: u32 = 1 << 10;
    /// D1850: a builtin with a leaf fast path; `FuncTemplate::FAST` holds its index into
    /// `builtins::fast::FAST`, set by `%MarkFast`. With `ExecOptions::native_builtins`
    /// a call tries the fast path first and runs the builtin only when it declines.
    pub const FAST: u32 = 1 << 11;

    pub const fn kind(flags: u32) -> u32 {
        flags & KIND_MASK
    }
}

/// The parts of a function template, for [`alloc_template`].
#[derive(Clone, Copy, Debug)]
pub struct TemplateParts<'a> {
    pub flags: u32,
    pub name: Atom,
    pub params: u16,
    pub length: u16,
    pub locals: u16,
    /// Capture descriptors (`FuncTemplate::CAPTURE_DESC`), one per capture.
    pub captures: &'a [u32],
    /// Final code (labels resolved).
    pub code: &'a [u8],
    pub consts: &'a [Value],
    /// Child templates.
    pub children: &'a [Ref],
    pub handlers: &'a [Handler],
    /// An encoded line table (`LineTableBuilder::finish`).
    pub lines: &'a [u8],
    /// Local names, then capture names; may be empty.
    pub names: &'a [Atom],
    pub script: Ref,
    pub src_start: u32,
    pub src_end: u32,
}

impl Default for TemplateParts<'_> {
    fn default() -> Self {
        TemplateParts {
            flags: 0,
            name: Atom::NULL,
            params: 0,
            length: 0,
            locals: 0,
            captures: &[],
            code: &[],
            consts: &[],
            children: &[],
            handlers: &[],
            lines: &[],
            names: &[],
            script: Ref::NULL,
            src_start: 0,
            src_end: 0,
        }
    }
}

/// Why [`alloc_template`] failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TemplateError {
    Heap(HeapError),
    /// The code fails the stack checker (a compiler bug).
    Stack(StackError),
    /// More than `u16::MAX` captures.
    TooManyCaptures,
}

impl From<HeapError> for TemplateError {
    fn from(e: HeapError) -> Self {
        TemplateError::Heap(e)
    }
}

/// Checks the code with the stack checker (which also computes `STACK`) and writes a
/// `FuncTemplate` cell with its `Bytes`, `ValueArray`, `RefArray` and `U32Array` parts.
/// Empty parts are stored as null.
pub fn alloc_template(heap: &mut Heap, p: &TemplateParts<'_>) -> Result<Ref, TemplateError> {
    alloc_template_in(heap, p, &mut StackScratch::default())
}

/// [`alloc_template`] with the stack checker's buffers reused (the compiler's loop over
/// the functions of a unit).
pub fn alloc_template_in(
    heap: &mut Heap,
    p: &TemplateParts<'_>,
    scratch: &mut StackScratch,
) -> Result<Ref, TemplateError> {
    let stack = check_stack_in(p.code, p.handlers, scratch).map_err(TemplateError::Stack)?;
    let cache_sites = &scratch.caches;
    let captures = u16::try_from(p.captures.len()).map_err(|_| TemplateError::TooManyCaptures)?;
    let mut code_bytes = p.code.to_vec();
    let ic_count = number_caches(&mut code_bytes, cache_sites);
    let code = alloc_bytes(heap, &code_bytes)?;
    let consts = nonempty(p.consts, |v| alloc_value_array(heap, v))?;
    let children = nonempty(p.children, |c| alloc_ref_array(heap, c))?;
    let capture_desc = nonempty(p.captures, |c| alloc_u32_array(heap, c))?;
    let handlers = nonempty(p.handlers, |h| alloc_bytes(heap, &encode_handlers(h)))?;
    let lines = nonempty(p.lines, |l| alloc_bytes(heap, l))?;
    let ids: Vec<u32> = p.names.iter().map(|a| a.id()).collect();
    let names = nonempty(&ids, |n| alloc_u32_array(heap, n))?;
    let off = heap.alloc_cell(CellKind::FuncTemplate, FuncTemplate::LAYOUT.min_size)?;
    let a = heap.arena_mut();
    a.set(off, FuncTemplate::FLAGS, p.flags);
    a.set(off, FuncTemplate::NAME, p.name.id());
    a.set(off, FuncTemplate::PARAMS, p.params);
    a.set(off, FuncTemplate::LENGTH, p.length);
    a.set(off, FuncTemplate::LOCALS, p.locals);
    a.set(off, FuncTemplate::STACK, stack);
    a.set(off, FuncTemplate::CAPTURES, captures);
    a.set(off, FuncTemplate::IC_COUNT, ic_count);
    a.set(off, FuncTemplate::CODE, code);
    a.set(off, FuncTemplate::CONSTS, consts);
    a.set(off, FuncTemplate::CHILDREN, children);
    a.set(off, FuncTemplate::CAPTURE_DESC, capture_desc);
    a.set(off, FuncTemplate::HANDLERS, handlers);
    a.set(off, FuncTemplate::LINES, lines);
    a.set(off, FuncTemplate::SCRIPT, p.script);
    a.set(off, FuncTemplate::SRC_START, p.src_start);
    a.set(off, FuncTemplate::SRC_END, p.src_end);
    a.set(off, FuncTemplate::NAMES, names);
    Ok(Ref(off))
}

/// The `Cache` immediate of a function with more cache sites than slots (D1620).
pub const NO_CACHE: u16 = u16::MAX;

/// Numbers the `Cache` immediates of `code` at `sites` (in code order, D1620); returns
/// the count. Sites beyond `NO_CACHE - 1` get `NO_CACHE` (never cached).
fn number_caches(code: &mut [u8], sites: &[u32]) -> u16 {
    let mut n: u16 = 0;
    for &at in sites {
        let idx = if n < NO_CACHE { n } else { NO_CACHE };
        code[at as usize..at as usize + 2].copy_from_slice(&idx.to_le_bytes());
        n = n.saturating_add(1);
    }
    n
}

fn nonempty<T>(
    items: &[T],
    f: impl FnOnce(&[T]) -> Result<Ref, HeapError>,
) -> Result<Ref, HeapError> {
    if items.is_empty() {
        Ok(Ref::NULL)
    } else {
        f(items)
    }
}

/// Allocates a `Script` cell.
pub fn alloc_script(heap: &mut Heap, name: Str, source: Option<Str>) -> Result<Ref, HeapError> {
    let off = heap.alloc_cell(CellKind::Script, Script::LAYOUT.min_size)?;
    let a = heap.arena_mut();
    a.set(off, Script::NAME, Ref(name.offset()));
    a.set(
        off,
        Script::SOURCE,
        source.map_or(Ref::NULL, |s| Ref(s.offset())),
    );
    Ok(Ref(off))
}

fn alloc_value_array(heap: &mut Heap, values: &[Value]) -> Result<Ref, HeapError> {
    let len = u32::try_from(values.len()).map_err(|_| HeapError::OutOfMemory)?;
    let size = ValueArray::VALUES
        .cell_size(len)
        .ok_or(HeapError::OutOfMemory)?;
    let off = heap.alloc_cell(CellKind::ValueArray, size)?;
    let a = heap.arena_mut();
    a.set(off, ValueArray::LEN, len);
    for (i, &v) in values.iter().enumerate() {
        a.write(ValueArray::VALUES.at(off, i as u32), v);
    }
    Ok(Ref(off))
}

fn alloc_u32_array(heap: &mut Heap, words: &[u32]) -> Result<Ref, HeapError> {
    let len = u32::try_from(words.len()).map_err(|_| HeapError::OutOfMemory)?;
    let size = U32Array::DATA
        .cell_size(len)
        .ok_or(HeapError::OutOfMemory)?;
    let off = heap.alloc_cell(CellKind::U32Array, size)?;
    let a = heap.arena_mut();
    a.set(off, U32Array::LEN, len);
    for (i, &w) in words.iter().enumerate() {
        a.write(U32Array::DATA.at(off, i as u32), w);
    }
    Ok(Ref(off))
}

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
            lines: bytes_of(heap, a.get(off, FuncTemplate::LINES)),
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
