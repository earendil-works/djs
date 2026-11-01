//! Strings: flat one-byte ([`Str8`]), flat two-byte ([`Str16`]) and [`Rope`] cells.
//!
//! JS strings are sequences of UTF-16 code units (lone surrogates allowed). Invariants:
//!
//! - A flat string whose code units all fit in a byte is always a `Str8`. Constructors
//!   narrow, so a `Str16` always holds a unit above 0xFF. Two equal strings therefore
//!   have the same width, and equal flat strings have equal bytes.
//! - A rope has length >= [`ROPE_MIN_LEN`]; shorter concatenations are copied. Its
//!   `ROPE_TWO_BYTE` flag is the OR of its children, which is the width its flat form has.
//! - Flattening writes a new flat cell and redirects the rope to it (`LEFT` = flat,
//!   `RIGHT` = null, `ROPE_FLATTENED`). Every operation resolves that redirect first; the GC
//!   can later short-circuit it.
//!
//! Nothing here recurses: rope walks use an explicit stack, so a million-deep rope built by
//! `s += x` in a loop is fine.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::cells::{Rope, Str8, Str16, flags};
use super::layout::{FieldType, Ref};
use super::{CellKind, Heap, HeapError};
use crate::value::Str;

/// Maximum string length in code units (the same as QuickJS).
pub const MAX_LEN: u32 = (1 << 30) - 1;

/// Concatenations shorter than this are copied into a flat string instead of making a rope.
pub const ROPE_MIN_LEN: u32 = 32;

// The three string kinds share LEN and HASH offsets, so code that only needs those does not
// have to dispatch on the kind.
const _: () =
    assert!(Str8::LEN.offset == Str16::LEN.offset && Str8::LEN.offset == Rope::LEN.offset);
const _: () =
    assert!(Str8::HASH.offset == Str16::HASH.offset && Str8::HASH.offset == Rope::HASH.offset);
const _: () = assert!(Str8::DATA.offset == Str16::DATA.offset);

/// A borrowed view of a flat string's code units.
#[derive(Clone, Copy, Debug)]
pub enum StrView<'a> {
    /// One byte per code unit.
    Latin1(&'a [u8]),
    /// Two little-endian bytes per code unit.
    Utf16(&'a [u8]),
}

impl<'a> StrView<'a> {
    /// Length in code units.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            StrView::Latin1(b) => b.len(),
            StrView::Utf16(b) => b.len() / 2,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_two_byte(&self) -> bool {
        matches!(self, StrView::Utf16(_))
    }

    /// The code unit at `i`. Panics when out of range.
    #[inline]
    pub fn get(&self, i: usize) -> u16 {
        match self {
            StrView::Latin1(b) => b[i] as u16,
            StrView::Utf16(b) => u16::from_le_bytes([b[2 * i], b[2 * i + 1]]),
        }
    }

    /// The raw bytes (Latin-1 or UTF-16LE).
    #[inline]
    pub fn bytes(&self) -> &'a [u8] {
        match self {
            StrView::Latin1(b) | StrView::Utf16(b) => b,
        }
    }

    pub fn units(self) -> Units<'a> {
        Units { view: self, pos: 0 }
    }
}

/// Iterator over the code units of a [`StrView`].
#[derive(Clone)]
pub struct Units<'a> {
    view: StrView<'a>,
    pos: usize,
}

impl Iterator for Units<'_> {
    type Item = u16;

    #[inline]
    fn next(&mut self) -> Option<u16> {
        if self.pos < self.view.len() {
            let u = self.view.get(self.pos);
            self.pos += 1;
            Some(u)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.view.len() - self.pos;
        (n, Some(n))
    }
}

/// Our string hash: an FxHash-style multiply-rotate over code units, finished with the
/// murmur3 avalanche. It is defined on code units, so it does not depend on the string's
/// representation (width, rope shape). Never 0, because 0 marks "not computed".
#[derive(Clone, Copy)]
pub struct StrHasher(u32);

impl StrHasher {
    pub const fn new() -> StrHasher {
        StrHasher(0)
    }

