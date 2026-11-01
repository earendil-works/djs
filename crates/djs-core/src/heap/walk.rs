//! Walking the arena cell by cell, tracing cells, and verifying a heap.
//!
//! Tracing is driven entirely by the layout table: a field or tail element field of type
//! `Value` or `Ref` is a slot the GC must visit. This is the groundwork for the copying GC
//! (a separate issue), and [`Heap::verify`] is what restoring an untrusted image runs.

use alloc::vec::Vec;

use super::cells::{
    AtomTable, Header, Object, PropTable, Roots, Rope, Str8, Str16, Symbol, U32Array, ValueArray,
    flags,
};
use super::layout::{FieldTy, Ref};
use super::object::ObjectClass;
use super::{CellKind, Heap};
use crate::value::{Value, tag};

/// One cell found by the walker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellInfo {
    pub offset: u32,
    pub kind: CellKind,
    pub size: u32,
}

/// A traced slot: the arena offset of a `Value` or a `Ref`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Value(u32),
    Ref(u32),
}

/// What [`Heap::verify`] found wrong.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerifyError {
    /// The cell at `offset` has an unknown kind byte.
    BadKind { offset: u32, kind: u8 },
    /// The cell size is not a multiple of 8, is below the kind's minimum, or runs past the
    /// end of the arena.
    BadSize { offset: u32, size: u32 },
    /// GC bits are set outside a collection.
    GcBits { offset: u32 },
    /// The cell at offset 0 is not the roots cell, or there is a second roots cell.
    Roots { offset: u32 },
    /// A length field does not fit the cell.
    BadLength { offset: u32 },
    /// A traced slot holds an invalid value encoding.
    BadValue { slot: u32 },
    /// A traced slot points somewhere that is not the start of a cell of a fitting kind.
    BadRef { slot: u32, target: u32 },
    /// A cell breaks an invariant of its kind (a ref field to the wrong kind of cell, an
    /// inconsistent rope, an out-of-range count, an unknown object class).
    Invalid { offset: u32 },
    /// The prototype chain through the object at `offset` is cyclic.
    ProtoCycle { offset: u32 },
}

/// Iterator over all cells, in address order. Panics on a malformed header; use
/// [`Heap::verify`] on untrusted heaps first.
pub struct Cells<'a> {
    heap: &'a Heap,
    pos: u32,
}

impl Iterator for Cells<'_> {
    type Item = CellInfo;

    fn next(&mut self) -> Option<CellInfo> {
        if self.pos >= self.heap.arena().len() {
            return None;
        }
        let offset = self.pos;
        let kind = self.heap.cell_kind(offset);
        let size = self.heap.cell_size(offset);
        assert!(
            size >= 8 && size.is_multiple_of(8),
            "bad cell size {size} at {offset}"
        );
        self.pos += size;
        Some(CellInfo { offset, kind, size })
    }
}

