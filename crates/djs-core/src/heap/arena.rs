//! The arena: one growable little-endian byte buffer addressed by `u32` offsets.
//!
//! All access is bounds checked through slice indexing, so a bad offset panics (a wasm
//! trap) but never causes undefined behavior. Bulk operations take one slice up front so
//! the per-byte checks can be hoisted by the compiler.

use alloc::vec::Vec;

use super::HeapError;
use super::layout::{Field, FieldType};

/// The largest arena size: offsets must fit in a `u32`, and cell sizes are multiples of 8.
pub const MAX_ARENA: u32 = !7;

/// A growable byte buffer with a memory cap. Allocation only bumps; it never collects.
pub struct Arena {
    bytes: Vec<u8>,
    limit: u32,
}

impl Arena {
    /// An empty arena that may grow to `limit` bytes (clamped to [`MAX_ARENA`]).
    pub fn new(limit: usize) -> Arena {
        Arena {
            bytes: Vec::new(),
            limit: clamp_limit(limit),
        }
    }

    /// An empty arena with `capacity` bytes reserved up front, so allocations up to that
    /// total never touch the host allocator (the GC's to-space).
    pub fn with_capacity(capacity: usize, limit: usize) -> Result<Arena, HeapError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| HeapError::OutOfMemory)?;
        Ok(Arena {
            bytes,
            limit: clamp_limit(limit),
        })
    }

    /// Bytes reserved from the host.
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Returns reserved memory beyond `keep` bytes (at least the length) to the host.
    pub fn shrink_capacity(&mut self, keep: usize) {
        self.bytes.shrink_to(keep.max(self.bytes.len()));
    }

    /// An arena over existing bytes (for restoring a heap image). The caller validates the
    /// contents.
    pub fn from_bytes(bytes: Vec<u8>, limit: usize) -> Arena {
        Arena {
            bytes,
            limit: clamp_limit(limit),
        }
    }

    /// Bytes in use. This is also the offset of the next allocation.
    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.bytes.len() as u32
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The memory cap in bytes.
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// All bytes in use.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Allocates `size` zeroed bytes (rounded up to 8) at the end and returns their offset.
    /// Fails with [`HeapError::OutOfMemory`] when the cap would be exceeded or the host
    /// allocator fails. Never panics on exhaustion.
    pub fn alloc(&mut self, size: u32) -> Result<u32, HeapError> {
        let (start, end) = self.reserve(size)?;
        self.extend_zeros(end - start);
        Ok(start)
    }

    /// Like [`Arena::alloc`], but the `data.len()` bytes at offset `at` of the new block
    /// are `data` instead of zeros: they are written once, not zeroed and then copied (the
    /// bulk of strings and GC copies). Panics if `data` does not fit in `size` at `at`.
    pub fn alloc_init(&mut self, size: u32, at: u32, data: &[u8]) -> Result<u32, HeapError> {
        let tail = (size as usize).checked_sub(at as usize + data.len());
        assert!(tail.is_some(), "alloc_init: data does not fit");
        let (start, end) = self.reserve(size)?;
        self.extend_zeros(at);
        self.bytes.extend_from_slice(data);
        self.extend_zeros(end - self.len());
        Ok(start)
    }

    /// Checks the cap and reserves room for `size` bytes (rounded up to 8) at the end;
    /// returns the block's start and end.
    #[inline]
    fn reserve(&mut self, size: u32) -> Result<(u32, u32), HeapError> {
        let size = size.checked_add(7).ok_or(HeapError::OutOfMemory)? & !7;
        let start = self.len();
        let end = start
            .checked_add(size)
            .filter(|&e| e <= self.limit)
            .ok_or(HeapError::OutOfMemory)?;
        let end_us = end as usize;
        if end_us > self.bytes.capacity() {
            // Amortized doubling, but never past the cap.
            let want = (self.bytes.capacity() * 2)
                .max(end_us)
                .max(4096)
                .min(self.limit as usize);
            self.bytes
                .try_reserve_exact(want - self.bytes.len())
                .map_err(|_| HeapError::OutOfMemory)?;
        }
        Ok((start, end))
    }

    /// Appends `n` zero bytes (room is reserved). Copies from a static zero block rather
    /// than `resize(_, 0)`, which opt-level "z" compiles to a byte-at-a-time loop; a copy
    /// is one `memory.copy` in wasm at every opt-level (D1261).
    #[inline]
    fn extend_zeros(&mut self, mut n: u32) {
        static ZEROS: [u8; 256] = [0; 256];
        while n > 0 {
            let k = n.min(ZEROS.len() as u32);
            self.bytes.extend_from_slice(&ZEROS[..k as usize]);
            n -= k;
        }
    }

    #[inline(always)]
    pub fn read<T: FieldType>(&self, off: u32) -> T {
        T::read(&self.bytes, off as usize)
    }

    #[inline(always)]
    pub fn write<T: FieldType>(&mut self, off: u32, v: T) {
        T::write(&mut self.bytes, off as usize, v)
    }

    /// Reads field `f` of the cell (or record) at `base`.
    #[inline(always)]
    pub fn get<T: FieldType>(&self, base: u32, f: Field<T>) -> T {
        self.read(base + f.offset)
    }

    /// Writes field `f` of the cell (or record) at `base`.
    #[inline(always)]
    pub fn set<T: FieldType>(&mut self, base: u32, f: Field<T>, v: T) {
        self.write(base + f.offset, v)
    }

    #[inline]
    pub fn slice(&self, off: u32, len: u32) -> &[u8] {
        &self.bytes[off as usize..off as usize + len as usize]
    }

    #[inline]
    pub fn slice_mut(&mut self, off: u32, len: u32) -> &mut [u8] {
        &mut self.bytes[off as usize..off as usize + len as usize]
    }

    /// Splits the arena at `at`: everything before it read-only, everything from it on
    /// writable. Used to copy from older cells into a freshly allocated one.
    #[inline]
    pub fn split_mut(&mut self, at: u32) -> (&[u8], &mut [u8]) {
        let (a, b) = self.bytes.split_at_mut(at as usize);
        (a, b)
    }

    /// Copies `len` bytes from `src` to `dst` (the ranges may overlap).
    #[inline]
    pub fn copy_within(&mut self, src: u32, len: u32, dst: u32) {
        let s = src as usize;
        self.bytes.copy_within(s..s + len as usize, dst as usize);
    }

    /// Zeroes `len` bytes at `off`.
    #[inline]
    pub fn zero(&mut self, off: u32, len: u32) {
        self.slice_mut(off, len).fill(0);
    }
}

