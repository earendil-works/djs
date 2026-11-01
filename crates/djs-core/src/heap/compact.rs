//! Compaction for shipped images (0043): cells grow with slack (property tables double,
//! the atom table and the value stack are sized for growth), which an image made once
//! and copied into every VM would carry along. `compact_for_image` reallocates them at
//! their used size; the next collection drops the old cells. Growth works as before
//! afterwards: a full table or array grows on the next insert.

use super::cells::{AtomTable, Object, PropEntry, PropTable, Roots, ValueArray};
use super::object::LINEAR_MAX;
use super::{CellKind, Heap, HeapError, Ref};

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
            if let Some(t) = t.get() {
                self.shrink_table(o, t)?;
            }
        }
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
