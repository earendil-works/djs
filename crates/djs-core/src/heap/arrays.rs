//! Helpers for the generic array cells, [`ValueArray`] and [`U32Array`].

use super::cells::{U32Array, ValueArray};
use super::{CellKind, Heap, HeapError};
use crate::value::Value;

impl Heap {
    /// Allocates a [`ValueArray`] with room for `cap` values and `LEN` 0.
    pub(crate) fn alloc_value_array(&mut self, cap: u32) -> Result<u32, HeapError> {
        let size = ValueArray::VALUES
            .cell_size(cap)
            .ok_or(HeapError::OutOfMemory)?;
        self.alloc_cell(CellKind::ValueArray, size)
    }

    /// Slots a [`ValueArray`] can hold.
    #[inline]
    pub(crate) fn value_array_capacity(&self, arr: u32) -> u32 {
        ValueArray::LAYOUT.tail_capacity(self.cell_size(arr))
    }

    #[inline]
    pub(crate) fn value_array_len(&self, arr: u32) -> u32 {
        self.arena().get(arr, ValueArray::LEN)
    }

    #[inline]
    pub(crate) fn value_array_get(&self, arr: u32, i: u32) -> Value {
        debug_assert!(i < self.value_array_capacity(arr));
        self.arena().read(ValueArray::VALUES.at(arr, i))
    }

    #[inline]
    pub(crate) fn value_array_set(&mut self, arr: u32, i: u32, v: Value) {
        debug_assert!(i < self.value_array_capacity(arr));
        self.arena_mut().write(ValueArray::VALUES.at(arr, i), v)
    }

    /// Returns an array with the contents of `arr` (0 for none) and room for at least
    /// `min_cap` values: `arr` itself if it is big enough, else a new copy with doubled
    /// capacity. The old cell becomes garbage.
    pub(crate) fn value_array_reserve(&mut self, arr: u32, min_cap: u32) -> Result<u32, HeapError> {
        let (cap, len) = if arr == 0 {
            (0, 0)
        } else {
            (self.value_array_capacity(arr), self.value_array_len(arr))
        };
        if min_cap <= cap {
            return Ok(arr);
        }
        let new_cap = min_cap.max(cap.saturating_mul(2)).max(8);
        let new = self.alloc_value_array(new_cap)?;
        if len > 0 {
            let src = ValueArray::VALUES.at(arr, 0);
            let dst = ValueArray::VALUES.at(new, 0);
            self.arena_mut().copy_within(src, len * 8, dst);
        }
        self.arena_mut().set(new, ValueArray::LEN, len);
        Ok(new)
    }

    /// Allocates a zeroed [`U32Array`] of `len` entries.
    pub(crate) fn alloc_u32_array(&mut self, len: u32) -> Result<u32, HeapError> {
        let size = U32Array::DATA
            .cell_size(len)
            .ok_or(HeapError::OutOfMemory)?;
        let a = self.alloc_cell(CellKind::U32Array, size)?;
        self.arena_mut().set(a, U32Array::LEN, len);
        Ok(a)
    }

    #[inline]
    pub(crate) fn u32_array_len(&self, arr: u32) -> u32 {
        self.arena().get(arr, U32Array::LEN)
    }

    #[inline]
    pub(crate) fn u32_array_get(&self, arr: u32, i: u32) -> u32 {
        debug_assert!(i < self.u32_array_len(arr));
        self.arena().read(U32Array::DATA.at(arr, i))
    }

    #[inline]
    pub(crate) fn u32_array_set(&mut self, arr: u32, i: u32, v: u32) {
        debug_assert!(i < self.u32_array_len(arr));
        self.arena_mut().write(U32Array::DATA.at(arr, i), v)
    }
}
