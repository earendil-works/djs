//! Compaction for shipped images (0043): cells grow with slack (property tables double,
//! the atom table and the value stack are sized for growth), which an image made once
//! and copied into every VM would carry along. `compact_for_image` reallocates them at
//! their used size; the next collection drops the old cells. Growth works as before
//! afterwards: a full table or array grows on the next insert.

use super::cells::flags::OBJ_DICTIONARY;
use super::cells::{
    AtomTable, Object, PropEntry, PropTable, PropValues, RefArray, Roots, Shape, U32Array,
    ValueArray,
};
use super::object::LINEAR_MAX;
use super::{CellKind, Heap, HeapError, Ref};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// The value stack of an image: room for a few frames; `run` grows it on demand.
const IMAGE_STACK: u32 = 256;

impl Heap {
    /// Shrinks property tables, the atom table's entries and the value stack (of an idle
    /// VM: nothing above slot 0 is live) to their used sizes. Collect afterwards.
    pub fn compact_for_image(&mut self) -> Result<(), HeapError> {
        let objects: alloc::vec::Vec<u32> = self
            .cells()
            .filter(|c| c.kind == CellKind::Object)
            .map(|c| c.offset)
            .collect();
        for o in objects {
            let t: Ref = self.arena().get(o, Object::PROPS);
            match t.get() {
                Some(t) if self.cell_flags(o) & OBJ_DICTIONARY != 0 => self.shrink_table(o, t)?,
                Some(v) => self.shrink_values(o, v)?,
                None => {}
            }
        }
        self.shrink_shapes()?;
        // The atom table's entries (slot 0 unused, then one per atom).
        let table = self.atom_table();
        let entries: Ref = self.arena().get(table, AtomTable::ENTRIES);
        let len = self.value_array_len(entries.0);
        let copy = self.copy_value_array(entries.0, len, len)?;
        self.arena_mut().set(table, AtomTable::ENTRIES, Ref(copy));
        // The value stack: its LEN is its capacity.
        let stack: Ref = self.arena().get(0, Roots::STACK);
        if let Some(s) = stack.get()
            && self.value_array_capacity(s) > IMAGE_STACK
        {
            let copy = self.copy_value_array(s, 0, IMAGE_STACK)?;
            let cap = self.value_array_capacity(copy);
            self.arena_mut().set(copy, ValueArray::LEN, cap);
            self.arena_mut().set(0, Roots::STACK, Ref(copy));
        }
        Ok(())
    }

