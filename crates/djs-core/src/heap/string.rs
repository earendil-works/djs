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
//! - An append rope (`ROPE_APPEND`, D1610) is the first `LEN` code units of the flat cell
//!   in `LEFT`, an append buffer (`STR_APPEND_BUF`: spare capacity past its `LEN`, the
//!   used length) or a frozen former one. `concat` writes into a buffer in place only when
//!   the rope ends where the buffer's used part ends, so every older, shorter rope over the
//!   same buffer keeps its value. Buffers are only ever referenced from append ropes'
//!   `LEFT`: [`Heap::flatten`] hands out a buffer only after freezing it (clearing the
//!   flag), and a frozen buffer is never written again.
//!
//! Nothing here recurses: rope walks use an explicit stack, so a million-deep rope built by
//! `s += x` in a loop is fine.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::cells::{Rope, Str8, Str16, flags};
use super::layout::Ref;
use super::{CellKind, Heap, HeapError};
use crate::value::Str;

/// Maximum string length in code units (the same as QuickJS).
pub const MAX_LEN: u32 = (1 << 30) - 1;

/// Concatenations shorter than this are copied into a flat string instead of making a rope.
pub const ROPE_MIN_LEN: u32 = 32;

/// Smallest append buffer, in code units.
const APPEND_MIN_CAP: u32 = 64;

/// A concatenation chain gets an append buffer once its left side is this long: shorter
/// strings (the pieces of an expression like `"id " + i + "\n"`) stay ropes or copies.
const APPEND_START_LEN: u32 = 256;

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

/// Whether every byte is ASCII. In wasm32, `<[u8]>::is_ascii` runs at 8–9 GB/s in node,
/// [`is_ascii_blocks`] at ~40 (0077, D1780); natively the standard one is faster (SIMD).
#[inline]
pub fn is_ascii(bytes: &[u8]) -> bool {
    #[cfg(target_arch = "wasm32")]
    return is_ascii_blocks(bytes);
    #[cfg(not(target_arch = "wasm32"))]
    return bytes.is_ascii();
}

/// The ASCII check for wasm32: 32-byte blocks OR-ed as u64 words and tested once per
/// block, so the loop has one branch per 32 bytes.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn is_ascii_blocks(bytes: &[u8]) -> bool {
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let (blocks, rest) = bytes.as_chunks::<32>();
    for b in blocks {
        let w = b.as_chunks::<8>().0;
        let acc = u64::from_ne_bytes(w[0])
            | u64::from_ne_bytes(w[1])
            | u64::from_ne_bytes(w[2])
            | u64::from_ne_bytes(w[3]);
        if acc & HIGH != 0 {
            return false;
        }
    }
    let (words, tail) = rest.as_chunks::<8>();
    let mut acc = 0;
    for w in words {
        acc |= u64::from_ne_bytes(*w);
    }
    for &b in tail {
        acc |= b as u64;
    }
    acc & HIGH == 0
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
    /// `(cell, length)`: the length clips an append rope's buffer.
    stack: Vec<(u32, u32)>,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = StrView<'a>;

    fn next(&mut self) -> Option<StrView<'a>> {
        while let Some((off, len)) = self.stack.pop() {
            match self.heap.cell_kind(off) {
                CellKind::Str8 | CellKind::Str16 => {
                    let v = self.heap.prefix_view(off, len);
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
                CellKind::Rope => self.heap.push_children(off, &mut self.stack),
                k => super::fail::not_a_string(k, off),
            }
        }
        None
    }
}

impl Heap {
    /// The parts of the rope `off` as `(cell, length)`, in order (one or two).
    fn children(&self, off: u32) -> ([(u32, u32); 2], usize) {
        let a = self.arena();
        let fl = self.cell_flags(off);
        let left = a.get(off, Rope::LEFT).0;
        if fl & flags::ROPE_APPEND != 0 {
            ([(left, a.get(off, Rope::LEN)), (0, 0)], 1)
        } else if fl & flags::ROPE_FLATTENED != 0 {
            ([(left, self.str_len(Str(left))), (0, 0)], 1)
        } else {
            let right = a.get(off, Rope::RIGHT).0;
            (
                [
                    (left, self.str_len(Str(left))),
                    (right, self.str_len(Str(right))),
                ],
                2,
            )
        }
    }

