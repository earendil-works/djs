//! The in-place mark-compact collector (0101, docs/mark-compact.md): the VM's own
//! collections, without a to-space.
//!
//! Four passes over the arena:
//! 1. **Mark** from the roots cell. Marking sets `gc_bits::MARK` in the header and the
//!    cell's words in a live-word bitmap (one bit per 8 bytes). The mark stack is
//!    bounded; on overflow a cell is marked but not pushed, and a walk over the marked
//!    cells rescans them afterwards. References to a flattened rope mark its flat string
//!    instead (Cheney's short-circuit). Ephemeron tables are marked but not traced: their
//!    pairs are resolved to a fixpoint, then the pairs whose key died are cleared.
//! 2. **Block table:** the new address of each 512-byte block's first live word, a prefix
//!    sum over the bitmap. A live cell at `a` moves to `table[a / 512]` plus 8 times the
//!    live words before it in its block (one popcount). Nothing is written into cells.
//! 3. **Update** every traced slot of every marked cell to its target's new address,
//!    and empty the offset slots of inline caches (D1621), as Cheney does.
//! 4. **Slide** the marked cells down in address order, clear their marks, and zero the
//!    freed tail. Map/Set indexes are rehashed afterwards (D1381).
//!
//! Only the *set* of marked cells reaches the result, so the mark order (stack,
//! overflow rescans, the ephemeron fixpoint) cannot make it nondeterministic, and both
//! collectors keep the same cells at the same sizes: live bytes, thresholds and GC points
//! do not depend on which one runs. The side tables (len/64 + len/128 bytes and a fixed
//! stack) are allocated before anything is touched: a refusal leaves the heap as it was.

use alloc::vec::Vec;

use super::arena::Arena;
use super::cells::{EphemeronArray, Header, Object, Rope, flags, gc_bits};
use super::layout::Ref;
use super::object::ObjectClass;
use super::walk::{self, Slot};
use super::{CellKind, Heap, HeapError};
use crate::value::{Obj, Value};

/// Mark stack entries reserved up front (64 KiB). The stack grows on demand up to
/// len/64 entries (len/16 bytes, for deep structures like long lists); past that, or when
/// the host refuses, cells are marked without being pushed and found by a rescan.
const STACK_CAP: usize = 16 * 1024;

#[cfg(test)]
extern crate std;

#[cfg(test)]
std::thread_local! {
    /// Test hook: a smaller mark stack, so the overflow path runs (0 = `STACK_CAP`).
    pub(crate) static TEST_STACK_CAP: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

/// The largest mark stack (entries) for an arena of `len` bytes.
fn stack_cap(len: u32) -> usize {
    #[cfg(test)]
    {
        let c = TEST_STACK_CAP.with(|c| c.get());
        if c != 0 {
            return c;
        }
    }
    (len as usize / 64).max(STACK_CAP)
}

/// Bytes per block of the forwarding table: one `u64` of the bitmap.
const BLOCK: u32 = 512;

struct Marker<'a> {
    arena: &'a mut Arena,
    /// The end of the old region: 0 for a full collection. Cells below it are live and
    /// are neither marked nor moved.
    old: u32,
    /// One bit per 8-byte word of a marked cell, from `old`.
    live: Live,
    stack: Vec<u32>,
    overflow: bool,
    /// The lowest cell marked but not pushed since the last rescan.
    overflow_from: u32,
    /// The largest stack (entries) before overflowing.
    cap: usize,
    /// Marked ephemeron tables (their pairs need the fixpoint).
    ephemerons: u32,
    /// Ephemeron tables in the old region (found by the old walk).
    old_ephemerons: u32,
}

