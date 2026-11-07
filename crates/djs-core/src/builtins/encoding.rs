//! TextEncoder and TextDecoder, UTF-8 only (the WHATWG Encoding standard; 0244, D3172):
//! the leaf work of builtins/encoding.ts. A TextEncoder or TextDecoder is an object of
//! class `TextCodec` with two slots: the flags (int32: [`DECODER`], [`FATAL`],
//! [`IGNORE_BOM`]) and a decoder's stream state (int32: the up to three bytes of an
//! incomplete sequence held back by `stream: true` in bits 0..24, their number in bits
//! 24..26, then [`BOM_SEEN`] and [`DO_NOT_FLUSH`]).

use alloc::vec::Vec;

use crate::buf::vec_with_capacity;
use crate::heap::cells::Bytes;
use crate::heap::string::is_ascii;
use crate::heap::typed::Kind;
use crate::heap::{Heap, ObjectClass, StrView};
use crate::interp::conv::{Fault, type_error};
use crate::interp::{Intrinsic, realm};
use crate::value::{Obj, Str, Value};

/// Flag bits of a codec's slot 0.
pub(crate) const DECODER: i32 = 1;
pub(crate) const FATAL: i32 = 2;
pub(crate) const IGNORE_BOM: i32 = 4;
/// State bits of a decoder's slot 1 (the spec's BOM seen and do not flush).
const BOM_SEEN: u32 = 1 << 26;
const DO_NOT_FLUSH: u32 = 1 << 27;

/// The labels of UTF-8, the only encoding djs has (Encoding standard, 4.2).
const LABELS: [&[u8]; 6] = [
    b"unicode-1-1-utf-8",
    b"unicode11utf8",
    b"unicode20utf8",
    b"utf-8",
    b"utf8",
    b"x-unicode20utf8",
];

/// A new TextEncoder (flags 0) or TextDecoder; `None` if `label` is not one of UTF-8's
/// (get an encoding: ASCII whitespace trimmed, ASCII case-insensitive).
pub(crate) fn create(heap: &mut Heap, proto: Obj, flags: i32, label: Str) -> Result<Value, Fault> {
    let units: Vec<u16> = heap.str_units(label).collect();
    let ws = |u: &u16| matches!(*u, 0x09 | 0x0A | 0x0C | 0x0D | 0x20);
    let start = units.iter().position(|u| !ws(u)).unwrap_or(units.len());
    let end = units.iter().rposition(|u| !ws(u)).map_or(start, |i| i + 1);
    let t = &units[start..end];
    let known = LABELS.iter().any(|l| {
        l.len() == t.len()
            && l.iter()
                .zip(t)
                .all(|(&c, &u)| u < 0x80 && (u as u8).to_ascii_lowercase() == c)
    });
    if !known {
        return Ok(Value::UNDEFINED);
    }
    let o = heap.alloc_object(ObjectClass::TextCodec, Some(proto), 2)?;
    heap.set_object_slot(o, 0, Value::int(flags));
    heap.set_object_slot(o, 1, Value::int(0));
    Ok(Value::object(o))
}

/// The flags of `v` if it is a TextDecoder (`decoder`) or a TextEncoder, else WebIDL's
/// TypeError (in node's words).
pub(crate) fn flags(heap: &Heap, v: Value, decoder: bool) -> Result<i32, Fault> {
    if let Some(o) = v.as_object()
        && heap.object_class(o) == ObjectClass::TextCodec
    {
        let f = heap.object_slot(o, 0).as_int().unwrap_or(0);
        if (f & DECODER != 0) == decoder {
            return Ok(f);
        }
    }
    type_error(if decoder {
        "Value of \"this\" must be of type TextDecoder"
    } else {
        "Value of \"this\" must be of type TextEncoder"
    })
}