    #[inline]
    pub fn write(&mut self, unit: u16) {
        self.0 = (self.0.rotate_left(5) ^ unit as u32).wrapping_mul(0x9E37_79B9);
    }

    pub fn finish(self) -> u32 {
        let mut h = self.0;
        h ^= h >> 16;
        h = h.wrapping_mul(0x85EB_CA6B);
        h ^= h >> 13;
        h = h.wrapping_mul(0xC2B2_AE35);
        h ^= h >> 16;
        if h == 0 { 1 } else { h }
    }
}

impl Default for StrHasher {
    fn default() -> Self {
        StrHasher::new()
    }
}

/// Hashes a sequence of code units.
pub fn hash_units(units: impl Iterator<Item = u16>) -> u32 {
    let mut h = StrHasher::new();
    for u in units {
        h.write(u);
    }
    h.finish()
}

/// Iterator over the flat pieces of a string, in order, without flattening it.
pub struct Chunks<'a> {
    heap: &'a Heap,
    stack: Vec<u32>,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = StrView<'a>;

    fn next(&mut self) -> Option<StrView<'a>> {
        while let Some(off) = self.stack.pop() {
            match self.heap.cell_kind(off) {
                CellKind::Str8 | CellKind::Str16 => {
                    let v = self.heap.flat_view(Str(off));
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
                CellKind::Rope => {
                    let a = self.heap.arena();
                    if self.heap.cell_flags(off) & flags::ROPE_FLATTENED != 0 {
                        self.stack.push(a.get(off, Rope::LEFT).0);
                    } else {
                        self.stack.push(a.get(off, Rope::RIGHT).0);
                        self.stack.push(a.get(off, Rope::LEFT).0);
                    }
                }
                k => panic!("not a string cell: {k:?} at {off}"),
            }
        }
        None
    }
}

impl Heap {
    /// Allocates a flat string cell for `len` code units, with `LEN` set and the data
    /// zeroed.
    fn alloc_flat(&mut self, len: u32, two_byte: bool) -> Result<u32, HeapError> {
        if len > MAX_LEN {
            return Err(HeapError::StringTooLong);
        }
        let (kind, size) = if two_byte {
            (CellKind::Str16, Str16::DATA.cell_size(len))
        } else {
            (CellKind::Str8, Str8::DATA.cell_size(len))
        };
        let off = self.alloc_cell(kind, size.ok_or(HeapError::StringTooLong)?)?;
        self.arena_mut().set(off, Str8::LEN, len);
        Ok(off)
    }

    /// A string from Latin-1 bytes (each byte is one code unit).
    pub fn alloc_latin1(&mut self, data: &[u8]) -> Result<Str, HeapError> {
        let len = u32::try_from(data.len()).map_err(|_| HeapError::StringTooLong)?;
        let off = self.alloc_flat(len, false)?;
        self.arena_mut()
            .slice_mut(off + Str8::DATA.offset, len)
            .copy_from_slice(data);
        Ok(Str(off))
    }

    /// A string from UTF-16 code units (lone surrogates allowed). Stored one-byte if every
    /// unit fits.
    pub fn alloc_utf16(&mut self, units: &[u16]) -> Result<Str, HeapError> {
        let len = u32::try_from(units.len()).map_err(|_| HeapError::StringTooLong)?;
        let two_byte = units.iter().any(|&u| u > 0xFF);
        let off = self.alloc_flat(len, two_byte)?;
        let data = off + Str8::DATA.offset;
        if two_byte {
            let dst = self.arena_mut().slice_mut(data, len * 2);
            for (d, &u) in dst.as_chunks_mut::<2>().0.iter_mut().zip(units) {
                d.copy_from_slice(&u.to_le_bytes());
            }
        } else {
            let dst = self.arena_mut().slice_mut(data, len);
            for (d, &u) in dst.iter_mut().zip(units) {
                *d = u as u8;
            }
        }
        Ok(Str(off))
    }

