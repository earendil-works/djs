//! The inbound wire format (host → VM): decodes a buffer written by the host's `encode`
//! (js/src/wire.ts) into heap values. The format is specified in docs/wire.md.
//!
//! One pass over the input with an explicit stack (no recursion): the pending keys and values
//! of all open containers sit in Rust vectors, and each object or array is created in one
//! allocation when its last element has been read (0012's bulk constructors). Keys are
//! interned once each through the buffer's key table. Strings are allocated at their final
//! size and width and filled straight from the input.
//!
//! GC contract: decoding allocates but never collects. Values held in the Rust vectors are
//! not visible to the collector, which is fine because nothing can collect before `decode`
//! returns; the caller checks `Heap::gc_pending` afterwards.

#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use core::fmt;

use crate::heap::{Atom, Heap, HeapError, atoms, string::MAX_LEN};
use crate::json::DEFAULT_MAX_DEPTH;
use crate::value::{Obj, Value};

pub const VERSION: u8 = 1;
pub const MAGIC: [u8; 3] = *b"DJW";

pub mod tag {
    pub const UNDEFINED: u8 = 0x00;
    pub const NULL: u8 = 0x01;
    pub const FALSE: u8 = 0x02;
    pub const TRUE: u8 = 0x03;
    pub const INT32: u8 = 0x04;
    pub const FLOAT64: u8 = 0x05;
    pub const HOLE: u8 = 0x06;
    pub const INT8: u8 = 0x07;
    pub const STR_UTF8: u8 = 0x10;
    pub const STR_UTF16: u8 = 0x11;
    pub const ARRAY: u8 = 0x20;
    pub const OBJECT: u8 = 0x21;
    pub const BYTES: u8 = 0x30;
    pub const HOST_REF: u8 = 0x3F;
}

