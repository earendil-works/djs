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
//! element or a few code units can charge for them: a single call costs nothing extra
//! (the fraction is dropped when the interpreter takes the counter), a loop of them
//! adds up.
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

impl Heap {
    /// Adds `n` sixteenths of a unit of fuel.
    #[inline(always)]
    fn charge16(&self, n: u64) {
        self.work.set(self.work.get() + n);
    }

    /// Charges a scan, comparison or copy of `n` code units.
    #[inline(always)]
    pub(crate) fn charge_units(&self, n: usize) {
        self.charge16((n as u64) * (16 / UNITS as u64));
    }

    /// Charges a loop over `n` elements or keys, or a move of `n` elements.
    #[inline(always)]
    pub(crate) fn charge_elements(&self, n: usize) {
        self.charge16((n as u64) * (16 / ELEMENTS as u64));
    }

    /// Charges an allocation of `size` bytes.
    #[inline(always)]
    pub(crate) fn charge_alloc(&self, size: u32) {
        self.charge16(u64::from(size.saturating_sub(ALLOC_FREE)) * 16 / u64::from(ALLOC_BYTES));
    }

    /// The whole units of fuel owed since the last call, which clears the counter (the
    /// fraction left is dropped).
    #[inline(always)]
    pub(crate) fn take_work(&self) -> i64 {
        (self.work.replace(0) >> SHIFT) as i64
    }

    /// The counter as it is: saved around work that must charge nothing.
    #[inline(always)]
    pub(crate) fn work_mark(&self) -> u64 {
        self.work.get()
    }

    /// Puts back a [`Heap::work_mark`].
    #[inline(always)]
    pub(crate) fn work_reset(&self, mark: u64) {
        self.work.set(mark);
    }
}

/// The bytes of two strings a comparison reads for free: [`equal_blocks`]' block.
pub(crate) const FREE_COMPARE: u32 = 256;

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