    /// A string from UTF-8, transcoded to Latin-1 or UTF-16.
    pub fn alloc_str(&mut self, s: &str) -> Result<Str, HeapError> {
        if s.is_ascii() {
            return self.alloc_latin1(s.as_bytes());
        }
        let mut len: usize = 0;
        let mut two_byte = false;
        for c in s.chars() {
            let c = c as u32;
            two_byte |= c > 0xFF;
            len += if c > 0xFFFF { 2 } else { 1 };
        }
        let len = u32::try_from(len).map_err(|_| HeapError::StringTooLong)?;
        let off = self.alloc_flat(len, two_byte)?;
        let data = off + Str8::DATA.offset;
        if two_byte {
            let dst = self.arena_mut().slice_mut(data, len * 2);
            for (d, u) in dst.as_chunks_mut::<2>().0.iter_mut().zip(s.encode_utf16()) {
                d.copy_from_slice(&u.to_le_bytes());
            }
        } else {
            let dst = self.arena_mut().slice_mut(data, len);
            for (d, c) in dst.iter_mut().zip(s.chars()) {
                *d = c as u32 as u8;
            }
        }
        Ok(Str(off))
    }

    /// Follows a flattened rope to its flat string.
    #[inline]
    pub(crate) fn resolve_str(&self, s: Str) -> Str {
        if self.cell_kind(s.0) == CellKind::Rope
            && self.cell_flags(s.0) & flags::ROPE_FLATTENED != 0
        {
            Str(self.arena().get(s.0, Rope::LEFT).0)
        } else {
            s
        }
    }

    /// Length in UTF-16 code units.
    #[inline]
    pub fn str_len(&self, s: Str) -> u32 {
        self.arena().get(s.0, Str8::LEN)
    }

    /// Whether the string (flat or rope) has a code unit above 0xFF.
    pub fn str_is_two_byte(&self, s: Str) -> bool {
        match self.cell_kind(s.0) {
            CellKind::Str8 => false,
            CellKind::Str16 => true,
            CellKind::Rope => self.cell_flags(s.0) & flags::ROPE_TWO_BYTE != 0,
            k => panic!("not a string cell: {k:?}"),
        }
    }