impl Marker<'_> {
    /// Whether the cell at `off` is live: old, or marked.
    #[inline]
    fn is_marked(&self, off: u32) -> bool {
        off < self.old || self.arena.get(off, Header::GC) & gc_bits::MARK != 0
    }

    /// The cell a reference to `off` keeps alive: a flattened rope stands for its flat
    /// string (an old one stands for itself: it is not resolved, see `mark`).
    #[inline]
    fn resolve(&self, off: u32) -> u32 {
        let arena = &*self.arena;
        if off >= self.old
            && arena.get(off, Header::KIND) == CellKind::Rope as u8
            && arena.get(off, Header::FLAGS) & flags::ROPE_FLATTENED != 0
        {
            arena.get(off, Rope::LEFT).0
        } else {
            off
        }
    }

    /// Marks the cell a reference to `off` keeps alive and returns it: `off`, or the
    /// flat string of a flattened rope (the caller rewrites its slot, so the update pass
    /// needs no rope check).
    fn mark(&mut self, mut off: u32) -> u32 {
        if off < self.old {
            // Old: live, not marked, not moved (an old flattened rope stays, and keeps its
            // flat string through its own slot, which the old walk traces).
            return off;
        }
        // The whole header in one read: kind, gc bits, flags, size.
        let mut h: u64 = self.arena.read(off);
        if h as u8 == CellKind::Rope as u8 && (h >> 16) as u16 & flags::ROPE_FLATTENED != 0 {
            off = self.arena.get(off, Rope::LEFT).0;
            if off < self.old {
                return off;
            }
            h = self.arena.read(off);
        }
        let gc = (h >> 8) as u8;
        if gc & gc_bits::MARK != 0 {
            return off;
        }
        self.arena.set(off, Header::GC, gc | gc_bits::MARK);
        self.live.set(off, (h >> 32) as u32);
        let kind = h as u8;
        if kind == CellKind::EphemeronArray as u8 {
            // A pair lives only through its key (the fixpoint below).
            self.ephemerons += 1;
            return off;
        }
        if is_leaf(kind) {
            // No traced slots: nothing to push.
            return off;
        }
        // The stack grows on demand (a refusal is only an overflow, handled by a rescan).
        if self.stack.len() == self.stack.capacity()
            && (self.stack.len() >= self.cap || self.stack.try_reserve(self.stack.len()).is_err())
        {
            self.overflow = true;
            self.overflow_from = self.overflow_from.min(off);
            return off;
        }
        self.stack.push(off);
        off
    }

    /// Marks the target of the traced slot `s`, rewriting a reference to a flattened
    /// rope to its flat string.
    #[inline]
    fn mark_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Value(s) => {
                let v: Value = self.arena.read(s);
                if let Some(t) = v.ref_offset() {
                    let n = self.mark(t);
                    if n != t {
                        let bits = (v.to_bits() & !0xFFFF_FFFF) | n as u64;
                        self.arena.write(s, Value::from_raw(bits));
                    }
                }
            }
            Slot::Ref(s) => {
                if let Some(t) = self.arena.read::<Ref>(s).get() {
                    let n = self.mark(t);
                    if n != t {
                        self.arena.write(s, Ref(n));
                    }
                }
            }
        }
    }

    /// Marks the targets of `cell`'s traced slots.
    fn trace(&mut self, cell: u32) {
        let kb = self.arena.get(cell, Header::KIND);
        let kind = CellKind::from_u8(kb).unwrap_or_else(|| super::fail::bad_cell_kind(kb, cell));
        if kind == CellKind::EphemeronArray {
            return;
        }
        let size = self.arena.get(cell, Header::SIZE);
        walk::for_each_slot(kind, cell, size, |slot| self.mark_slot(slot));
    }

    fn drain(&mut self) {
        while let Some(c) = self.stack.pop() {
            self.trace(c);
        }
    }

    /// The old walk of a minor collection: every traced slot of every old cell is a root
    /// for the young cells it references (no write barrier: the walk reads them all). Old
    /// ephemeron tables are counted for the fixpoint instead, and old collections with
    /// young keys are flagged for a rehash.
    fn mark_old(&mut self) {
        let mut c = 0;
        while c < self.old {
            let h: u64 = self.arena.read(c);
            let (kb, size) = (h as u8, (h >> 32) as u32);
            if kb == CellKind::EphemeronArray as u8 {
                self.old_ephemerons += 1;
            } else if !is_leaf(kb) {
                let kind =
                    CellKind::from_u8(kb).unwrap_or_else(|| super::fail::bad_cell_kind(kb, c));
                if kind == CellKind::Object
                    && is_collection(self.arena, c)
                    && super::collection::has_young_keys(self.arena, c, self.old)
                {
                    let gc = (h >> 8) as u8;
                    self.arena.set(c, Header::GC, gc | gc_bits::REHASH);
                }
                walk::for_each_slot(kind, c, size, |slot| self.mark_slot(slot));
                self.drain();
            }
            c += size;
        }
    }

    /// The first ephemeron table to resolve at or after `from`, in address order: old
    /// ones (by a walk over the old region, only if it has any), then marked young ones.
    fn next_ephemeron(&self, mut from: u32) -> Option<u32> {
        if from < self.old {
            if self.old_ephemerons > 0 {
                while from < self.old {
                    if self.arena.get(from, Header::KIND) == CellKind::EphemeronArray as u8 {
                        return Some(from);
                    }
                    from += self.arena.get(from, Header::SIZE);
                }
            }
            from = self.old;
        }
        let mut at = self.live.next_live(from);
        while let Some(c) = at {
            if self.arena.get(c, Header::KIND) == CellKind::EphemeronArray as u8 {
                return Some(c);
            }
            at = self.live.next_live(c + self.arena.get(c, Header::SIZE));
        }
        None
    }

    /// Marks everything reachable: the stack, overflow rescans and the ephemeron
    /// fixpoint, until nothing new is marked.
    fn mark_all(&mut self) {
        loop {
            self.drain();
            if self.overflow {
                self.overflow = false;
                // Cells below the first one left unpushed were all traced.
                let from = core::mem::replace(&mut self.overflow_from, u32::MAX);
                let mut at = self.live.next_live(from);
                while let Some(c) = at {
                    self.trace(c);
                    self.drain();
                    at = self.live.next_live(c + self.arena.get(c, Header::SIZE));
                }
                continue;
            }
            if self.ephemerons + self.old_ephemerons == 0 || !self.ephemeron_round() {
                break;
            }
        }
    }

    /// One pass over the marked ephemeron tables: marks the value of every pair whose
    /// key is marked. True if that marked anything.
    fn ephemeron_round(&mut self) -> bool {
        let mut progress = false;
        let mut at = self.next_ephemeron(0);
        while let Some(c) = at {
            let len = self.arena.get(c, EphemeronArray::LEN);
            for i in (0..len).step_by(2) {
                let k: Value = self.arena.read(EphemeronArray::VALUES.at(c, i));
                let Some(k) = k.ref_offset() else { continue };
                if !self.is_marked(k) {
                    continue;
                }
                let vs = EphemeronArray::VALUES.at(c, i + 1);
                let v: Value = self.arena.read(vs);
                if let Some(t) = v.ref_offset() {
                    let fresh = !self.is_marked(self.resolve(t));
                    self.mark_slot(Slot::Value(vs));
                    progress |= fresh;
                }
            }
            at = self.next_ephemeron(c + self.arena.get(c, Header::SIZE));
        }
        progress
    }

    /// Clears the pairs whose key died (HOLE key, undefined value), as Cheney does.
    fn clear_dead_pairs(&mut self) {
        let mut at = self.next_ephemeron(0);
        while let Some(c) = at {
            let len = self.arena.get(c, EphemeronArray::LEN);
            for i in (0..len).step_by(2) {
                let ks = EphemeronArray::VALUES.at(c, i);
                let k: Value = self.arena.read(ks);
                if k.ref_offset().is_some_and(|k| !self.is_marked(k)) {
                    self.arena.write(ks, Value::HOLE);
                    self.arena.write(ks + 8, Value::UNDEFINED);
                }
            }
            at = self.next_ephemeron(c + self.arena.get(c, Header::SIZE));
        }
    }
}

