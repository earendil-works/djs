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
    /// One bit per 8-byte word of a marked cell.
    live: Vec<u64>,
    stack: Vec<u32>,
    overflow: bool,
    /// The lowest cell marked but not pushed since the last rescan.
    overflow_from: u32,
    /// The largest stack (entries) before overflowing.
    cap: usize,
    /// Marked ephemeron tables (their pairs need the fixpoint).
    ephemerons: u32,
}

impl Marker<'_> {
    #[inline]
    fn is_marked(&self, off: u32) -> bool {
        self.arena.get(off, Header::GC) & gc_bits::MARK != 0
    }

    /// The cell a reference to `off` keeps alive: a flattened rope stands for its flat
    /// string.
    #[inline]
    fn resolve(arena: &Arena, off: u32) -> u32 {
        if arena.get(off, Header::KIND) == CellKind::Rope as u8
            && arena.get(off, Header::FLAGS) & flags::ROPE_FLATTENED != 0
        {
            arena.get(off, Rope::LEFT).0
        } else {
            off
        }
    }

    fn mark(&mut self, off: u32) {
        let off = Self::resolve(self.arena, off);
        if self.is_marked(off) {
            return;
        }
        let gc = self.arena.get(off, Header::GC);
        self.arena.set(off, Header::GC, gc | gc_bits::MARK);
        let size = self.arena.get(off, Header::SIZE);
        set_bits(&mut self.live, off / 8, size / 8);
        let kind = self.arena.get(off, Header::KIND);
        if kind == CellKind::EphemeronArray as u8 {
            // A pair lives only through its key (the fixpoint below).
            self.ephemerons += 1;
            return;
        }
        if matches!(
            CellKind::from_u8(kind),
            Some(
                CellKind::Str8
                    | CellKind::Str16
                    | CellKind::Bytes
                    | CellKind::U32Array
                    | CellKind::BigInt
            )
        ) {
            // No traced slots: nothing to push.
            return;
        }
        // The stack grows on demand (a refusal is only an overflow, handled by a rescan).
        if self.stack.len() == self.stack.capacity()
            && (self.stack.len() >= self.cap || self.stack.try_reserve(self.stack.len()).is_err())
        {
            self.overflow = true;
            self.overflow_from = self.overflow_from.min(off);
            return;
        }
        self.stack.push(off);
    }

    /// Marks the targets of `cell`'s traced slots.
    fn trace(&mut self, cell: u32) {
        let kb = self.arena.get(cell, Header::KIND);
        let kind = CellKind::from_u8(kb).unwrap_or_else(|| super::fail::bad_cell_kind(kb, cell));
        if kind == CellKind::EphemeronArray {
            return;
        }
        let size = self.arena.get(cell, Header::SIZE);
        walk::for_each_slot(kind, cell, size, |slot| {
            let t = match slot {
                Slot::Value(s) => self.arena.read::<Value>(s).ref_offset(),
                Slot::Ref(s) => self.arena.read::<Ref>(s).get(),
            };
            if let Some(t) = t {
                self.mark(t);
            }
        });
    }

    fn drain(&mut self) {
        while let Some(c) = self.stack.pop() {
            self.trace(c);
        }
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
                let mut at = next_live(&self.live, from);
                while let Some(c) = at {
                    self.trace(c);
                    self.drain();
                    at = next_live(&self.live, c + self.arena.get(c, Header::SIZE));
                }
                continue;
            }
            if self.ephemerons == 0 || !self.ephemeron_round() {
                break;
            }
        }
    }

    /// One pass over the marked ephemeron tables: marks the value of every pair whose
    /// key is marked. True if that marked anything.
    fn ephemeron_round(&mut self) -> bool {
        let mut progress = false;
        let mut at = next_live(&self.live, 0);
        while let Some(c) = at {
            let size = self.arena.get(c, Header::SIZE);
            if self.arena.get(c, Header::KIND) == CellKind::EphemeronArray as u8 {
                let len = self.arena.get(c, EphemeronArray::LEN);
                for i in (0..len).step_by(2) {
                    let k: Value = self.arena.read(EphemeronArray::VALUES.at(c, i));
                    let Some(k) = k.ref_offset() else { continue };
                    if !self.is_marked(k) {
                        continue;
                    }
                    let v: Value = self.arena.read(EphemeronArray::VALUES.at(c, i + 1));
                    if let Some(t) = v.ref_offset()
                        && !self.is_marked(Self::resolve(self.arena, t))
                    {
                        self.mark(t);
                        progress = true;
                    }
                }
            }
            at = next_live(&self.live, c + size);
        }
        progress
    }

    /// Clears the pairs whose key died (HOLE key, undefined value), as Cheney does.
    fn clear_dead_pairs(&mut self) {
        let mut at = next_live(&self.live, 0);
        while let Some(c) = at {
            let size = self.arena.get(c, Header::SIZE);
            if self.arena.get(c, Header::KIND) == CellKind::EphemeronArray as u8 {
                let len = self.arena.get(c, EphemeronArray::LEN);
                for i in (0..len).step_by(2) {
                    let ks = EphemeronArray::VALUES.at(c, i);
                    let k: Value = self.arena.read(ks);
                    if k.ref_offset().is_some_and(|k| !self.is_marked(k)) {
                        self.arena.write(ks, Value::HOLE);
                        self.arena.write(ks + 8, Value::UNDEFINED);
                    }
                }
            }
            at = next_live(&self.live, c + size);
        }
    }
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

