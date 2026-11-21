//! The work counter (0289): fuel owed for what leaf primitives did.
//!
//! Fuel bounds a run's CPU time, so every native operation must pay for its work, not a
//! flat amount per call: `a.join()` of a million elements, `s.indexOf(c)` over a million
//! code units or `new Set(a)` cost what they do. The primitives that do work in
//! proportion to their input (allocation, string search and comparison, element and key
//! loops, moves of element ranges) add to the counter; they have only the heap, not the
//! interpreter's fuel. The interpreter takes the counter into its fuel after every leaf
//! call and before every fuel check ([`Heap::take_work`]), so the charge is
//! deterministic: where it is taken depends only on the program, never on where a fuel
//! slice ends, on a collection or on an image.
//!
//! The counter is in sixteenths of a unit of fuel, so that a primitive that handles one
//! element or a few code units can charge for them, and a loop of them adds up. The
//! interpreter takes whole units; the fraction stays in the counter, and a run that
//! stops carries it in its fuel debt, so the total does not depend on where the
//! counter was taken or where a slice ended.
//!
//! A fast path that declines charges nothing (the general path does the work again and
//! pays for it), and a comparison or hash of a few units is free (the tier-0 loop
//! compares short strings itself): so the loop and the general path charge the same.
//!
//! Collections charge nothing (the counter is saved around them), and neither do
//! suspension records (allocated between `begin_unbudgeted` and `end_unbudgeted`): both
//! depend on the schedule. Atom interning compares strings uncharged too: whether an atom
//! is still there to compare with depends on the collections.

use super::Heap;

/// Sixteenths of a unit of fuel: the counter's unit.
const SHIFT: u32 = 4;
/// An allocation's first bytes are free: the instruction or call that allocates pays
/// for a small cell.
pub(crate) const ALLOC_FREE: u32 = 64;
/// Bytes allocated (written or zeroed) per unit of fuel, beyond [`ALLOC_FREE`].
pub(crate) const ALLOC_BYTES: u32 = 32;
/// Code units scanned or compared per unit of fuel (D1561's rate for string work).
pub(crate) const UNITS: u32 = 16;
/// Elements or keys visited or moved per unit of fuel (the dense loops' rate, D1801).
pub(crate) const ELEMENTS: u32 = 4;
/// The units of two strings a comparison or a hash reads for free: [`equal_blocks`]'
/// block.
pub(crate) const FREE_COMPARE: u32 = 256;

impl Heap {
    /// Adds `n` sixteenths of a unit of fuel. (Out of line: the charges are many and
    /// small, the code size counts.)
    #[inline(never)]
    fn charge16(&self, n: u64) {
        // (Saturating: past 2^28 units of fuel in one call the rest is forgiven; such a
        // call ran for seconds anyway.)
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        let w = u32::from_le_bytes(self.work.get()).saturating_add(n);
        self.work.set(w.to_le_bytes());
    }

    /// Charges a scan, comparison or copy of `n` code units.
    #[inline(always)]
    pub(crate) fn charge_units(&self, n: usize) {
        self.charge16((n as u64) * u64::from(16 / UNITS));
    }

    /// Charges a loop over `n` elements or keys, or a move of `n` elements.
    #[inline(always)]
    pub(crate) fn charge_elements(&self, n: usize) {
        self.charge16((n as u64) * u64::from(16 / ELEMENTS));
    }

    /// Charges an allocation of `size` bytes (a small one, the most, costs a compare).
    #[inline(always)]
    pub(crate) fn charge_alloc(&self, size: u32) {
        if size > ALLOC_FREE {
            self.charge16(u64::from(size - ALLOC_FREE) * 16 / u64::from(ALLOC_BYTES));
        }
    }

    /// The whole units of fuel owed, taken from the counter; the fraction of a unit
    /// stays, so that the sum does not depend on where the interpreter takes it. Out of
    /// line: the tier-0 loop takes it after every call into the heap.
    #[inline(never)]
    pub(crate) fn take_work(&self) -> i64 {
        let w = u32::from_le_bytes(self.work.get());
        self.work.set((w & ((1 << SHIFT) - 1)).to_le_bytes());
        i64::from(w >> SHIFT)
    }

    /// The fraction of a unit left after [`Heap::take_work`], in sixteenths; clears the
    /// counter. A run that stops carries it in its debt (D1740).
    pub(crate) fn take_fraction(&self) -> u32 {
        u32::from_le_bytes(self.work.replace([0; 4]))
    }

    /// The counter as it is: saved around work that must charge nothing.
    #[inline(always)]
    pub(crate) fn work_mark(&self) -> u32 {
        u32::from_le_bytes(self.work.get())
    }

    /// Puts back a [`Heap::work_mark`].
    #[inline(always)]
    pub(crate) fn work_reset(&self, mark: u32) {
        self.work.set(mark.to_le_bytes());
    }
}

/// The length of the equal prefix of `x` and `y` in whole blocks: what a comparison of
/// the two is charged for. The caller compares the rest, where a difference is within
/// the first block (or the shorter string ends), so it costs a constant at most.
#[inline]
pub(crate) fn equal_blocks(x: &[u8], y: &[u8]) -> usize {
    const BLOCK: usize = FREE_COMPARE as usize;
    let n = x.len().min(y.len());
    let mut i = 0;
    while i + BLOCK <= n && x[i..i + BLOCK] == y[i..i + BLOCK] {
        i += BLOCK;
    }
    i
}