/// Kinds without traced slots.
#[inline]
fn is_leaf(kind: u8) -> bool {
    matches!(
        CellKind::from_u8(kind),
        Some(
            CellKind::Str8
                | CellKind::Str16
                | CellKind::Bytes
                | CellKind::U32Array
                | CellKind::BigInt
        )
    )
}

/// Whether the object cell `c` is a Map, Set, WeakMap or WeakSet (offset-hashed keys).
fn is_collection(arena: &Arena, c: u32) -> bool {
    ObjectClass::from_u32(arena.get(c, Object::CLASS)).is_some_and(|k| {
        matches!(
            k,
            ObjectClass::Map | ObjectClass::Set | ObjectClass::WeakMap | ObjectClass::WeakSet
        )
    })
}

/// The live-word bitmap of the cells from `base` (the old region's end) on: one bit per
/// 8-byte word.
struct Live {
    base: u32,
    bits: Vec<u64>,
}

impl Live {
    /// Marks the `size` bytes of the cell at `off`.
    fn set(&mut self, off: u32, size: u32) {
        set_bits(&mut self.bits, (off - self.base) / 8, size / 8);
    }

    /// The first live word at or after `from` (`>= base`): the next marked cell when
    /// `from` is a cell boundary.
    fn next_live(&self, from: u32) -> Option<u32> {
        next_live(&self.bits, from - self.base).map(|r| r + self.base)
    }