/// Where the bytes a BufferSource holds now are in the arena: an ArrayBuffer, a typed
/// array or a DataView (no bytes when detached or out of bounds); `None` for anything else.
pub(crate) fn source_range(heap: &Heap, v: Value) -> Option<(u32, u32)> {
    let o = v.as_object()?;
    let (buffer, off, len) = match heap.object_class(o) {
        ObjectClass::ArrayBuffer => (o, 0, heap.buffer_len(o)),
        ObjectClass::TypedArray => match heap.ta_length(o) {
            Some(n) => (
                heap.ta_buffer(o),
                heap.ta_byte_offset(o),
                n * heap.ta_kind(o).size(),
            ),
            None => return Some((0, 0)),
        },
        ObjectClass::DataView => {
            let buffer = heap.object_slot(o, 0).as_object()?;
            let num = |i| heap.object_slot(o, i).as_number().unwrap_or(0.0) as u32;
            let off = num(1);
            // A negative length (-1) tracks the buffer's.
            let len = if heap.object_slot(o, 2).as_number().unwrap_or(0.0) < 0.0 {
                heap.buffer_len(buffer).saturating_sub(off)
            } else {
                num(2)
            };
            (buffer, off, len)
        }
        _ => return None,
    };
    match heap.buffer_bytes(buffer) {
        Some(b) if off as u64 + len as u64 <= heap.buffer_len(buffer) as u64 => {
            Some((Bytes::DATA.at(b, 0) + off, len))
        }
        _ => Some((0, 0)),
    }
}

/// The length of an incomplete (but so far valid) UTF-8 sequence at the end of `b`, 0 if
/// there is none: what `stream: true` holds back for the next call.
fn incomplete_tail(b: &[u8]) -> usize {
    for k in 1..=b.len().min(3) {
        if b[b.len() - k] & 0xC0 != 0x80 {
            return match core::str::from_utf8(&b[b.len() - k..]) {
                Err(e) if e.valid_up_to() == 0 && e.error_len().is_none() => k,
                _ => 0,
            };
        }
    }
    0
}

/// TextDecoder.prototype.decode's steps after the conversions: decodes `input` (after
/// the bytes the last `stream: true` call held back) with the UTF-8 decoder, a U+FFFD
/// per maximal invalid subpart (which `utf8_chunks` gives). `undefined` for invalid
/// input to a fatal decoder, which then starts over.
pub(crate) fn decode(
    heap: &mut Heap,
    d: Obj,
    input: Value,
    stream: bool,
    fuel: &mut i64,
) -> Result<Value, Fault> {
    let flags = heap.object_slot(d, 0).as_int().unwrap_or(0);
    let mut state = heap.object_slot(d, 1).as_int().unwrap_or(0) as u32;
    if state & DO_NOT_FLUSH == 0 {
        state = 0;
    }
    let (at, len) = source_range(heap, input).unwrap_or((0, 0));
    let held = (state >> 24 & 3) as usize;
    let mut joined: Vec<u8> = Vec::new();
    if held != 0 {
        joined = vec_with_capacity(held + len as usize);
        joined.extend((0..held).map(|i| (state >> (8 * i)) as u8));
        joined.extend_from_slice(heap.arena().slice(at, len));
    }
    let bytes = if held == 0 {
        heap.arena().slice(at, len)
    } else {
        &joined
    };
    *fuel -= (bytes.len() / 8) as i64;
    let keep = if stream { incomplete_tail(bytes) } else { 0 };
    let body = &bytes[..bytes.len() - keep];
    let mut next = 0;
    if stream {
        next = DO_NOT_FLUSH | state & BOM_SEEN | (keep as u32) << 24;
        for (i, &b) in bytes[body.len()..].iter().enumerate() {
            next |= (b as u32) << (8 * i);
        }
    }
    if !body.is_empty() {
        next |= BOM_SEEN;
    }
    // ASCII input is copied straight into the string (no BOM to strip).
    let n = body.len() as u32;
    let units = if held == 0 && is_ascii(body) {
        None
    } else {
        let mut units: Vec<u16> = vec_with_capacity(body.len());
        for chunk in body.utf8_chunks() {
            units.extend(chunk.valid().encode_utf16());
            if !chunk.invalid().is_empty() {
                if flags & FATAL != 0 {
                    heap.set_object_slot(d, 1, Value::int(0));
                    return Ok(Value::UNDEFINED);
                }
                units.push(0xFFFD);
            }
        }
        Some(units)
    };
    let s = match units {
        None => {
            let s = heap.alloc_str_uninit(n, false)?;
            heap.arena_mut()
                .copy_within(at, n, Heap::str_data_offset(s));
            s
        }
        Some(units) => {
            let strip = flags & IGNORE_BOM == 0 && state & BOM_SEEN == 0;
            let skip = usize::from(strip && units.first() == Some(&0xFEFF));
            heap.alloc_utf16(&units[skip..])?
        }
    };
    heap.set_object_slot(d, 1, Value::int(next as i32));
    Ok(Value::string(s))
}

