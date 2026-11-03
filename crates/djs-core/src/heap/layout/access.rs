//! The arena's byte access: every field read and write of the heap goes through
//! [`read`] and [`write`] (via `FieldType`, `Arena::read`/`write`/`get`/`set` and the
//! tier-0 loop's slice). Two implementations, one chosen at build time (D2520):
//!
//! - **Checked (the default, no `unsafe`).** One compare per access, `off + N <= len` in
//!   `u64` so that it cannot wrap on wasm32. With it LLVM proves the slice access that
//!   follows in range and drops its own two checks (`off <= len`, then `len - off >= N`),
//!   which is what a plain `bytes.get(off..)` costs. Out of range is a panic (a wasm
//!   trap), never undefined behavior.
//! - **Unchecked (`unsafe-heap`).** A plain load or store. Debug builds still assert the
//!   bounds, so the tests catch what the checked build would. This module is the only
//!   place in djs-core that may contain `unsafe` (lib.rs forbids it everywhere else).
//!
//! Why unchecked can be sound: every offset the engine uses comes from cells it laid out
//! itself or from an image that passed `Heap::verify`, which checks every header, length
//! and reference before anything reads through them (D1056). A violation is therefore an
//! engine or verifier bug, which the checked build turns into a trap and the unchecked
//! build into undefined behavior inside the wasm instance's own linear memory (which wasm
//! still bounds-checks). The gate runs both builds (D2520).

/// `N` bytes at `off`.
#[inline(always)]
pub(crate) fn read<const N: usize>(bytes: &[u8], off: usize) -> [u8; N] {
    #[cfg(not(feature = "unsafe-heap"))]
    {
        if off as u64 + N as u64 <= bytes.len() as u64
            && let Some(a) = bytes.get(off..).and_then(<[u8]>::first_chunk::<N>)
        {
            return *a;
        }
        super::out_of_bounds(off, N, bytes.len())
    }
    #[cfg(feature = "unsafe-heap")]
    {
        debug_assert!(
            off as u64 + N as u64 <= bytes.len() as u64,
            "arena read out of bounds"
        );
        // SAFETY: `off + N <= bytes.len()` for every offset the engine reads: offsets
        // come from its own cell layouts or from verified images (module docs, D2520).
        #[allow(unsafe_code)]
        unsafe {
            bytes.as_ptr().add(off).cast::<[u8; N]>().read_unaligned()
        }
    }
}

/// Writes `v` at `off`.
#[inline(always)]
pub(crate) fn write<const N: usize>(bytes: &mut [u8], off: usize, v: [u8; N]) {
    #[cfg(not(feature = "unsafe-heap"))]
    {
        let len = bytes.len();
        if off as u64 + N as u64 <= len as u64
            && let Some(a) = bytes.get_mut(off..).and_then(<[u8]>::first_chunk_mut::<N>)
        {
            *a = v;
            return;
        }
        super::out_of_bounds(off, N, len)
    }
    #[cfg(feature = "unsafe-heap")]
    {
        debug_assert!(
            off as u64 + N as u64 <= bytes.len() as u64,
            "arena write out of bounds"
        );
        // SAFETY: as for `read`.
        #[allow(unsafe_code)]
        unsafe {
            bytes
                .as_mut_ptr()
                .add(off)
                .cast::<[u8; N]>()
                .write_unaligned(v)
        }
    }
}