    /// Pushes the parts of the rope `off` as `(cell, length)` for a left-to-right walk
    /// with a stack (right first).
    fn push_children(&self, off: u32, stack: &mut Vec<(u32, u32)>) {
        let (kids, n) = self.children(off);
        stack.extend(kids[..n].iter().rev());
    }

    /// The first `len` code units of the flat cell `off`.
    fn prefix_view(&self, off: u32, len: u32) -> StrView<'_> {
        let data = off + Str8::DATA.offset;
        match self.cell_kind(off) {
            CellKind::Str8 => StrView::Latin1(self.arena().slice(data, len)),
            CellKind::Str16 => StrView::Utf16(self.arena().slice(data, len * 2)),
            k => super::fail::not_a_string(k, off),
        }
    }

    /// The buffer cell of an append rope, if `s` is one.
    fn append_rope_buffer(&self, s: Str) -> Option<u32> {
        (self.cell_kind(s.0) == CellKind::Rope && self.cell_flags(s.0) & flags::ROPE_APPEND != 0)
            .then(|| self.arena().get(s.0, Rope::LEFT).0)
    }

    /// The capacity of a flat string cell in code units.
    fn buffer_capacity(&self, buf: u32) -> u32 {
        let size = self.cell_size(buf);
        if self.cell_kind(buf) == CellKind::Str16 {
            Str16::LAYOUT.tail_capacity(size)
        } else {
            Str8::LAYOUT.tail_capacity(size)
        }
    }

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

    /// Allocates a flat string of `len` code units with zeroed data, for callers that write
    /// the data themselves (at [`Heap::str_data_offset`], one byte per unit for one-byte
    /// strings, little-endian pairs otherwise). A two-byte string must end up containing a
    /// unit above 0xFF (the width invariant), so only ask for one when that is known.
    pub fn alloc_str_uninit(&mut self, len: u32, two_byte: bool) -> Result<Str, HeapError> {
        self.alloc_flat(len, two_byte).map(Str)
    }

    /// The arena offset of the code units of a flat string cell.
    #[inline]
    pub fn str_data_offset(s: Str) -> u32 {
        s.0 + Str8::DATA.offset
    }

    /// A string from Latin-1 bytes (each byte is one code unit).
    pub fn alloc_latin1(&mut self, data: &[u8]) -> Result<Str, HeapError> {
        let len = u32::try_from(data.len()).map_err(|_| HeapError::StringTooLong)?;
        if len > MAX_LEN {
            return Err(HeapError::StringTooLong);
        }
        let size = Str8::DATA.cell_size(len).ok_or(HeapError::StringTooLong)?;
        let off = self.alloc_cell_init(CellKind::Str8, size, Str8::DATA.offset, data)?;
        self.arena_mut().set(off, Str8::LEN, len);
        Ok(Str(off))
    }

    /// A string from UTF-16 code units (lone surrogates allowed). Stored one-byte if every
    /// unit fits.
    pub fn alloc_utf16(&mut self, units: &[u16]) -> Result<Str, HeapError> {
        let len = u32::try_from(units.len()).map_err(|_| HeapError::StringTooLong)?;
        if len > MAX_LEN {
            return Err(HeapError::StringTooLong);
        }
        let two_byte = units.iter().any(|&u| u > 0xFF);
        // The data is converted straight into the new cell (written once, not zeroed
        // first); `flat_map` over arrays is TrustedLen, so the append is one tight loop.
        let off = if two_byte {
            let size = Str16::DATA.cell_size(len).ok_or(HeapError::StringTooLong)?;
            let data = units.iter().flat_map(|u| u.to_le_bytes());
            self.alloc_cell_from_iter(CellKind::Str16, size, Str16::DATA.offset, len * 2, data)?
        } else {
            let size = Str8::DATA.cell_size(len).ok_or(HeapError::StringTooLong)?;
            let data = units.iter().map(|&u| u as u8);
            self.alloc_cell_from_iter(CellKind::Str8, size, Str8::DATA.offset, len, data)?
        };
        self.arena_mut().set(off, Str8::LEN, len);
        Ok(Str(off))
    }

    /// A string from UTF-8, transcoded to Latin-1 or UTF-16.
    pub fn alloc_str(&mut self, s: &str) -> Result<Str, HeapError> {
        if is_ascii(s.as_bytes()) {
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
            k => super::fail::not_a_string(k, s.0),
        }
    }

    /// The view of a flat string cell. `s` must be a `Str8` or `Str16`.
    fn flat_view(&self, s: Str) -> StrView<'_> {
        self.prefix_view(s.0, self.str_len(s))
    }

    /// The code units of `s` if it is flat (a flattened rope or an append rope too), else
    /// `None`.
    pub fn str_view(&self, s: Str) -> Option<StrView<'_>> {
        let s = self.resolve_str(s);
        if let Some(buf) = self.append_rope_buffer(s) {
            return Some(self.prefix_view(buf, self.str_len(s)));
        }
        match self.cell_kind(s.0) {
            CellKind::Rope => None,
            _ => Some(self.flat_view(s)),
        }
    }

    /// The code units of `s`, flattening it first if it is a concatenation rope. An append
    /// rope is read in place (no copy, and its buffer stays appendable).
    pub fn str_view_flat(&mut self, s: Str) -> Result<StrView<'_>, HeapError> {
        let s = self.resolve_str(s);
        if let Some(buf) = self.append_rope_buffer(s) {
            return Ok(self.prefix_view(buf, self.str_len(s)));
        }
        let f = self.flatten(s)?;
        Ok(self.flat_view(f))
    }

    /// Iterates over the flat pieces of `s` without flattening it.
    pub fn str_chunks(&self, s: Str) -> Chunks<'_> {
        Chunks {
            heap: self,
            stack: alloc::vec![(s.0, self.str_len(s))],
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
                    if self.cell_flags(s.0) & flags::ROPE_APPEND != 0 {
                        // i < LEN <= the buffer's used length.
                        let buf = a.get(s.0, Rope::LEFT).0;
                        return Some(self.prefix_view(buf, i + 1).get(i as usize));
                    }
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

    /// Appends the flat leaves of `s` as `(cell, length)`, in order, to `out`.
    /// The leaves of `parts` (strings with fewer than [`ROPE_MIN_LEN`] units in total), in
    /// order, into `out`; returns how many. No Rust allocation (D1871): `'a' + i` used to
    /// spend more in malloc on three Vecs than on the concatenation. Every pending subtree
    /// and every leaf holds at least one unit, so neither array can overflow.
    fn short_leaves(
        &self,
        parts: [(u32, u32); 2],
        out: &mut [(u32, u32); ROPE_MIN_LEN as usize],
    ) -> usize {
        let mut stack = [(0, 0); ROPE_MIN_LEN as usize];
        stack[0] = parts[1];
        stack[1] = parts[0];
        let (mut sp, mut n) = (2, 0);
        while sp > 0 {
            sp -= 1;
            let (off, len) = stack[sp];
            if self.cell_kind(off) == CellKind::Rope {
                let (kids, k) = self.children(off);
                for &kid in kids[..k].iter().rev() {
                    stack[sp] = kid;
                    sp += 1;
                }
            } else {
                out[n] = (off, len);
                n += 1;
            }
        }
        n
    }

    fn collect_leaves(&self, s: Str, out: &mut Vec<(u32, u32)>) {
        let mut stack = alloc::vec![(s.0, self.str_len(s))];
        while let Some((off, len)) = stack.pop() {
            match self.cell_kind(off) {
                CellKind::Rope => self.push_children(off, &mut stack),
                _ => out.push((off, len)),
            }
        }
    }

    /// Copies the first `len` units of each flat cell in `leaves`, back to back, into the
    /// data of the flat cell `dst` from code unit `at` on. The leaves must lie below `dst`
    /// in the arena.
    fn write_leaves(&mut self, leaves: &[(u32, u32)], dst: u32, at: u32) {
        let two_byte = self.cell_kind(dst) == CellKind::Str16;
        let (src, out) = self.arena_mut().split_mut(dst);
        let unit = if two_byte { 2 } else { 1 };
        let mut pos = Str8::DATA.offset as usize + unit * at as usize;
        for &(leaf, len) in leaves {
            let leaf = leaf as usize;
            let len = len as usize;
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

    /// Appends `b` in place to the append buffer `buf`, whose used part ends at `at` (the
    /// caller checked the room, the width and that `at` is the end). The pieces may lie
    /// anywhere in the arena.
    fn append_into(&mut self, buf: u32, at: u32, b: Str) {
        if matches!(self.cell_kind(b.0), CellKind::Str8 | CellKind::Str16) {
            let lb = self.str_len(b);
            self.copy_piece(b.0, lb, buf, at);
            return;
        }
        // A small rope (a template literal's result): walk it with a stack on the Rust
        // stack, so the hot path does not allocate.
        let mut stack = [(0u32, 0u32); 16];
        let mut n = 0;
        stack[0] = (b.0, self.str_len(b));
        n += 1;
        let mut at = at;
        while n > 0 {
            n -= 1;
            let (off, len) = stack[n];
            if self.cell_kind(off) != CellKind::Rope {
                self.copy_piece(off, len, buf, at);
                at += len;
                continue;
            }
            let (kids, nk) = self.children(off);
            if n + nk > stack.len() {
                // Deep: the general walk.
                let mut leaves = Vec::new();
                for &(k, kl) in &kids[..nk] {
                    self.collect_leaves_of(k, kl, &mut leaves);
                }
                for (k, kl) in stack[..n].iter().rev().copied() {
                    self.collect_leaves_of(k, kl, &mut leaves);
                }
                for (leaf, len) in leaves {
                    self.copy_piece(leaf, len, buf, at);
                    at += len;
                }
                return;
            }
            for &k in kids[..nk].iter().rev() {
                stack[n] = k;
                n += 1;
            }
        }
    }

    /// The leaves of `(off, len)` (a string cell clipped to `len`), appended to `out`.
    fn collect_leaves_of(&self, off: u32, len: u32, out: &mut Vec<(u32, u32)>) {
        let mut stack = alloc::vec![(off, len)];
        while let Some((off, len)) = stack.pop() {
            match self.cell_kind(off) {
                CellKind::Rope => self.push_children(off, &mut stack),
                _ => out.push((off, len)),
            }
        }
    }

    /// Copies the first `len` units of the flat cell `src` into the buffer `buf` at unit
    /// `at` (widening when needed; anywhere in the arena).
    fn copy_piece(&mut self, src: u32, len: u32, buf: u32, at: u32) {
        let src_two = self.cell_kind(src) == CellKind::Str16;
        let wide = self.cell_kind(buf) == CellKind::Str16;
        let from = src + Str8::DATA.offset;
        let a = self.arena_mut();
        match (src_two, wide) {
            (false, false) => a.copy_within(from, len, buf + Str8::DATA.offset + at),
            (true, true) => a.copy_within(from, 2 * len, buf + Str16::DATA.offset + 2 * at),
            (false, true) => {
                for i in 0..len {
                    let c = a.read::<u8>(from + i);
                    a.write::<u16>(buf + Str16::DATA.offset + 2 * (at + i), c as u16);
                }
            }
            (true, false) => unreachable!("two-byte piece in a one-byte buffer"),
        }
    }

    /// A new append rope over `buf` with length `len`.
    fn new_append_rope(&mut self, buf: u32, len: u32) -> Result<Str, HeapError> {
        let two = self.cell_kind(buf) == CellKind::Str16;
        let r = self.alloc_cell(CellKind::Rope, Rope::LAYOUT.min_size)?;
        let ar = self.arena_mut();
        ar.set(r, Rope::LEN, len);
        ar.set(r, Rope::LEFT, Ref(buf));
        ar.set(r, Rope::RIGHT, Ref::NULL);
        let fl = flags::ROPE_APPEND | if two { flags::ROPE_TWO_BYTE } else { 0 };
        self.set_cell_flags(r, fl);
        Ok(Str(r))
    }

    /// A new append buffer holding `a` then `b`, with about twice their length as
    /// capacity, and an append rope over it.
    fn start_buffer(&mut self, a: Str, b: Str, len: u32, two_byte: bool) -> Result<Str, HeapError> {
        let cap = len.saturating_mul(2).clamp(APPEND_MIN_CAP, MAX_LEN);
        let buf = match self.alloc_flat(cap, two_byte) {
            Ok(buf) => buf,
            // No room for the slack: an exact buffer still works (it regrows later).
            Err(HeapError::OutOfMemory) => self.alloc_flat(len, two_byte)?,
            Err(e) => return Err(e),
        };
        let mut leaves = Vec::new();
        self.collect_leaves(a, &mut leaves);
        self.collect_leaves(b, &mut leaves);
        self.write_leaves(&leaves, buf, 0);
        self.arena_mut().set(buf, Str8::LEN, len);
        self.set_cell_flags(buf, flags::STR_APPEND_BUF);
        self.new_append_rope(buf, len)
    }

    /// Whether `a + b` looks like a step of `s += piece` in a loop: `a` is already an
    /// append rope, or a concatenation rope of at least `APPEND_START_LEN` units whose
    /// right part is short, and `b` is not longer than `a`. Such chains get a buffer (D1610); other concatenations stay O(1)
    /// ropes.
    fn appending(&self, a: Str, la: u32, lb: u32) -> bool {
        if lb > la || la < APPEND_START_LEN || self.cell_kind(a.0) != CellKind::Rope {
            return false;
        }
        let fl = self.cell_flags(a.0);
        if fl & flags::ROPE_APPEND != 0 {
            return true;
        }
        fl & flags::ROPE_FLATTENED == 0
            && self.str_len(Str(self.arena().get(a.0, Rope::RIGHT).0)) <= 2 * ROPE_MIN_LEN
    }

    /// Returns the flat form of `s`. A rope is copied into a new flat cell once and then
    /// redirected to it; later calls return that cell. An append rope that covers its
    /// whole buffer hands out the buffer itself, frozen first (from then on it may be
    /// referenced directly, so it is never written again); a shorter one is copied.
    pub fn flatten(&mut self, s: Str) -> Result<Str, HeapError> {
        let s = self.resolve_str(s);
        if self.cell_kind(s.0) != CellKind::Rope {
            return Ok(s);
        }
        let len = self.str_len(s);
        let two_byte = self.str_is_two_byte(s);
        let dst = match self.append_rope_buffer(s) {
            Some(buf) if self.str_len(Str(buf)) == len => {
                let f = self.cell_flags(buf);
                self.set_cell_flags(buf, f & !flags::STR_APPEND_BUF);
                buf
            }
            _ => {
                let mut leaves = Vec::new();
                self.collect_leaves(s, &mut leaves);
                let dst = self.alloc_flat(len, two_byte)?;
                self.write_leaves(&leaves, dst, 0);
                dst
            }
        };
        let hash = self.arena().get(s.0, Rope::HASH);
        let a = self.arena_mut();
        if a.get(dst, Str8::HASH) == 0 {
            a.set(dst, Str8::HASH, hash);
        }
        a.set(s.0, Rope::LEFT, Ref(dst));
        a.set(s.0, Rope::RIGHT, Ref::NULL);
        let f = self.cell_flags(s.0);
        self.set_cell_flags(s.0, (f & !flags::ROPE_APPEND) | flags::ROPE_FLATTENED);
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
            let mut leaves = [(0, 0); ROPE_MIN_LEN as usize];
            let n = self.short_leaves([(a.0, la), (b.0, lb)], &mut leaves);
            let dst = self.alloc_flat(len, two_byte)?;
            self.write_leaves(&leaves[..n], dst, 0);
            return Ok(Str(dst));
        }
        // In place (D1610): `a` ends exactly where its buffer's used part ends (no other
        // string sees the units past it), the buffer is not frozen, and `b` fits in room
        // and width.
        if let Some(buf) = self.append_rope_buffer(a)
            && self.cell_flags(buf) & flags::STR_APPEND_BUF != 0
            && self.str_len(Str(buf)) == la
            && len <= self.buffer_capacity(buf)
            && (self.cell_kind(buf) == CellKind::Str16 || !two_byte)
        {
            self.append_into(buf, la, b);
            self.arena_mut().set(buf, Str8::LEN, len);
            return self.new_append_rope(buf, len);
        }
        if self.appending(a, la, lb) {
            return self.start_buffer(a, b, len, two_byte);
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
            Some(v) => hash_units(v.units()),
            None => hash_units(self.str_units(s)),
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
            && is_ascii(t.as_bytes())
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