/// The new address of the live cell at `a`.
#[inline]
fn forward(live: &[u64], table: &[u32], a: u32) -> u32 {
    let b = (a / BLOCK) as usize;
    let below = live[b] & ((1u64 << ((a / 8) % 64)) - 1);
    table[b] + 8 * below.count_ones()
}

impl Heap {
    /// The in-place collection (see the module docs and [`Heap::collect`]).
    pub(super) fn mark_compact(&mut self) -> Result<(u32, u32), HeapError> {
        let before = self.arena.len();
        let blocks = before.div_ceil(BLOCK) as usize;
        let mut live: Vec<u64> = Vec::new();
        let mut table: Vec<u32> = Vec::new();
        let mut stack: Vec<u32> = Vec::new();
        live.try_reserve_exact(blocks)
            .and_then(|()| table.try_reserve_exact(blocks))
            .and_then(|()| stack.try_reserve_exact(STACK_CAP.min(stack_cap(before))))
            .map_err(|_| HeapError::HostOutOfMemory)?;
        live.resize(blocks, 0);
        // 1. Mark.
        let mut m = Marker {
            arena: &mut self.arena,
            live,
            stack,
            overflow: false,
            overflow_from: u32::MAX,
            cap: stack_cap(before),
            ephemerons: 0,
        };
        m.mark(0);
        m.mark_all();
        if m.ephemerons > 0 {
            m.clear_dead_pairs();
        }
        let live = m.live;
        // Reused to remember the Map/Set objects (new offsets) for the rehash.
        let mut collections = m.stack;
        let mut all_collections = true;
        // 2. Block table.
        let mut sum = 0;
        for &w in &live {
            table.push(sum);
            sum += 8 * w.count_ones();
        }
        let after = sum;
        // 3. Update every traced slot of the marked cells.
        let arena = &mut self.arena;
        let mut at = next_live(&live, 0);
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
                    collections.push(forward(&live, &table, c));
                } else {
                    all_collections = false;
                }
            }
            walk::for_each_slot(kind, c, size, |slot| match slot {
                Slot::Value(s) => {
                    let v: Value = arena.read(s);
                    if let Some(t) = v.ref_offset() {
                        let n = forward(&live, &table, Marker::resolve(arena, t));
                        arena.write(s, Value::from_raw((v.to_bits() & !0xFFFF_FFFF) | n as u64));
                    }
                }
                Slot::Ref(s) => {
                    if let Some(t) = arena.read::<Ref>(s).get() {
                        let n = forward(&live, &table, Marker::resolve(arena, t));
                        arena.write(s, Ref(n));
                    }
                }
            });
            at = next_live(&live, c + size);
        }
        // 4. Slide down in address order (never onto a cell not yet moved).
        let mut at = next_live(&live, 0);
        while let Some(c) = at {
            let size = arena.get(c, Header::SIZE);
            let to = forward(&live, &table, c);
            if to != c {
                arena.copy_within(c, size, to);
            }
            at = next_live(&live, c + size);
        }
        arena.truncate(after);
        // Map/Set indexes hash object keys by offset (D1381): the ones remembered, or (if
        // there were more than the stack holds) a walk over the dense heap.
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