/// Prototypes and limits for [`decode`].
#[derive(Clone, Copy, Debug)]
pub struct DecodeOptions {
    /// Prototype of decoded objects (the realm's `%Object.prototype%`).
    pub object_proto: Option<Obj>,
    /// Prototype of decoded arrays (and of the arrays that stand in for bytes).
    pub array_proto: Option<Obj>,
    /// Maximum nesting of arrays and objects.
    pub max_depth: u32,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        DecodeOptions {
            object_proto: None,
            array_proto: None,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WireError {
    /// The input is not a valid buffer; `offset` is where decoding stopped.
    Invalid { offset: usize, reason: &'static str },
    /// The heap ran out of memory or a string is too long.
    Heap(HeapError),
}

impl From<HeapError> for WireError {
    fn from(e: HeapError) -> WireError {
        WireError::Heap(e)
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::Invalid { offset, reason } => {
                write!(f, "invalid wire data at offset {offset}: {reason}")
            }
            WireError::Heap(e) => write!(f, "{e:?}"),
        }
    }
}

/// An open array or object.
struct Frame {
    is_object: bool,
    /// Elements or properties still to read.
    remaining: u32,
    /// Where this container's values start in `values` (and keys in `keys`).
    values_start: usize,
    keys_start: usize,
}

struct Decoder<'a, 'h> {
    heap: &'h mut Heap,
    input: &'a [u8],
    pos: usize,
    opts: DecodeOptions,
    /// The buffer's key table.
    key_table: Vec<Atom>,
    keys: Vec<Atom>,
    values: Vec<Value>,
    stack: Vec<Frame>,
}

/// Decodes a wire buffer into a heap value.
pub fn decode(heap: &mut Heap, input: &[u8], opts: &DecodeOptions) -> Result<Value, WireError> {
    let mut d = Decoder {
        heap,
        input,
        pos: 0,
        opts: *opts,
        key_table: Vec::new(),
        keys: Vec::new(),
        values: Vec::new(),
        stack: Vec::new(),
    };
    d.run()
}

impl<'a> Decoder<'a, '_> {
    fn invalid<T>(&self, reason: &'static str) -> Result<T, WireError> {
        Err(WireError::Invalid {
            offset: self.pos,
            reason,
        })
    }

    fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    fn byte(&mut self) -> Result<u8, WireError> {
        match self.input.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                Ok(b)
            }
            None => self.invalid("unexpected end"),
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if n > self.remaining() {
            return self.invalid("unexpected end");
        }
        let input: &'a [u8] = self.input;
        let s = &input[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn varint(&mut self) -> Result<u32, WireError> {
        let mut v: u64 = 0;
        for i in 0..5 {
            let b = self.byte()?;
            v |= ((b & 0x7F) as u64) << (7 * i);
            if b & 0x80 == 0 {
                return match u32::try_from(v) {
                    Ok(v) => Ok(v),
                    Err(_) => self.invalid("varint overflow"),
                };
            }
        }
        self.invalid("varint too long")
    }

    fn run(&mut self) -> Result<Value, WireError> {
        let header = self.take(4)?;
        if header[..3] != MAGIC {
            self.pos = 0;
            return self.invalid("bad magic");
        }
        if header[3] != VERSION {
            self.pos = 3;
            return self.invalid("unsupported version");
        }
        loop {
            // Inside an object, a key comes before each value.
            if let Some(f) = self.stack.last()
                && f.is_object
            {
                let key = self.key()?;
                self.keys.push(key);
            }
            let v = match self.value()? {
                Some(v) => v,
                None => continue, // a container was opened
            };
            let mut v = v;
            // Close every container that is now complete.
            loop {
                let Some(f) = self.stack.last_mut() else {
                    if self.pos != self.input.len() {
                        return self.invalid("trailing bytes");
                    }
                    return Ok(v);
                };
                self.values.push(v);
                f.remaining -= 1;
                if f.remaining > 0 {
                    break;
                }
                let f = self.stack.pop().unwrap();
                v = self.build(&f)?;
            }
        }
    }

    /// Creates the container of a completed frame.
    fn build(&mut self, f: &Frame) -> Result<Value, WireError> {
        let o = if f.is_object {
            self.heap.new_object_from_entries(
                self.opts.object_proto,
                &self.keys[f.keys_start..],
                &self.values[f.values_start..],
            )?
        } else {
            self.heap
                .new_array_from_values(self.opts.array_proto, &self.values[f.values_start..])?
        };
        self.keys.truncate(f.keys_start);
        self.values.truncate(f.values_start);
        Ok(Value::object(o))
    }

    /// Reads a value. A non-empty container is opened instead (`None`); its elements
    /// follow.
    fn value(&mut self) -> Result<Option<Value>, WireError> {
        let at = self.pos;
        let t = self.byte()?;
        let v = match t {
            tag::UNDEFINED => Value::UNDEFINED,
            tag::NULL => Value::NULL,
            tag::FALSE => Value::bool(false),
            tag::TRUE => Value::bool(true),
            tag::INT8 => Value::int(self.byte()? as i8 as i32),
            tag::INT32 => {
                let b = self.take(4)?;
                Value::int(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            }
            tag::FLOAT64 => {
                let b = self.take(8)?;
                number_value(f64::from_le_bytes(b.try_into().unwrap()))
            }
            tag::HOLE => {
                if !matches!(self.stack.last(), Some(f) if !f.is_object) {
                    self.pos = at;
                    return self.invalid("hole outside an array");
                }
                Value::HOLE
            }
            tag::STR_UTF8 | tag::STR_UTF16 => self.string(t, false)?.0,
            tag::ARRAY | tag::OBJECT => {
                let is_object = t == tag::OBJECT;
                let count = self.varint()?;
                if count == 0 {
                    let o = if is_object {
                        self.heap
                            .new_object_from_entries(self.opts.object_proto, &[], &[])?
                    } else {
                        self.heap
                            .new_array_from_values(self.opts.array_proto, &[])?
                    };
                    Value::object(o)
                } else {
                    // Every element takes at least one byte, every property two.
                    let min = count as usize * if is_object { 2 } else { 1 };
                    if min > self.remaining() {
                        return self.invalid("count exceeds the input");
                    }
                    if self.stack.len() as u32 >= self.opts.max_depth {
                        self.pos = at;
                        return self.invalid("nesting too deep");
                    }
                    self.stack.push(Frame {
                        is_object,
                        remaining: count,
                        values_start: self.values.len(),
                        keys_start: self.keys.len(),
                    });
                    return Ok(None);
                }
            }
            tag::BYTES => {
                let n = self.varint()? as usize;
                let bytes = self.take(n)?;
                // Until typed arrays exist: an array of numbers (docs/wire.md).
                let vals: Vec<Value> = bytes.iter().map(|&b| Value::int(b as i32)).collect();
                Value::object(
                    self.heap
                        .new_array_from_values(self.opts.array_proto, &vals)?,
                )
            }
            tag::HOST_REF => {
                self.pos = at;
                return self.invalid("host refs are not supported");
            }
            _ => {
                self.pos = at;
                return self.invalid("unknown tag");
            }
        };
        Ok(Some(v))
    }

    /// A key: an index into the key table, or 0 and a new key string.
    fn key(&mut self) -> Result<Atom, WireError> {
        let k = self.varint()?;
        if k == 0 {
            let t = self.byte()?;
            if t != tag::STR_UTF8 && t != tag::STR_UTF16 {
                self.pos -= 1;
                return self.invalid("expected a key string");
            }
            let atom = self.string(t, true)?.1;
            self.key_table.push(atom);
            return Ok(atom);
        }
        match self.key_table.get(k as usize - 1) {
            Some(&a) => Ok(a),
            None => self.invalid("key index out of range"),
        }
    }

    /// Reads a string payload (after its tag). As a key it is interned (the atom is
    /// returned, the value is undefined); otherwise it becomes a heap string.
    fn string(&mut self, t: u8, as_key: bool) -> Result<(Value, Atom), WireError> {
        let units = self.varint()?;
        if units > MAX_LEN {
            return self.invalid("string too long");
        }
        if t == tag::STR_UTF16 {
            let data = self.take(units as usize * 2)?;
            let mut buf: Vec<u16> = Vec::with_capacity(units as usize);
            buf.extend(
                data.as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| u16::from_le_bytes(*p)),
            );
            if as_key {
                return Ok((Value::UNDEFINED, self.heap.atom_utf16(&buf)?));
            }
            if buf.is_empty() {
                return Ok((self.empty_string()?, Atom::NULL));
            }
            let s = self.heap.alloc_utf16(&buf)?;
            return Ok((Value::string(s), Atom::NULL));
        }
        let nbytes = self.varint()?;
        if units > nbytes || (nbytes as u64) > 3 * units as u64 {
            return self.invalid("string lengths do not match");
        }
        let start = self.pos;
        let bytes = self.take(nbytes as usize)?;
        // Valid UTF-8 has as many bytes as UTF-16 units only if it is ASCII, which is much
        // cheaper to check than full UTF-8 validation (most strings are short).
        let ascii = units == nbytes;
        let valid = if ascii {
            bytes.is_ascii()
        } else {
            core::str::from_utf8(bytes).is_ok()
        };
        if !valid {
            self.pos = start;
            return self.invalid("invalid UTF-8");
        }
        if as_key {
            let text = core::str::from_utf8(bytes).unwrap_or_default();
            // `units` must still be right, for a consistent definition of valid input.
            if !ascii && text.encode_utf16().count() != units as usize {
                self.pos = start;
                return self.invalid("string lengths do not match");
            }
            return Ok((Value::UNDEFINED, self.heap.atom_str(text)?));
        }
        if units == 0 {
            return Ok((self.empty_string()?, Atom::NULL));
        }
        // A code point above U+00FF starts with a lead byte of 0xC4 or more.
        let two_byte = !ascii && bytes.iter().any(|&b| b >= 0xC4);
        let s = self.heap.alloc_str_uninit(units, two_byte)?;
        let data = Heap::str_data_offset(s);
        let ok = if ascii {
            // units == bytes and valid UTF-8: every byte is ASCII.
            self.heap
                .arena_mut()
                .slice_mut(data, units)
                .copy_from_slice(bytes);
            true
        } else if two_byte {
            let dst = self.heap.arena_mut().slice_mut(data, units * 2);
            transcode(bytes, dst.as_chunks_mut::<2>().0, |d, u| {
                *d = u.to_le_bytes()
            })
        } else {
            // Every code point is at most U+00FF: one byte each.
            let dst = self.heap.arena_mut().slice_mut(data, units);
            transcode(bytes, dst, |d, u| *d = u as u8)
        };
        if !ok {
            self.pos = start;
            return self.invalid("string lengths do not match");
        }
        Ok((Value::string(s), Atom::NULL))
    }

    fn empty_string(&mut self) -> Result<Value, WireError> {
        Ok(Value::string(self.heap.atom_string(atoms::EMPTY)?))
    }
}

/// Integral numbers that fit use the int32 form (as `json::parse` does).
fn number_value(n: f64) -> Value {
    let i = n as i32;
    if i as f64 == n && !(i == 0 && n.is_sign_negative()) {
        Value::int(i)
    } else {
        Value::number(n)
    }
}

/// Transcodes valid UTF-8 into exactly `dst.len()` UTF-16 code units, written with `put`.
/// Returns false if the unit count does not match. ASCII runs go eight bytes at a time.
fn transcode<D>(src: &[u8], dst: &mut [D], put: impl Fn(&mut D, u16)) -> bool {
    let (mut i, mut n) = (0, 0);
    while i < src.len() {
        if i + 8 <= src.len() && n + 8 <= dst.len() {
            let w = u64::from_le_bytes(src[i..i + 8].try_into().unwrap());
            if w & 0x8080_8080_8080_8080 == 0 {
                for (d, &b) in dst[n..n + 8].iter_mut().zip(&src[i..i + 8]) {
                    put(d, b as u16);
                }
                i += 8;
                n += 8;
                continue;
            }
        }
        let b = src[i];
        let (cp, len) = match b {
            0x00..=0x7F => (b as u32, 1),
            0xC0..=0xDF => (((b as u32 & 0x1F) << 6) | (src[i + 1] as u32 & 0x3F), 2),
            0xE0..=0xEF => (
                ((b as u32 & 0x0F) << 12)
                    | ((src[i + 1] as u32 & 0x3F) << 6)
                    | (src[i + 2] as u32 & 0x3F),
                3,
            ),
            _ => (
                ((b as u32 & 0x07) << 18)
                    | ((src[i + 1] as u32 & 0x3F) << 12)
                    | ((src[i + 2] as u32 & 0x3F) << 6)
                    | (src[i + 3] as u32 & 0x3F),
                4,
            ),
        };
        i += len;
        if cp > 0xFFFF {
            let v = cp - 0x10000;
            if n + 2 > dst.len() {
                return false;
            }
            put(&mut dst[n], 0xD800 | (v >> 10) as u16);
            put(&mut dst[n + 1], 0xDC00 | (v & 0x3FF) as u16);
            n += 2;
        } else {
            match dst.get_mut(n) {
                Some(d) => put(d, cp as u16),
                None => return false,
            }
            n += 1;
        }
    }
    n == dst.len()
}