    /// The first dead word at or after `from` (`u32::MAX` if none).
    fn next_dead(&self, from: u32) -> u32 {
        match next_dead(&self.bits, from - self.base) {
            u32::MAX => u32::MAX,
            r => r + self.base,
        }
    }

    /// The new address of the live cell at `a`, from the block table.
    #[inline]
    fn forward(&self, table: &[u32], a: u32) -> u32 {
        let r = a - self.base;
        let b = (r / BLOCK) as usize;
        let below = self.bits[b] & ((1u64 << ((r / 8) % 64)) - 1);
        table[b] + 8 * below.count_ones()
    }
}

/// Sets `n` bits from bit `first`.
fn set_bits(bits: &mut [u64], first: u32, n: u32) {
    let (mut i, end) = (first as usize, (first + n) as usize);
    while i < end {
        let w = i / 64;
        let lo = i % 64;
        let hi = (end - w * 64).min(64);
        let mask = if hi - lo == 64 {
            u64::MAX
        } else {
            ((1u64 << (hi - lo)) - 1) << lo
        };
        bits[w] |= mask;
        i = w * 64 + hi;
    }
}

/// The offset of the first live word at or after `from`: the next marked cell when
/// `from` is a cell boundary.
fn next_live(bits: &[u64], from: u32) -> Option<u32> {
    let i = (from / 8) as usize;
    let mut w = i / 64;
    if w >= bits.len() {
        return None;
    }
    let mut word = bits[w] & (u64::MAX << (i % 64));
    loop {
        if word != 0 {
            return Some(((w * 64) as u32 + word.trailing_zeros()) * 8);
        }
        w += 1;
        if w >= bits.len() {
            return None;
        }
        word = bits[w];
    }
}

/// The offset of the first dead word at or after `from` (`u32::MAX` if none).
fn next_dead(bits: &[u64], from: u32) -> u32 {
    let i = (from / 8) as usize;
    let mut w = i / 64;
    if w >= bits.len() {
        return u32::MAX;
    }
    let mut word = !bits[w] & (u64::MAX << (i % 64));
    loop {
        if word != 0 {
            return ((w * 64) as u32 + word.trailing_zeros()) * 8;
        }
        w += 1;
        if w >= bits.len() {
            return u32::MAX;
        }
        word = !bits[w];
    }
}

