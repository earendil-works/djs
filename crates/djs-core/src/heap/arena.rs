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
    #[inline]
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
        self.bytes.resize(end_us, 0);
        Ok(start)
    }

    #[inline]
    pub fn read<T: FieldType>(&self, off: u32) -> T {
        T::read(&self.bytes, off as usize)
    }

    #[inline]
    pub fn write<T: FieldType>(&mut self, off: u32, v: T) {
        T::write(&mut self.bytes, off as usize, v)
    }

    /// Reads field `f` of the cell (or record) at `base`.
    #[inline]
    pub fn get<T: FieldType>(&self, base: u32, f: Field<T>) -> T {
        self.read(base + f.offset)
    }

    /// Writes field `f` of the cell (or record) at `base`.
    #[inline]
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