    /// Reorders the cells of a collected image by kind (in `CellKind` order, the roots
    /// cell first), strings by their content and every other kind in its current order
    /// (0122, D2251). An image ships in the module, where gzip finds far more repeats
    /// in runs of like cells (templates next to templates, bytecode next to bytecode)
    /// than in a breadth-first walk's mix. The order is a function of the input image,
    /// so a canonical image stays canonical. Only for the base image's build: offsets
    /// change, so it updates every traced reference and, as a collection does, clears the
    /// templates' offset caches and rehashes Map/Set indexes.
    pub fn order_for_image(&mut self) -> Result<(), HeapError> {
        use super::cells::{Header, Str8};
        use super::walk::{Slot, for_each_slot};
        use crate::value::{Obj, Value};
        let cells: Vec<super::walk::CellInfo> = self.cells().collect();
        assert!(
            cells
                .first()
                .is_some_and(|c| c.offset == 0 && c.kind == CellKind::Roots),
            "order_for_image: no roots cell at 0"
        );
        let a = self.arena();
        let data = |c: &super::walk::CellInfo| -> &[u8] {
            a.slice(c.offset + Str8::DATA.offset, c.size - Str8::DATA.offset)
        };
        let is_str = |k: CellKind| matches!(k, CellKind::Str8 | CellKind::Str16);
        let mut order: Vec<usize> = (1..cells.len()).collect();
        order.sort_by(|&x, &y| {
            let (cx, cy) = (&cells[x], &cells[y]);
            (cx.kind as u8).cmp(&(cy.kind as u8)).then_with(|| {
                if is_str(cx.kind) {
                    data(cx).cmp(data(cy)).then(cx.offset.cmp(&cy.offset))
                } else {
                    cx.offset.cmp(&cy.offset)
                }
            })
        });
        order.insert(0, 0);
        // New offsets, by old cell index (cells are in address order: binary search).
        let mut new_at = alloc::vec![0u32; cells.len()];
        let mut bytes = alloc::vec![0u8; a.len() as usize];
        let mut at = 0u32;
        for &i in &order {
            let c = &cells[i];
            new_at[i] = at;
            bytes[at as usize..(at + c.size) as usize].copy_from_slice(a.slice(c.offset, c.size));
            at += c.size;
        }
        assert_eq!(at, a.len(), "order_for_image: cells do not cover the arena");
        let moved = |old: u32| -> u32 {
            match cells.binary_search_by_key(&old, |c| c.offset) {
                Ok(i) => new_at[i],
                Err(_) => panic!("order_for_image: a reference to {old} is not a cell"),
            }
        };
        let limit = a.limit() as usize;
        let mut arena = super::Arena::from_bytes(bytes, limit);
        let mut collections = Vec::new();
        for &i in &order {
            let (c, off) = (&cells[i], new_at[i]);
            if c.kind == CellKind::FuncTemplate {
                // (Before its IC ref is updated: the slots are cleared where they are.)
                super::gc::clear_offset_caches(&mut arena, off);
            } else if c.kind == CellKind::Object
                && super::ObjectClass::from_u32(arena.get(off, Object::CLASS)).is_some_and(|k| {
                    matches!(
                        k,
                        super::ObjectClass::Map
                            | super::ObjectClass::Set
                            | super::ObjectClass::WeakMap
                            | super::ObjectClass::WeakSet
                    )
                })
            {
                // Their indexes hash object keys by offset (D1381).
                collections.push(off);
            }
            for_each_slot(c.kind, off, c.size, |slot| match slot {
                Slot::Value(s) => {
                    let v: Value = arena.read(s);
                    if let Some(t) = v.ref_offset() {
                        let bits = (v.to_bits() & !0xFFFF_FFFF) | moved(t) as u64;
                        arena.write(s, Value::from_raw(bits));
                    }
                }
                Slot::Ref(s) => {
                    if let Some(t) = arena.read::<Ref>(s).get() {
                        arena.write(s, Ref(moved(t)));
                    }
                }
            });
            debug_assert_eq!(arena.get(off, Header::SIZE), c.size);
        }
        self.arena = arena;
        for o in collections {
            self.collection_rehash(Obj(o));
        }
        Ok(())
    }

    /// A new ValueArray with room for `cap` values holding the first `len` of `arr`.
    fn copy_value_array(&mut self, arr: u32, len: u32, cap: u32) -> Result<u32, HeapError> {
        let new = self.alloc_value_array(cap.max(1))?;
        if len > 0 {
            let src = ValueArray::VALUES.at(arr, 0);
            let dst = ValueArray::VALUES.at(new, 0);
            self.arena_mut().copy_within(src, len * 8, dst);
        }
        self.arena_mut().set(new, ValueArray::LEN, len);
        Ok(new)
    }