impl Heap {
    /// Walks every cell in the arena, in address order.
    pub fn cells(&self) -> Cells<'_> {
        Cells { heap: self, pos: 0 }
    }

    /// Visits every traced slot of the cell at `cell` (whether or not it currently holds
    /// a reference).
    pub fn trace_cell(&self, cell: u32, mut f: impl FnMut(Slot)) {
        let layout = self.cell_kind(cell).layout();
        for field in layout.fields {
            visit(field.ty, cell + field.offset, &mut f);
        }
        if let Some(tail) = &layout.tail {
            if !tail.elem.fields.iter().any(|fi| fi.ty.is_traced()) {
                return;
            }
            let n = layout.tail_capacity(self.cell_size(cell));
            for i in 0..n {
                let base = cell + tail.offset + i * tail.elem.size;
                for field in tail.elem.fields {
                    visit(field.ty, base + field.offset, &mut f);
                }
            }
        }
    }

    /// The offsets of all cells the cell at `cell` references.
    pub fn cell_refs(&self, cell: u32) -> Vec<u32> {
        let mut out = Vec::new();
        self.trace_cell(cell, |slot| {
            if let Some(t) = self.slot_target(slot) {
                out.push(t)
            }
        });
        out
    }

    /// The cell a slot references, if any.
    pub fn slot_target(&self, slot: Slot) -> Option<u32> {
        match slot {
            Slot::Value(off) => self.arena().read::<Value>(off).ref_offset(),
            Slot::Ref(off) => self.arena().read::<Ref>(off).get(),
        }
    }

    /// Checks the whole heap: every header, every length field, and every traced slot
    /// (valid encoding, pointing at the start of a cell of a fitting kind). Never panics,
    /// whatever the bytes are.
    pub fn verify(&self) -> Result<(), VerifyError> {
        let a = self.arena();
        let end = a.len();
        let mut starts: Vec<(u32, CellKind)> = Vec::new();
        let mut pos = 0;
        while pos < end {
            if end - pos < 8 {
                return Err(VerifyError::BadSize {
                    offset: pos,
                    size: end - pos,
                });
            }
            let kb = a.get(pos, Header::KIND);
            let Some(kind) = CellKind::from_u8(kb) else {
                return Err(VerifyError::BadKind {
                    offset: pos,
                    kind: kb,
                });
            };
            let size = a.get(pos, Header::SIZE);
            if !size.is_multiple_of(8) || size < kind.layout().min_size || size > end - pos {
                return Err(VerifyError::BadSize { offset: pos, size });
            }
            if a.get(pos, Header::GC) != 0 {
                return Err(VerifyError::GcBits { offset: pos });
            }
            if (pos == 0) != (kind == CellKind::Roots) {
                return Err(VerifyError::Roots { offset: pos });
            }
            if !self.length_fits(pos, kind, size) {
                return Err(VerifyError::BadLength { offset: pos });
            }
            starts.push((pos, kind));
            pos += size;
        }
        if starts.is_empty() {
            return Err(VerifyError::Roots { offset: 0 });
        }
        let kind_at = |t: u32| -> Option<CellKind> {
            starts
                .binary_search_by_key(&t, |&(o, _)| o)
                .ok()
                .map(|i| starts[i].1)
        };
        for &(cell, _) in &starts {
            let mut err = None;
            self.trace_cell(cell, |slot| {
                if err.is_some() {
                    return;
                }
                match slot {
                    Slot::Value(off) => {
                        let bits = a.read::<u64>(off);
                        let Some(v) = Value::from_bits(bits) else {
                            err = Some(VerifyError::BadValue { slot: off });
                            return;
                        };
                        if let Some(t) = v.ref_offset() {
                            let ok = ref_fits(v.tag(), kind_at(t));
                            if !ok {
                                err = Some(VerifyError::BadRef {
                                    slot: off,
                                    target: t,
                                });
                            }
                        }
                    }
                    Slot::Ref(off) => {
                        let t = a.read::<Ref>(off).0;
                        if t != 0 && !matches!(kind_at(t), Some(k) if k != CellKind::Roots) {
                            err = Some(VerifyError::BadRef {
                                slot: off,
                                target: t,
                            });
                        }
                    }
                }
            });
            if let Some(e) = err {
                return Err(e);
            }
        }
        for &(cell, kind) in &starts {
            if !self.cell_invariants(cell, kind, &kind_at) {
                return Err(VerifyError::Invalid { offset: cell });
            }
        }
        self.check_proto_cycles(&starts)
    }

    /// Kind-specific invariants that the rest of the heap code relies on to terminate:
    /// ref fields point at the right kind of cell, counts fit, and rope lengths strictly
    /// decrease towards the leaves (so ropes cannot be cyclic).
    fn cell_invariants(
        &self,
        cell: u32,
        kind: CellKind,
        kind_at: &impl Fn(u32) -> Option<CellKind>,
    ) -> bool {
        let a = self.arena();
        // A ref field: null allowed or not, and the kinds it may point at.
        let fits = |r: Ref, nullable: bool, kinds: &[CellKind]| match r.get() {
            None => nullable,
            Some(t) => kind_at(t).is_some_and(|k| kinds.contains(&k)),
        };
        let is_str = |k: Option<CellKind>| {
            matches!(k, Some(CellKind::Str8 | CellKind::Str16 | CellKind::Rope))
        };
        let pow2 = |r: Ref| self.u32_array_len(r.0).is_power_of_two();
        match kind {
            CellKind::Roots => fits(a.get(cell, Roots::ATOMS), false, &[CellKind::AtomTable]),
            CellKind::AtomTable => {
                let index = a.get(cell, AtomTable::INDEX);
                fits(
                    a.get(cell, AtomTable::ENTRIES),
                    false,
                    &[CellKind::ValueArray],
                ) && fits(index, false, &[CellKind::U32Array])
                    && pow2(index)
            }
            CellKind::Object => {
                let class = ObjectClass::from_u32(a.get(cell, Object::CLASS));
                class.is_some_and(|c| {
                    Object::LAYOUT.tail_capacity(self.cell_size(cell)) >= c.slots()
                }) && fits(a.get(cell, Object::PROTO), true, &[CellKind::Object])
                    && fits(a.get(cell, Object::PROPS), true, &[CellKind::PropTable])
                    && fits(a.get(cell, Object::ELEMENTS), true, &[CellKind::ValueArray])
            }
            CellKind::PropTable => {
                let count = a.get(cell, PropTable::COUNT);
                let index = a.get(cell, PropTable::INDEX);
                count <= PropTable::LAYOUT.tail_capacity(self.cell_size(cell))
                    && a.get(cell, PropTable::DELETED) <= count
                    && fits(index, true, &[CellKind::U32Array])
                    && (index.is_null() || (pow2(index) && self.u32_array_len(index.0) > count))
            }
            CellKind::Rope => {
                let len = a.get(cell, Rope::LEN);
                let (left, right) = (a.get(cell, Rope::LEFT), a.get(cell, Rope::RIGHT));
                let fl = self.cell_flags(cell);
                let two = fl & flags::ROPE_TWO_BYTE != 0;
                if fl & flags::ROPE_FLATTENED != 0 {
                    let k = left.get().and_then(kind_at);
                    let flat = matches!(k, Some(CellKind::Str8 | CellKind::Str16));
                    flat && right.is_null()
                        && a.get(left.0, Str8::LEN) == len
                        && (k == Some(CellKind::Str16)) == two
                } else {
                    let (kl, kr) = (left.get().and_then(kind_at), right.get().and_then(kind_at));
                    if !is_str(kl) || !is_str(kr) {
                        return false;
                    }
                    let (ll, lr) = (a.get(left.0, Str8::LEN), a.get(right.0, Str8::LEN));
                    let two_of = |off: u32, k: Option<CellKind>| match k {
                        Some(CellKind::Str16) => true,
                        Some(CellKind::Rope) => self.cell_flags(off) & flags::ROPE_TWO_BYTE != 0,
                        _ => false,
                    };
                    ll > 0
                        && lr > 0
                        && ll.checked_add(lr) == Some(len)
                        && two == (two_of(left.0, kl) || two_of(right.0, kr))
                }
            }
            CellKind::Symbol => {
                let d: Value = a.get(cell, Symbol::DESCRIPTION);
                d.is_undefined() || d.as_string().is_some()
            }
            _ => true,
        }
    }

    /// Prototype chains must end in null.
    fn check_proto_cycles(&self, starts: &[(u32, CellKind)]) -> Result<(), VerifyError> {
        let objects: Vec<u32> = starts
            .iter()
            .filter(|&&(_, k)| k == CellKind::Object)
            .map(|&(o, _)| o)
            .collect();
        // 0 = unvisited, 1 = on the current path, 2 = known to reach null.
        let mut state = alloc::vec![0u8; objects.len()];
        let mut path = Vec::new();
        for start in 0..objects.len() {
            let mut cur = Some(start);
            while let Some(i) = cur {
                match state[i] {
                    2 => break,
                    1 => return Err(VerifyError::ProtoCycle { offset: objects[i] }),
                    _ => {}
                }
                state[i] = 1;
                path.push(i);
                // Already checked: PROTO is null or an object cell.
                cur = self
                    .arena()
                    .get(objects[i], Object::PROTO)
                    .get()
                    .and_then(|p| objects.binary_search(&p).ok());
            }
            for i in path.drain(..) {
                state[i] = 2;
            }
        }
        Ok(())
    }

    /// Whether the cell's own length field fits its size.
    fn length_fits(&self, cell: u32, kind: CellKind, size: u32) -> bool {
        let a = self.arena();
        let need = match kind {
            CellKind::Str8 => Str8::DATA.cell_size(a.get(cell, Str8::LEN)),
            CellKind::Str16 => Str16::DATA.cell_size(a.get(cell, Str16::LEN)),
            CellKind::ValueArray => ValueArray::VALUES.cell_size(a.get(cell, ValueArray::LEN)),
            CellKind::U32Array => U32Array::DATA.cell_size(a.get(cell, U32Array::LEN)),
            _ => Some(0),
        };
        matches!(need, Some(n) if n <= size)
    }
}

#[inline]
fn visit(ty: FieldTy, off: u32, f: &mut impl FnMut(Slot)) {
    match ty {
        FieldTy::Value => f(Slot::Value(off)),
        FieldTy::Ref => f(Slot::Ref(off)),
        _ => {}
    }
}

/// Whether a reference value with `tag` may point at a cell of `kind`.
fn ref_fits(tag: Option<u16>, kind: Option<CellKind>) -> bool {
    matches!(
        (tag, kind),
        (
            Some(tag::STRING),
            Some(CellKind::Str8 | CellKind::Str16 | CellKind::Rope)
        ) | (Some(tag::OBJECT), Some(CellKind::Object))
            | (Some(tag::SYMBOL), Some(CellKind::Symbol))
    )
}