fn clamp_limit(limit: usize) -> u32 {
    (limit.min(MAX_ARENA as usize) as u32) & !7
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn alloc_is_aligned_and_zeroed() {
        let mut a = Arena::new(1 << 20);
        assert_eq!(a.alloc(1).unwrap(), 0);
        assert_eq!(a.alloc(9).unwrap(), 8);
        assert_eq!(a.alloc(0).unwrap(), 24);
        assert_eq!(a.alloc(8).unwrap(), 24);
        assert_eq!(a.len(), 32);
        assert!(a.bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn read_write_little_endian() {
        let mut a = Arena::new(1 << 20);
        let o = a.alloc(32).unwrap();
        a.write::<u32>(o, 0x0403_0201);
        assert_eq!(a.slice(o, 4), &[1, 2, 3, 4]);
        a.write::<u16>(o + 4, 0xBBAA);
        assert_eq!(a.slice(o + 4, 2), &[0xAA, 0xBB]);
        a.write(o + 8, Value::UNDEFINED);
        assert_eq!(a.read::<Value>(o + 8), Value::UNDEFINED);
        assert_eq!(a.read::<u64>(o + 8), Value::UNDEFINED.to_bits());
        a.write::<f64>(o + 16, -2.5);
        assert_eq!(a.read::<f64>(o + 16), -2.5);
        a.write::<i32>(o + 24, -3);
        assert_eq!(a.read::<i32>(o + 24), -3);
        a.copy_within(o, 4, o + 28);
        assert_eq!(a.read::<u32>(o + 28), 0x0403_0201);
    }

    #[test]
    fn alloc_init_writes_data_and_zeroes_the_rest() {
        let mut a = Arena::new(1 << 20);
        a.alloc(8).unwrap();
        let o = a.alloc_init(13, 3, b"abcd").unwrap();
        assert_eq!(o, 8);
        assert_eq!(a.len(), 24);
        assert_eq!(a.slice(8, 16), b"\0\0\0abcd\0\0\0\0\0\0\0\0\0");
        // Large blocks (more than one zero chunk) on both sides.
        let o = a.alloc_init(2000, 700, &[7; 600]).unwrap();
        let s = a.slice(o, 2000);
        assert!(s[..700].iter().all(|&b| b == 0));
        assert!(s[700..1300].iter().all(|&b| b == 7));
        assert!(s[1300..].iter().all(|&b| b == 0));
        let mut small = Arena::new(32);
        assert_eq!(small.alloc_init(40, 0, b"x"), Err(HeapError::OutOfMemory));
        assert_eq!(small.len(), 0);
    }

    #[test]
    #[should_panic(expected = "does not fit")]
    fn alloc_init_checks_the_fit() {
        Arena::new(1 << 20).alloc_init(8, 4, b"12345").unwrap();
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_panics() {
        let mut a = Arena::new(1 << 20);
        a.alloc(8).unwrap();
        a.read::<u32>(6);
    }

    #[test]
    fn memory_cap() {
        let mut a = Arena::new(64);
        assert!(a.alloc(48).is_ok());
        assert!(a.alloc(16).is_ok());
        assert_eq!(a.alloc(1), Err(HeapError::OutOfMemory));
        assert_eq!(a.len(), 64);
        let mut b = Arena::new(usize::MAX);
        assert_eq!(b.limit(), MAX_ARENA);
        assert_eq!(b.alloc(u32::MAX), Err(HeapError::OutOfMemory));
        assert_eq!(b.alloc(u32::MAX - 3), Err(HeapError::OutOfMemory));
        assert_eq!(b.len(), 0);
    }
}
