//! Walking the arena cell by cell, tracing cells, and verifying a heap.
//!
//! Tracing is driven entirely by the layout table: a field or tail element field of type
//! `Value` or `Ref` is a slot the GC must visit. This is the groundwork for the copying GC
//! (a separate issue), and [`Heap::verify`] is what restoring an untrusted image runs.

use alloc::vec::Vec;

use super::cells::{Header, Str8, Str16, U32Array, ValueArray};
use super::layout::{FieldTy, Ref};
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