/// UTF-8 output of a string's code units, at most `cap` bytes: a lone surrogate becomes
/// U+FFFD, and a code point that does not fit ends it (encodeInto).
struct Encoder {
    out: Vec<u8>,
    cap: usize,
    /// Code units consumed so far (encodeInto's `read`)
    read: u32,
    /// A high surrogate waiting for its low half, 0 if none
    high: u16,
    full: bool,
}

impl Encoder {
    fn unit(&mut self, u: u16) {
        if self.high != 0 {
            let h = self.high;
            self.high = 0;
            if (0xDC00..0xE000).contains(&u) {
                let c = 0x10000 + ((u32::from(h) - 0xD800) << 10 | (u32::from(u) - 0xDC00));
                return self.put(c, 2);
            }
            self.put(0xFFFD, 1);
        }
        if (0xD800..0xDC00).contains(&u) {
            self.high = u;
        } else if (0xDC00..0xE000).contains(&u) {
            self.put(0xFFFD, 1);
        } else {
            self.put(u32::from(u), 1);
        }
    }

    fn put(&mut self, c: u32, units: u32) {
        let mut buf = [0; 4];
        let b = char::from_u32(c)
            .unwrap_or('\u{FFFD}')
            .encode_utf8(&mut buf);
        if self.full || self.out.len() + b.len() > self.cap {
            self.full = true;
            return;
        }
        self.out.extend_from_slice(b.as_bytes());
        self.read += units;
    }
}

/// The UTF-8 of `s` (USVString: lone surrogates as U+FFFD), at most `cap` bytes, and the
/// number of code units it covers. ASCII Latin-1 chunks are copied whole.
pub(crate) fn encode(heap: &Heap, s: Str, cap: usize, fuel: &mut i64) -> (Vec<u8>, u32) {
    let len = heap.str_len(s) as usize;
    *fuel -= (len / 8) as i64;
    let mut e = Encoder {
        out: vec_with_capacity(len.min(cap)),
        cap,
        read: 0,
        high: 0,
        full: false,
    };
    for chunk in heap.str_chunks(s) {
        if e.full {
            break;
        }
        match chunk {
            StrView::Latin1(b) if e.high == 0 && is_ascii(b) && e.out.len() + b.len() <= cap => {
                e.out.extend_from_slice(b);
                e.read += b.len() as u32;
            }
            c => c.units().for_each(|u| e.unit(u)),
        }
    }
    if e.high != 0 {
        e.put(0xFFFD, 1);
    }
    (e.out, e.read)
}

/// TextEncoder.prototype.encode: a new Uint8Array of the UTF-8 of `s`.
pub(crate) fn encode_new(heap: &mut Heap, s: Str, fuel: &mut i64) -> Result<Value, Fault> {
    let (bytes, _) = encode(heap, s, usize::MAX, fuel);
    let bp = realm::intrinsic_obj(heap, Intrinsic::ArrayBufferProto);
    let p = realm::intrinsic_obj(heap, Intrinsic::Uint8ArrayProto);
    Ok(Value::object(heap.new_uint8_array(
        &bytes,
        Some(bp),
        Some(p),
    )?))
}

/// TextEncoder.prototype.encodeInto: writes the UTF-8 of `s` into the Uint8Array `dest`
/// as far as whole code points fit; `{ read, written }`, or `undefined` if `dest` is not
/// a Uint8Array.
pub(crate) fn encode_into(
    heap: &mut Heap,
    s: Str,
    dest: Value,
    fuel: &mut i64,
) -> Result<Value, Fault> {
    let Some(o) = dest.as_object().filter(|&o| {
        heap.object_class(o) == ObjectClass::TypedArray && heap.ta_kind(o) == Kind::Uint8
    }) else {
        return Ok(Value::UNDEFINED);
    };
    let room = heap.ta_length(o).unwrap_or(0) as usize;
    let (bytes, read) = encode(heap, s, room, fuel);
    if !bytes.is_empty() {
        let off = heap.ta_byte_offset(o) as usize;
        let buffer = heap.ta_buffer(o);
        heap.buffer_data_mut(buffer)[off..off + bytes.len()].copy_from_slice(&bytes);
    }
    let keys = [heap.atom_str("read")?, heap.atom_str("written")?];
    let n = |x: usize| i32::try_from(x).map_or(Value::number(x as f64), Value::int);
    let values = [n(read as usize), n(bytes.len())];
    let proto = realm::intrinsic_obj(heap, Intrinsic::ObjectProto);
    Ok(Value::object(heap.new_object_from_entries(
        Some(proto),
        &keys,
        &values,
    )?))
}