    /// The view of a flat string cell. `s` must be a `Str8` or `Str16`.
    fn flat_view(&self, s: Str) -> StrView<'_> {
        let len = self.str_len(s);
        let data = s.0 + Str8::DATA.offset;
        match self.cell_kind(s.0) {
            CellKind::Str8 => StrView::Latin1(self.arena().slice(data, len)),
            CellKind::Str16 => StrView::Utf16(self.arena().slice(data, len * 2)),
            k => panic!("not a flat string: {k:?}"),
        }
    }

    /// The code units of `s` if it is flat (or a flattened rope), else `None`.
    pub fn str_view(&self, s: Str) -> Option<StrView<'_>> {
        let s = self.resolve_str(s);
        match self.cell_kind(s.0) {
            CellKind::Rope => None,
            _ => Some(self.flat_view(s)),
        }
    }

    /// Flattens `s` and returns its code units.
    pub fn str_view_flat(&mut self, s: Str) -> Result<StrView<'_>, HeapError> {
        let f = self.flatten(s)?;
        Ok(self.flat_view(f))
    }

    /// Iterates over the flat pieces of `s` without flattening it.
    pub fn str_chunks(&self, s: Str) -> Chunks<'_> {
        Chunks {
            heap: self,
            stack: alloc::vec![s.0],
        }
    }

    /// Iterates over the code units of `s` without flattening it.
    pub fn str_units(&self, s: Str) -> impl Iterator<Item = u16> + '_ {
        self.str_chunks(s).flat_map(StrView::units)
    }

    /// The code unit at `i`, or `None` when out of range. Does not flatten; on an
    /// unflattened rope this walks down the tree, so callers indexing repeatedly should
    /// flatten first.
    pub fn char_code_at(&self, s: Str, i: u32) -> Option<u16> {
        let mut s = self.resolve_str(s);
        if i >= self.str_len(s) {
            return None;
        }
        let mut i = i;
        loop {
            match self.cell_kind(s.0) {
                CellKind::Str8 | CellKind::Str16 => return Some(self.flat_view(s).get(i as usize)),
                _ => {
                    let a = self.arena();
                    if self.cell_flags(s.0) & flags::ROPE_FLATTENED != 0 {
                        s = Str(a.get(s.0, Rope::LEFT).0);
                        continue;
                    }
                    let left = Str(a.get(s.0, Rope::LEFT).0);
                    let ll = self.str_len(left);
                    if i < ll {
                        s = left;
                    } else {
                        i -= ll;
                        s = Str(a.get(s.0, Rope::RIGHT).0);
                    }
                }
            }
        }
    }

    /// Appends the flat leaves of `s`, in order, to `out`.
    fn collect_leaves(&self, s: Str, out: &mut Vec<u32>) {
        let mut stack = alloc::vec![s.0];
        while let Some(off) = stack.pop() {
            match self.cell_kind(off) {
                CellKind::Rope => {
                    let a = self.arena();
                    if self.cell_flags(off) & flags::ROPE_FLATTENED != 0 {
                        stack.push(a.get(off, Rope::LEFT).0);
                    } else {
                        stack.push(a.get(off, Rope::RIGHT).0);
                        stack.push(a.get(off, Rope::LEFT).0);
                    }
                }
                _ => out.push(off),
            }
        }
    }

    /// Copies the flat cells `leaves` back to back into the data of the flat cell `dst`,
    /// which must have been allocated after all of them.
    fn write_leaves(&mut self, leaves: &[u32], dst: u32) {
        let two_byte = self.cell_kind(dst) == CellKind::Str16;
        let (src, out) = self.arena_mut().split_mut(dst);
        let mut pos = Str8::DATA.offset as usize;
        for &leaf in leaves {
            let leaf = leaf as usize;
            let len = u32::read(src, leaf + Str8::LEN.offset as usize) as usize;
            let data = leaf + Str8::DATA.offset as usize;
            let leaf_two = src[leaf] == CellKind::Str16 as u8;
            match (leaf_two, two_byte) {
                (false, false) => {
                    out[pos..pos + len].copy_from_slice(&src[data..data + len]);
                    pos += len;
                }
                (false, true) => {
                    let d = &mut out[pos..pos + 2 * len];
                    for (d, &b) in d
                        .as_chunks_mut::<2>()
                        .0
                        .iter_mut()
                        .zip(&src[data..data + len])
                    {
                        d[0] = b;
                        d[1] = 0;
                    }
                    pos += 2 * len;
                }
                (true, true) => {
                    out[pos..pos + 2 * len].copy_from_slice(&src[data..data + 2 * len]);
                    pos += 2 * len;
                }
                (true, false) => unreachable!("two-byte leaf in a one-byte string"),
            }
        }
    }

    /// Returns the flat form of `s`. A rope is copied into a new flat cell once and then
    /// redirected to it; later calls return that cell.
    pub fn flatten(&mut self, s: Str) -> Result<Str, HeapError> {
        let s = self.resolve_str(s);
        if self.cell_kind(s.0) != CellKind::Rope {
            return Ok(s);
        }
        let len = self.str_len(s);
        let two_byte = self.str_is_two_byte(s);
        let mut leaves = Vec::new();
        self.collect_leaves(s, &mut leaves);
        let dst = self.alloc_flat(len, two_byte)?;
        self.write_leaves(&leaves, dst);
        let hash = self.arena().get(s.0, Rope::HASH);
        let a = self.arena_mut();
        a.set(dst, Str8::HASH, hash);
        a.set(s.0, Rope::LEFT, Ref(dst));
        a.set(s.0, Rope::RIGHT, Ref::NULL);
        let f = self.cell_flags(s.0);
        self.set_cell_flags(s.0, f | flags::ROPE_FLATTENED);
        Ok(Str(dst))
    }

    /// Concatenates two strings. Results shorter than [`ROPE_MIN_LEN`] are flat copies;
    /// longer ones are ropes.
    pub fn concat(&mut self, a: Str, b: Str) -> Result<Str, HeapError> {
        let a = self.resolve_str(a);
        let b = self.resolve_str(b);
        let (la, lb) = (self.str_len(a), self.str_len(b));
        if la == 0 {
            return Ok(b);
        }
        if lb == 0 {
            return Ok(a);
        }
        let len = la + lb; // both <= MAX_LEN < 2^30, so no overflow
        if len > MAX_LEN {
            return Err(HeapError::StringTooLong);
        }
        let two_byte = self.str_is_two_byte(a) || self.str_is_two_byte(b);
        if len < ROPE_MIN_LEN {
            let mut leaves = Vec::new();
            self.collect_leaves(a, &mut leaves);
            self.collect_leaves(b, &mut leaves);
            let dst = self.alloc_flat(len, two_byte)?;
            self.write_leaves(&leaves, dst);
            return Ok(Str(dst));
        }
        let r = self.alloc_cell(CellKind::Rope, Rope::LAYOUT.min_size)?;
        let ar = self.arena_mut();
        ar.set(r, Rope::LEN, len);
        ar.set(r, Rope::LEFT, Ref(a.0));
        ar.set(r, Rope::RIGHT, Ref(b.0));
        if two_byte {
            self.set_cell_flags(r, flags::ROPE_TWO_BYTE);
        }
        Ok(Str(r))
    }

    /// The string hash (see [`StrHasher`]), cached in the cell.
    pub fn str_hash(&mut self, s: Str) -> u32 {
        let s = self.resolve_str(s);
        let cached = self.arena().get(s.0, Str8::HASH);
        if cached != 0 {
            return cached;
        }
        let h = self.str_hash_uncached(s);
        self.arena_mut().set(s.0, Str8::HASH, h);
        h
    }

    /// The string hash without caching it.
    pub fn str_hash_uncached(&self, s: Str) -> u32 {
        let s = self.resolve_str(s);
        let cached = self.arena().get(s.0, Str8::HASH);
        if cached != 0 {
            return cached;
        }
        match self.str_view(s) {
            Some(StrView::Latin1(b)) => {
                let mut h = StrHasher::new();
                for &c in b {
                    h.write(c as u16);
                }
                h.finish()
            }
            _ => hash_units(self.str_units(s)),
        }
    }

    /// Code unit equality.
    pub fn str_eq(&self, a: Str, b: Str) -> bool {
        if a == b {
            return true;
        }
        let a = self.resolve_str(a);
        let b = self.resolve_str(b);
        if a == b {
            return true;
        }
        if self.str_len(a) != self.str_len(b) || self.str_is_two_byte(a) != self.str_is_two_byte(b)
        {
            return false;
        }
        let (ha, hb) = (
            self.arena().get(a.0, Str8::HASH),
            self.arena().get(b.0, Str8::HASH),
        );
        if ha != 0 && hb != 0 && ha != hb {
            return false;
        }
        match (self.str_view(a), self.str_view(b)) {
            // Same width (checked above), so equal content means equal bytes.
            (Some(x), Some(y)) => x.bytes() == y.bytes(),
            _ => self.str_units(a).eq(self.str_units(b)),
        }
    }

    /// Code unit order (what `<` on strings means in JS).
    pub fn str_cmp(&self, a: Str, b: Str) -> Ordering {
        if a == b {
            return Ordering::Equal;
        }
        match (self.str_view(a), self.str_view(b)) {
            (Some(StrView::Latin1(x)), Some(StrView::Latin1(y))) => x.cmp(y),
            _ => self.str_units(a).cmp(self.str_units(b)),
        }
    }

    /// Whether `s` has the same code units as the Rust string `t`.
    pub fn str_eq_str(&self, s: Str, t: &str) -> bool {
        if let Some(StrView::Latin1(b)) = self.str_view(s)
            && t.is_ascii()
        {
            return b == t.as_bytes();
        }
        self.str_units(s).eq(t.encode_utf16())
    }

    /// Converts to a Rust string, replacing lone surrogates with U+FFFD. For tests,
    /// debugging and host output.
    pub fn str_to_string(&self, s: Str) -> String {
        char::decode_utf16(self.str_units(s))
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    /// The code units as a vector (tests and debugging).
    pub fn str_to_utf16(&self, s: Str) -> Vec<u16> {
        self.str_units(s).collect()
    }
}
