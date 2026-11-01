//! The copying collector (Cheney).
//!
//! [`Heap::collect`] copies everything reachable from the [`Roots`] cell into a fresh
//! arena and drops the old one:
//!
//! 1. Reserve the to-space: as many bytes as the from-space holds, which bounds the live
//!    data. If the host cannot provide them, the collection fails with
//!    [`HeapError::OutOfMemory`] before anything was touched. With the space reserved,
//!    nothing during the copy can fail.
//! 2. Copy the roots cell (offset 0 → offset 0).
//! 3. Scan the to-space cell by cell. For every traced slot (from the layout table, see
//!    [`walk::for_each_slot`]) that references a from-space cell, *forward* it: if the cell
//!    has already been copied, take its new offset, else copy it to the end of the to-space.
//!    Write the new offset back into the slot. The scan ends when it catches up with the
//!    allocation point.
//!
//! Forwarding sets [`gc_bits::FORWARDED`] in the old header and stores the new offset in
//! the old cell's first payload word (every cell has one; `cells.rs` asserts it). The
//! from-space is never scanned, so overwriting it is fine.
//!
//! References to a flattened rope are short-circuited to its flat string, so flattened
//! ropes do not survive a collection. Unflattened ropes are copied as they are; flattening
//! them here would allocate and could not be bounded by the reservation.
//!
//! The resulting layout is breadth-first from the roots, in slot order. That makes it a
//! function of the object graph alone: collecting an already collected heap reproduces it
//! byte for byte, which keeps snapshots deterministic.
//!
//! Nothing in the heap is keyed by arena offsets: the atom index is keyed by string
//! content hashes, property indexes by atom ids, string hashes are content based. So no
//! table needs rehashing after a move.
//!
//! [`Roots`]: super::cells::Roots

use super::arena::Arena;
use super::cells::{Header, Rope, flags, gc_bits};
use super::layout::Ref;
use super::walk::{self, Slot};
use super::{CellKind, Heap, HeapError};
use crate::value::Value;

/// The smallest growth between collections.
pub const MIN_GROWTH: u32 = 256 * 1024;

/// Where the forwarding offset goes in a copied cell: the first payload word.
const FORWARD: u32 = 8;

/// What a collection did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GcStats {
    /// Arena size before.
    pub before: u32,
    /// Arena size after: the live data.
    pub after: u32,
}

/// The arena size at which the next collection is requested, for `live` bytes after a
/// collection: twice the live size (at least [`MIN_GROWTH`] more), but never more than half
/// of the remaining headroom below `limit`. Close to the cap, collections get more
/// frequent instead of running into `OutOfMemory`.
pub fn next_threshold(live: u32, limit: u32) -> u32 {
    let headroom = limit.saturating_sub(live);
    live + live.max(MIN_GROWTH).min(headroom / 2)
}

/// Returns the new offset of the from-space cell at `off`, copying it if needed.
fn forward(from: &mut Arena, to: &mut Arena, mut off: u32) -> u32 {
    if from.get(off, Header::KIND) == CellKind::Rope as u8
        && from.get(off, Header::FLAGS) & flags::ROPE_FLATTENED != 0
    {
        off = from.get(off, Rope::LEFT).0;
    }
    if from.get(off, Header::GC) & gc_bits::FORWARDED != 0 {
        return from.read::<u32>(off + FORWARD);
    }
    let size = from.get(off, Header::SIZE);
    let new = to
        .alloc(size)
        .expect("to-space is reserved for the whole from-space");
    to.slice_mut(new, size)
        .copy_from_slice(from.slice(off, size));
    from.set(off, Header::GC, gc_bits::FORWARDED);
    from.write::<u32>(off + FORWARD, new);
    new
}

impl Heap {
    /// Whether allocation has crossed the GC threshold. The owner should call
    /// [`Heap::collect`] at its next safepoint.
    #[inline]
    pub fn gc_pending(&self) -> bool {
        self.gc_pending
    }

    /// The arena size at which `gc_pending` will be set.
    pub fn gc_threshold(&self) -> u32 {
        self.gc_threshold
    }

    /// Collects garbage. Must only be called at a safepoint: every `Obj`, `Str`, `Sym` and
    /// `Value` held outside the heap is invalid afterwards; keep [`Handle`]s instead.
    ///
    /// On `Err(OutOfMemory)` (the host could not provide the to-space) the heap is
    /// unchanged.
    ///
    /// [`Handle`]: super::Handle
    pub fn collect(&mut self) -> Result<GcStats, HeapError> {
        let before = self.arena.len();
        #[cfg(test)]
        if core::mem::take(&mut self.fail_next_gc_reserve) {
            return Err(HeapError::OutOfMemory);
        }
        let mut to = Arena::with_capacity(before as usize, self.arena.limit() as usize)?;
        let from = &mut self.arena;
        let roots = forward(from, &mut to, 0);
        debug_assert_eq!(roots, 0);
        let mut scan = 0;
        while scan < to.len() {
            let kb = to.get(scan, Header::KIND);
            let kind = CellKind::from_u8(kb).unwrap_or_else(|| panic!("gc: bad kind {kb}"));
            let size = to.get(scan, Header::SIZE);
            walk::for_each_slot(kind, scan, size, |slot| match slot {
                Slot::Value(s) => {
                    let v: Value = to.read(s);
                    if let Some(t) = v.ref_offset() {
                        let n = forward(from, &mut to, t);
                        let bits = (v.to_bits() & !0xFFFF_FFFF) | n as u64;
                        to.write(s, Value::from_raw(bits));
                    }
                }
                Slot::Ref(s) => {
                    let r: Ref = to.read(s);
                    if let Some(t) = r.get() {
                        let n = forward(from, &mut to, t);
                        to.write(s, Ref(n));
                    }
                }
            });
            scan += size;
        }
        let after = to.len();
        self.gc_threshold = next_threshold(after, to.limit());
        to.shrink_capacity(self.gc_threshold as usize);
        self.arena = to;
        self.gc_pending = false;
        Ok(GcStats { before, after })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::arena;

    #[test]
    fn thresholds() {
        let big = arena::MAX_ARENA;
        assert_eq!(next_threshold(0, big), MIN_GROWTH);
        assert_eq!(next_threshold(1 << 20, big), 2 << 20);
        // Near the cap: half the headroom.
        assert_eq!(next_threshold(900, 1000), 950);
        assert_eq!(next_threshold(1000, 1000), 1000);
        assert_eq!(next_threshold(2000, 1000), 2000);
    }
}