impl Heap {
    /// The in-place collection (see the module docs and [`Heap::collect`]) of the cells
    /// from `old` on: 0 for a full collection, the old region's end for a minor one, where
    /// the cells below `old` count as live and stay where they are.
    pub(super) fn mark_compact(&mut self, old: u32) -> Result<(u32, u32), HeapError> {
        let before = self.arena.len();
        debug_assert!(old <= before);
        let blocks = (before - old).div_ceil(BLOCK) as usize;
        let mut live: Vec<u64> = Vec::new();
        let mut table: Vec<u32> = Vec::new();
        let mut stack: Vec<u32> = Vec::new();
        live.try_reserve_exact(blocks)
            .and_then(|()| table.try_reserve_exact(blocks))
            .and_then(|()| stack.try_reserve_exact(STACK_CAP.min(stack_cap(before))))
            .map_err(|_| HeapError::HostOutOfMemory)?;
        live.resize(blocks, 0);
        // 1. Mark: from the roots cell, or (minor) from every old cell.
        let mut m = Marker {
            arena: &mut self.arena,
            old,
            live: Live {
                base: old,
                bits: live,
            },
            stack,
            overflow: false,
            overflow_from: u32::MAX,
            cap: stack_cap(before),
            ephemerons: 0,
            old_ephemerons: 0,
        };
        if old == 0 {
            m.mark(0);
        } else {
            m.mark_old();
        }
        m.mark_all();
        if m.ephemerons + m.old_ephemerons > 0 {
            m.clear_dead_pairs();
        }
        let live = m.live;
        // Reused to remember the Map/Set objects (new offsets) for the rehash.
        let mut collections = m.stack;
        let mut all_collections = true;
        // 2. Block table.
        let mut sum = old;
        for &w in &live.bits {
            table.push(sum);
            sum += 8 * w.count_ones();
        }
        let after = sum;
        // Nothing below the first dead word moves (usually the base image's cells).
        let dense = live.next_dead(old).min(before);
        // 3. Update every traced slot of the old and the marked cells.
        let arena = &mut self.arena;
        let update = |arena: &mut Arena, kind: CellKind, c: u32, size: u32| {
            walk::for_each_slot(kind, c, size, |slot| match slot {
                Slot::Value(s) => {
                    let v: Value = arena.read(s);
                    if let Some(t) = v.ref_offset()
                        && t >= dense
                    {
                        let n = live.forward(&table, t);
                        arena.write(s, Value::from_raw((v.to_bits() & !0xFFFF_FFFF) | n as u64));
                    }
                }
                Slot::Ref(s) => {
                    if let Some(t) = arena.read::<Ref>(s).get()
                        && t >= dense
                    {
                        arena.write(s, Ref(live.forward(&table, t)));
                    }
                }
            });
        };
        // The old region: references to young cells (any of its cells may hold one).
        let mut c = 0;
        while c < old {
            let h: u64 = arena.read(c);
            let (kb, size) = (h as u8, (h >> 32) as u32);
            if !is_leaf(kb) {
                let kind =
                    CellKind::from_u8(kb).unwrap_or_else(|| super::fail::bad_cell_kind(kb, c));
                if kind == CellKind::FuncTemplate {
                    // Offsets of young shapes and prototypes may move (D1621).
                    super::gc::clear_offset_caches(arena, c);
                }
                let gc = (h >> 8) as u8;
                if gc & gc_bits::REHASH != 0 {
                    arena.set(c, Header::GC, gc & !gc_bits::REHASH);
                    if collections.len() < collections.capacity() {
                        collections.push(c);
                    } else {
                        all_collections = false;
                    }
                }
                update(arena, kind, c, size);
            }
            c += size;
        }
        let mut at = live.next_live(old);
        while let Some(c) = at {
            let kb = arena.get(c, Header::KIND);
            let kind = CellKind::from_u8(kb).unwrap_or_else(|| super::fail::bad_cell_kind(kb, c));
            let size = arena.get(c, Header::SIZE);
            // The mark is not needed past this point (the bitmap has it).
            let gc = arena.get(c, Header::GC);
            arena.set(c, Header::GC, gc & !gc_bits::MARK);
            if kind == CellKind::FuncTemplate {
                // Before its IC ref is updated: the slots are cleared where they are now.
                super::gc::clear_offset_caches(arena, c);
            } else if kind == CellKind::Object && is_collection(arena, c) {
                if collections.len() < collections.capacity() {
                    collections.push(live.forward(&table, c));
                } else {
                    all_collections = false;
                }
            }
            // Marking rewrote references to flattened ropes: every target is marked.
            update(arena, kind, c, size);
            at = live.next_live(c + size);
        }
        // 4. Slide down in address order (never onto a cell not yet moved), a run of
        // adjacent live cells at a time: they all move by the same distance.
        let mut at = live.next_live(old);
        while let Some(c) = at {
            let end = live.next_dead(c).min(before);
            let to = live.forward(&table, c);
            if to != c {
                arena.copy_within(c, end - c, to);
            }
            at = live.next_live(end);
        }
        arena.truncate(after);
        // Map/Set indexes hash object keys by offset (D1381): the ones remembered, or (if
        // there were more than the stack holds) a walk over the whole heap.
        if all_collections {
            for &c in &collections {
                self.collection_rehash(Obj(c));
            }
        } else {
            let mut c = 0;
            while c < after {
                let size = self.arena.get(c, Header::SIZE);
                if self.arena.get(c, Header::KIND) == CellKind::Object as u8
                    && is_collection(&self.arena, c)
                {
                    self.collection_rehash(Obj(c));
                }
                c += size;
            }
        }
        Ok((before, after))
    }
}