    /// Shrinks the shapes' descriptor arrays to the pairs written and their transition
    /// arrays to their children (0098). A shared descriptor array is copied once and
    /// every shape on it points at the copy; the next child then copies instead of
    /// appending in place, as for any full array.
    fn shrink_shapes(&mut self) -> Result<(), HeapError> {
        let shapes: Vec<u32> = self
            .cells()
            .filter(|c| c.kind == CellKind::Shape)
            .map(|c| c.offset)
            .collect();
        let mut copies: BTreeMap<u32, u32> = BTreeMap::new();
        for s in shapes {
            if let Some(d) = self.arena().get::<Ref>(s, Shape::DESCS).get() {
                let copy = match copies.get(&d) {
                    Some(&c) => c,
                    None => {
                        let n = 1 + 2 * self.u32_array_get(d, 0);
                        let c = if n < self.u32_array_len(d) {
                            let c = self.alloc_u32_array(n)?;
                            let (src, dst) = (U32Array::DATA.at(d, 0), U32Array::DATA.at(c, 0));
                            self.arena_mut().copy_within(src, 4 * n, dst);
                            c
                        } else {
                            d
                        };
                        copies.insert(d, c);
                        c
                    }
                };
                self.arena_mut().set(s, Shape::DESCS, Ref(copy));
            }
            if let Some(t) = self.arena().get::<Ref>(s, Shape::TRANSITIONS).get() {
                let len = self.arena().get(t, RefArray::LEN);
                if len < RefArray::LAYOUT.tail_capacity(self.cell_size(t)) {
                    let size = RefArray::REFS
                        .cell_size(len)
                        .ok_or(HeapError::OutOfMemory)?;
                    let c = self.alloc_cell(CellKind::RefArray, size)?;
                    let (src, dst) = (RefArray::REFS.at(t, 0), RefArray::REFS.at(c, 0));
                    self.arena_mut().copy_within(src, 4 * len, dst);
                    self.arena_mut().set(c, RefArray::LEN, len);
                    self.arena_mut().set(s, Shape::TRANSITIONS, Ref(c));
                }
            }
        }
        Ok(())
    }

    /// Replaces object `o`'s values block `v` by one of exactly its length.
    fn shrink_values(&mut self, o: u32, v: u32) -> Result<(), HeapError> {
        let len: u32 = self.arena().get(v, PropValues::LEN);
        if len == PropValues::LAYOUT.tail_capacity(self.cell_size(v)) {
            return Ok(());
        }
        let size = PropValues::VALUES
            .cell_size(len)
            .ok_or(HeapError::OutOfMemory)?;
        let new = self.alloc_cell(CellKind::PropValues, size)?;
        let src = PropValues::VALUES.at(v, 0);
        let dst = PropValues::VALUES.at(new, 0);
        self.arena_mut().copy_within(src, 8 * len, dst);
        let shape: Ref = self.arena().get(v, PropValues::SHAPE);
        let a = self.arena_mut();
        a.set(new, PropValues::LEN, len);
        a.set(new, PropValues::SHAPE, shape);
        a.set(o, Object::PROPS, Ref(new));
        Ok(())
    }

    /// Replaces object `o`'s table `t` by one with exactly its live entries.
    fn shrink_table(&mut self, o: u32, t: u32) -> Result<(), HeapError> {
        let count: u32 = self.arena().get(t, PropTable::COUNT);
        let deleted: u32 = self.arena().get(t, PropTable::DELETED);
        let live = count - deleted;
        if live == PropTable::LAYOUT.tail_capacity(self.cell_size(t)) {
            return Ok(());
        }
        let size = PropTable::ENTRIES
            .cell_size(live.max(1))
            .ok_or(HeapError::OutOfMemory)?;
        let new = self.alloc_cell(CellKind::PropTable, size)?;
        let mut n = 0;
        for i in 0..count {
            let src = PropTable::ENTRIES.at(t, i);
            if self.arena().get::<u32>(src, PropEntry::KEY) == 0 {
                continue;
            }
            let dst = PropTable::ENTRIES.at(new, n);
            self.arena_mut()
                .copy_within(src, PropTable::ENTRIES.elem_size(), dst);
            n += 1;
        }
        self.arena_mut().set(new, PropTable::COUNT, n);
        self.arena_mut().set(o, Object::PROPS, Ref(new));
        if n > LINEAR_MAX {
            self.build_table_index(new)?;
        }
        Ok(())
    }
}
