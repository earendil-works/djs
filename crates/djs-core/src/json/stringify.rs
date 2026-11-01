//! The `JSON.stringify` fast path: SerializeJSONProperty for plain data, without a
//! replacer.
//!
//! It walks the value with an explicit stack in exactly the order of the spec algorithm.
//! When it meets anything that could run user code or needs the full algorithm (an
//! enumerable accessor, a `toJSON` that is an object anywhere on a prototype chain, an
//! object class other than ordinary or array, BigInt, host refs), it returns
//! [`StringifyError::Slow`]. Up to that point it has only read the heap (and flattened
//! ropes, which is invisible), so the caller can rerun the full algorithm from the start.
//! Because everything before the bail-out point was plain data, a cycle or the depth limit
//! found here would be found at the same point by the full algorithm, so those errors are
//! final.
//!
//! The output goes into a growable one-byte buffer that switches to two-byte the first
//! time a code unit above 0xFF is written. Its size is checked against the string length
//! limit and the heap's remaining room (the result has to fit there anyway), so transient
//! memory stays bounded by the memory cap.

use alloc::vec::Vec;

use super::DEFAULT_MAX_DEPTH;
use crate::heap::{
    Atom, Heap, HeapError, Lookup, ObjectClass, Property, StrView, atoms, string::MAX_LEN,
};
use crate::num;
use crate::value::{Kind, Obj, Str, Value};

/// Why [`stringify_fast`] did not produce a result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StringifyError {
    /// The value needs the full algorithm; nothing observable has happened.
    Slow,
    /// `TypeError: circular reference`.
    Circular,
    /// `InternalError: stack overflow` (nesting deeper than [`DEFAULT_MAX_DEPTH`]).
    StackOverflow,
    /// Out of memory, or the result would be longer than the string limit
    /// (`RangeError: invalid string length`).
    Heap(HeapError),
    /// `TypeError: Do not know how to serialize a BigInt` (no `toJSON` on its
    /// prototype chain).
    BigInt,
}

impl StringifyError {
    /// The JS error constructor name and message, as QuickJS reports them. `None` for
    /// [`StringifyError::Slow`].
    pub fn js_error(self) -> Option<(&'static str, &'static str)> {
        match self {
            StringifyError::Slow => None,
            StringifyError::Circular => Some(("TypeError", "circular reference")),
            StringifyError::StackOverflow => Some(("InternalError", "stack overflow")),
            StringifyError::Heap(HeapError::OutOfMemory) => {
                Some(("InternalError", "out of memory"))
            }
            StringifyError::Heap(_) => Some(("RangeError", "invalid string length")),
            StringifyError::BigInt => Some(("TypeError", "Do not know how to serialize a BigInt")),
        }
    }
}

impl From<HeapError> for StringifyError {
    fn from(e: HeapError) -> Self {
        StringifyError::Heap(e)
    }
}

/// The gap for a Number `space` argument: `min(10, ToIntegerOrInfinity(space))` spaces.
pub fn gap_from_number(space: f64) -> Vec<u16> {
    // `as` truncates toward zero and saturates; NaN becomes 0.
    let n = (space.clamp(0.0, 10.0) as usize).min(10);
    alloc::vec![b' ' as u16; n]
}

/// The gap for a String `space` argument: its first 10 code units.
pub fn gap_from_units(space: &[u16]) -> &[u16] {
    &space[..space.len().min(10)]
}

/// `JSON.stringify(value, undefined, space)` for plain data, with `gap` already computed
/// from `space` (see [`gap_from_number`] / [`gap_from_units`]). `Ok(None)` is the
/// `undefined` result (for `undefined`, symbols).
pub fn stringify_fast(
    heap: &mut Heap,
    value: Value,
    gap: &[u16],
) -> Result<Option<Str>, StringifyError> {
    let limit = {
        let a = heap.arena();
        // The result cell needs a 16-byte header plus the data.
        let room = (a.limit() - a.len()).saturating_sub(16) as usize;
        (MAX_LEN as usize, room)
    };
    let mut s = Ser {
        heap,
        out: Out {
            buf: Buf::Narrow(Vec::new()),
            max_len: limit.0,
            room: limit.1,
        },
        gap,
        stack: Vec::new(),
        keys: Vec::new(),
        deep: Vec::new(),
        clean_protos: Vec::new(),
        key_cache: alloc::vec![(Atom::NULL, Vec::new()); KEY_CACHE],
    };
    if !s.run(value)? {
        return Ok(None);
    }
    let str = match s.out.buf {
        Buf::Narrow(b) => s.heap.alloc_latin1(&b)?,
        Buf::Wide(w) => s.heap.alloc_utf16(&w)?,
    };
    Ok(Some(str))
}

enum Buf {
    Narrow(Vec<u8>),
    Wide(Vec<u16>),
}

struct Out {
    buf: Buf,
    max_len: usize,
    room: usize,
}

impl Out {
    /// Fails once the result could no longer become a heap string.
    #[inline]
    fn check(&self) -> Result<(), StringifyError> {
        let (len, bytes) = match &self.buf {
            Buf::Narrow(b) => (b.len(), b.len()),
            Buf::Wide(w) => (w.len(), w.len() * 2),
        };
        if len > self.max_len {
            Err(StringifyError::Heap(HeapError::StringTooLong))
        } else if bytes > self.room {
            Err(StringifyError::Heap(HeapError::OutOfMemory))
        } else {
            Ok(())
        }
    }

    /// Appends Latin-1 (or ASCII) bytes.
    #[inline]
    fn latin1(&mut self, s: &[u8]) {
        match &mut self.buf {
            // Most pieces are short (numbers, punctuation, keys): an inline copy beats a
            // memcpy call; long ones (strings) still get the memcpy.
            Buf::Narrow(b) if s.len() <= 32 => b.extend(s.iter().copied()),
            Buf::Narrow(b) => b.extend_from_slice(s),
            Buf::Wide(w) => w.extend(s.iter().map(|&c| c as u16)),
        }
    }

    #[inline]
    fn byte(&mut self, c: u8) {
        match &mut self.buf {
            Buf::Narrow(b) => b.push(c),
            Buf::Wide(w) => w.push(c as u16),
        }
    }

    #[inline]
    fn unit(&mut self, u: u16) {
        if u <= 0xFF {
            return self.byte(u as u8);
        }
        self.widen();
        if let Buf::Wide(w) = &mut self.buf {
            w.push(u);
        }
    }

    /// Switches to the wide buffer (the result will have a unit above 0xFF).
    fn widen(&mut self) {
        if let Buf::Narrow(b) = &mut self.buf {
            let mut w = Vec::with_capacity(b.len() * 2 + 16);
            w.extend(b.iter().map(|&c| c as u16));
            self.buf = Buf::Wide(w);
        }
    }

    /// Appends little-endian units; the buffer must be wide.
    fn wide_run(&mut self, units: &[[u8; 2]]) {
        if let Buf::Wide(w) = &mut self.buf {
            w.extend(units.iter().map(|p| u16::from_le_bytes(*p)));
        }
    }

    fn units(&mut self, us: &[u16]) {
        for &u in us {
            self.unit(u);
        }
    }

    /// `\uXXXX` with lowercase hex, as QuickJS writes it.
    fn escape_u(&mut self, u: u16) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let n = u as usize;
        self.latin1(&[
            b'\\',
            b'u',
            HEX[n >> 12],
            HEX[(n >> 8) & 15],
            HEX[(n >> 4) & 15],
            HEX[n & 15],
        ]);
    }

    /// The escape for an ASCII unit that QuoteJSONString escapes.
    fn escape_ascii(&mut self, c: u8) {
        match c {
            b'"' => self.latin1(b"\\\""),
            b'\\' => self.latin1(b"\\\\"),
            0x08 => self.latin1(b"\\b"),
            0x09 => self.latin1(b"\\t"),
            0x0A => self.latin1(b"\\n"),
            0x0C => self.latin1(b"\\f"),
            0x0D => self.latin1(b"\\r"),
            _ => self.escape_u(c as u16),
        }
    }

    /// QuoteJSONString.
    fn quote(&mut self, v: StrView<'_>) {
        self.byte(b'"');
        match v {
            StrView::Latin1(b) => {
                let mut run = 0;
                let mut i = 0;
                while i < b.len() {
                    // Skip 8 bytes at a time while none needs escaping.
                    if let Some(w) = b.get(i..i + 8)
                        && !needs_escape8(u64::from_le_bytes(w.try_into().unwrap()))
                    {
                        i += 8;
                        continue;
                    }
                    let c = b[i];
                    if c < 0x20 || c == b'"' || c == b'\\' {
                        self.latin1(&b[run..i]);
                        self.escape_ascii(c);
                        run = i + 1;
                    }
                    i += 1;
                }
                self.latin1(&b[run..]);
            }
            StrView::Utf16(bytes) => {
                // A two-byte string has a unit above 0xFF (the width invariant), so the
                // output becomes wide anyway: switch once, then copy unescaped runs in
                // bulk instead of unit by unit.
                self.widen();
                let units = bytes.as_chunks::<2>().0;
                let mut run = 0;
                let mut i = 0;
                while i < units.len() {
                    let u = u16::from_le_bytes(units[i]);
                    if !matches!(u, 0..=0x1F | 0x22 | 0x5C | 0xD800..=0xDFFF) {
                        i += 1;
                        continue;
                    }
                    self.wide_run(&units[run..i]);
                    i += 1;
                    match u {
                        0xD800..=0xDBFF
                            if i < units.len()
                                && (0xDC00..=0xDFFF).contains(&u16::from_le_bytes(units[i])) =>
                        {
                            // A valid pair is copied as is; the run continues after it.
                            run = i - 1;
                            i += 1;
                            continue;
                        }
                        0xD800..=0xDFFF => self.escape_u(u),
                        _ => self.escape_ascii(u as u8),
                    }
                    run = i;
                }
                self.wide_run(&units[run..]);
            }
        }
        self.byte(b'"');
    }
}

/// Whether any of the 8 bytes in `w` is below 0x20, `"` or `\\` (SWAR: a byte is zero
/// after the subtraction or xor exactly when it matched).
#[inline]
fn needs_escape8(w: u64) -> bool {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let lt = |x: u64, n: u64| x.wrapping_sub(ONES * n) & !x & HIGH;
    let zero = |x: u64| x.wrapping_sub(ONES) & !x & HIGH;
    (lt(w, 0x20) | zero(w ^ (ONES * 0x22)) | zero(w ^ (ONES * 0x5C))) != 0
}

/// Writes `n` in decimal.
fn write_int(out: &mut Out, n: i64) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let neg = n < 0;
    let mut m = n.unsigned_abs();
    loop {
        i -= 1;
        buf[i] = b'0' + (m % 10) as u8;
        m /= 10;
        if m == 0 {
            break;
        }
    }
    if neg {
        out.byte(b'-');
    }
    out.latin1(&buf[i..]);
}

#[derive(Clone, Copy)]
enum Frame {
    /// A plain object (see `Heap::plain_entry_slots`): serializing its entry slots
    /// `next..count` directly.
    Plain {
        o: Obj,
        next: u32,
        count: u32,
        any: bool,
    },
    /// Serializing the keys `next..end` of `keys`; the frame owns `start..end`.
    Obj {
        o: Obj,
        start: usize,
        next: usize,
        end: usize,
        any: bool,
    },
    Arr {
        o: Obj,
        next: u32,
        len: u32,
    },
}

impl Frame {
    fn obj(&self) -> Obj {
        match *self {
            Frame::Plain { o, .. } | Frame::Obj { o, .. } | Frame::Arr { o, .. } => o,
        }
    }
}

/// How a value serializes.
enum Class {
    /// `undefined`: skipped in objects, `null` in arrays.
    Skip,
    Primitive,
    Object(Obj, bool),
}

/// Frames beyond this many are also kept in a set, so the cycle check stays cheap for deep
/// structures.
const LINEAR_CYCLE_CHECK: usize = 32;

struct Ser<'h, 'g> {
    heap: &'h mut Heap,
    out: Out,
    gap: &'g [u16],
    stack: Vec<Frame>,
    keys: Vec<Atom>,
    /// Sorted offsets of the frames beyond the first [`LINEAR_CYCLE_CHECK`].
    deep: Vec<u32>,
    /// Prototypes whose chain is known to have no (object-valued) `toJSON`. Nothing can
    /// change during one call, so the answer stays valid.
    clean_protos: Vec<Obj>,
    /// Quoted Latin-1 keys by atom, direct-mapped: arrays of similar objects repeat keys.
    key_cache: Vec<(Atom, Vec<u8>)>,
}

const KEY_CACHE: usize = 128;
const CLEAN_PROTOS: usize = 8;

/// Whether a `toJSON` lookup result means user code might run.
fn to_json_blocks(p: Option<Property>) -> bool {
    match p {
        None => false,
        Some(Property::Data { value, .. }) => value.as_object().is_some(),
        Some(Property::Accessor { .. }) => true,
    }
}

impl Ser<'_, '_> {
    /// Fails with `Slow` if `toJSON` on `o` (own or inherited) might be callable.
    fn check_to_json(&mut self, o: Obj) -> Result<(), StringifyError> {
        let own = self.heap.get_own_property(o, atoms::TO_JSON);
        if own.is_some() {
            return if to_json_blocks(own) {
                Err(StringifyError::Slow)
            } else {
                Ok(())
            };
        }
        let Some(p) = self.heap.get_prototype_of(o) else {
            return Ok(());
        };
        if self.clean_protos.contains(&p) {
            return Ok(());
        }
        // A proxy on the chain answers with its `get` trap (user code).
        let found = match self.heap.lookup_property(p, atoms::TO_JSON) {
            Lookup::Found(_, p) => Some(p),
            Lookup::Absent => None,
            Lookup::Proxy(_) => return Err(StringifyError::Slow),
        };
        if to_json_blocks(found) {
            return Err(StringifyError::Slow);
        }
        if self.clean_protos.len() < CLEAN_PROTOS {
            self.clean_protos.push(p);
        }
        Ok(())
    }

    fn classify(&mut self, v: Value) -> Result<Class, StringifyError> {
        Ok(match v.kind() {
            Kind::Undefined | Kind::Symbol => Class::Skip,
            Kind::Null | Kind::Bool | Kind::Number | Kind::String => Class::Primitive,
            Kind::Object => {
                let o = v.as_object().unwrap();
                let is_array = match self.heap.object_class(o) {
                    ObjectClass::Ordinary => false,
                    ObjectClass::Array => true,
                    // Functions (skipped), wrappers (ToPrimitive), exotics (proxies
                    // included): full algorithm.
                    #[allow(unreachable_patterns)]
                    _ => return Err(StringifyError::Slow),
                };
                // A toJSON that might be callable means calling user code.
                self.check_to_json(o)?;
                Class::Object(o, is_array)
            }
            Kind::BigInt => {
                // A BigInt.prototype.toJSON (or one further up) is user code.
                if self.heap.realm().is_undefined() {
                    return Err(StringifyError::BigInt);
                }
                let proto = crate::interp::realm::primitive_proto(self.heap, v).unwrap();
                let found = self
                    .heap
                    .find_property(proto, atoms::TO_JSON)
                    .map(|(_, p)| p);
                return Err(if found.is_some() {
                    StringifyError::Slow
                } else {
                    StringifyError::BigInt
                });
            }
            Kind::HostRef | Kind::Hole | Kind::Uninitialized | Kind::Internal => {
                return Err(StringifyError::Slow);
            }
        })
    }

    fn newline(&mut self, depth: usize) {
        if self.gap.is_empty() {
            return;
        }
        self.out.byte(b'\n');
        for _ in 0..depth {
            self.out.units(self.gap);
        }
    }

    /// Writes a primitive.
    fn primitive(&mut self, v: Value) -> Result<(), StringifyError> {
        if let Some(i) = v.as_int() {
            write_int(&mut self.out, i as i64);
        } else if let Some(n) = v.as_number() {
            if n.is_finite() {
                self.out.latin1(num::to_string(n).as_bytes());
            } else {
                self.out.latin1(b"null");
            }
        } else if let Some(b) = v.as_bool() {
            self.out.latin1(if b { b"true" } else { b"false" });
        } else if v.is_null() {
            self.out.latin1(b"null");
        } else if let Some(s) = v.as_string() {
            let flat = self.heap.flatten(s)?;
            let view = self.heap.str_view(flat).expect("flat");
            self.out.quote(view);
        }
        self.out.check()
    }

    /// Writes the key of an object member.
    fn key(&mut self, k: Atom) {
        if let Some(i) = k.as_int() {
            self.out.byte(b'"');
            write_int(&mut self.out, i as i64);
            self.out.byte(b'"');
        } else {
            let slot = k.id() as usize % KEY_CACHE;
            if self.key_cache[slot].0 == k {
                self.out.latin1(&self.key_cache[slot].1);
                return;
            }
            let s = self
                .heap
                .atom_string(k)
                .expect("string atoms do not allocate");
            let view = self.heap.str_view(s).expect("atom strings are flat");
            if let StrView::Latin1(_) = view {
                let mut q = Out {
                    buf: Buf::Narrow(Vec::new()),
                    max_len: usize::MAX,
                    room: usize::MAX,
                };
                q.quote(view);
                if let Buf::Narrow(b) = q.buf {
                    self.out.latin1(&b);
                    self.key_cache[slot] = (k, b);
                }
            } else {
                self.out.quote(view);
            }
        }
    }

    /// Writes a value of class `c`: a primitive, or the opening of a container.
    fn emit(&mut self, v: Value, c: Class) -> Result<(), StringifyError> {
        let (o, is_array) = match c {
            Class::Skip => unreachable!(),
            Class::Primitive => return self.primitive(v),
            Class::Object(o, a) => (o, a),
        };
        let n = self.stack.len();
        if self.stack[..n.min(LINEAR_CYCLE_CHECK)]
            .iter()
            .any(|f| f.obj() == o)
            || self.deep.binary_search(&o.offset()).is_ok()
        {
            return Err(StringifyError::Circular);
        }
        if n >= DEFAULT_MAX_DEPTH as usize {
            return Err(StringifyError::StackOverflow);
        }
        if n >= LINEAR_CYCLE_CHECK
            && let Err(i) = self.deep.binary_search(&o.offset())
        {
            self.deep.insert(i, o.offset());
        }
        // Containers without primitives (`[[],[]]` with a long gap) must not grow the
        // output unchecked either.
        self.out.check()?;
        if is_array {
            self.out.byte(b'[');
            let len = self.heap.array_length(o);
            self.stack.push(Frame::Arr { o, next: 0, len });
        } else if let Some(count) = self.heap.plain_entry_slots(o) {
            self.out.byte(b'{');
            self.stack.push(Frame::Plain {
                o,
                next: 0,
                count,
                any: false,
            });
        } else {
            self.out.byte(b'{');
            let start = self.keys.len();
            for k in self.heap.own_keys(o) {
                if !k.is_symbol() {
                    self.keys.push(k);
                }
            }
            let end = self.keys.len();
            self.stack.push(Frame::Obj {
                o,
                start,
                next: start,
                end,
                any: false,
            });
        }
        Ok(())
    }

    fn pop(&mut self) {
        let f = self.stack.pop().unwrap();
        if self.stack.len() >= LINEAR_CYCLE_CHECK
            && let Ok(i) = self.deep.binary_search(&f.obj().offset())
        {
            self.deep.remove(i);
        }
        if let Frame::Obj { start, end, .. } = f {
            // Keys of nested frames were pushed after ours and are gone already.
            debug_assert_eq!(self.keys.len(), end);
            self.keys.truncate(start);
        }
    }

    /// Array element `i` ([[Get]]), or `Slow` if that would call a getter.
    fn element(&mut self, o: Obj, i: u32) -> Result<Value, StringifyError> {
        if let Some(v) = self.heap.get_element_fast(o, i) {
            return Ok(v);
        }
        let key = self.heap.atom_from_index(i)?;
        match self.heap.lookup_property(o, key) {
            Lookup::Absent => Ok(Value::UNDEFINED),
            Lookup::Found(_, Property::Data { value, .. }) => Ok(value),
            Lookup::Found(_, Property::Accessor { .. }) | Lookup::Proxy(_) => {
                Err(StringifyError::Slow)
            }
        }
    }

    /// The value of an own property as an object member: `None` if it is skipped
    /// (non-enumerable, or serializes to `undefined`), `Slow` for an enumerable accessor.
    fn member_value(&mut self, p: Property) -> Result<Option<(Value, Class)>, StringifyError> {
        let value = match p {
            Property::Data { value, flags } if flags.enumerable() => value,
            Property::Accessor { flags, .. } if flags.enumerable() => {
                return Err(StringifyError::Slow);
            }
            _ => return Ok(None),
        };
        Ok(match self.classify(value)? {
            Class::Skip => None,
            c => Some((value, c)),
        })
    }

    /// Writes `,`, the indentation, `"key":` and the value.
    fn member(
        &mut self,
        key: Atom,
        any: bool,
        depth: usize,
        value: Value,
        c: Class,
    ) -> Result<(), StringifyError> {
        if any {
            self.out.byte(b',');
        }
        self.newline(depth);
        self.key(key);
        self.out.byte(b':');
        if !self.gap.is_empty() {
            self.out.byte(b' ');
        }
        self.emit(value, c)
    }

    /// Serializes `v`; `false` if the result is `undefined`.
    fn run(&mut self, v: Value) -> Result<bool, StringifyError> {
        match self.classify(v)? {
            Class::Skip => return Ok(false),
            c => self.emit(v, c)?,
        }
        while let Some(&frame) = self.stack.last() {
            let top = self.stack.len() - 1;
            let depth = self.stack.len();
            match frame {
                Frame::Plain {
                    o,
                    next,
                    count,
                    any,
                } => {
                    if next == count {
                        if any {
                            self.newline(depth - 1);
                        }
                        self.out.byte(b'}');
                        self.pop();
                        continue;
                    }
                    self.stack[top] = Frame::Plain {
                        o,
                        next: next + 1,
                        count,
                        any,
                    };
                    let Some((key, p)) = self.heap.plain_entry(o, next) else {
                        continue;
                    };
                    if key.is_symbol() {
                        continue;
                    }
                    let Some((value, c)) = self.member_value(p)? else {
                        continue;
                    };
                    self.stack[top] = Frame::Plain {
                        o,
                        next: next + 1,
                        count,
                        any: true,
                    };
                    self.member(key, any, depth, value, c)?;
                }
                Frame::Obj {
                    o,
                    start,
                    next,
                    end,
                    any,
                } => {
                    if next == end {
                        if any {
                            self.newline(depth - 1);
                        }
                        self.out.byte(b'}');
                        self.pop();
                        continue;
                    }
                    let key = self.keys[next];
                    self.stack[top] = Frame::Obj {
                        o,
                        start,
                        next: next + 1,
                        end,
                        any,
                    };
                    let Some(p) = self.heap.get_own_property(o, key) else {
                        continue;
                    };
                    let Some((value, c)) = self.member_value(p)? else {
                        continue;
                    };
                    self.stack[top] = Frame::Obj {
                        o,
                        start,
                        next: next + 1,
                        end,
                        any: true,
                    };
                    self.member(key, any, depth, value, c)?;
                }
                Frame::Arr { o, next, len } => {
                    if next == len {
                        if len > 0 {
                            self.newline(depth - 1);
                        }
                        self.out.byte(b']');
                        self.pop();
                        continue;
                    }
                    self.stack[top] = Frame::Arr {
                        o,
                        next: next + 1,
                        len,
                    };
                    if next > 0 {
                        self.out.byte(b',');
                    }
                    self.newline(depth);
                    let value = self.element(o, next)?;
                    match self.classify(value)? {
                        Class::Skip => {
                            self.out.latin1(b"null");
                            self.out.check()?;
                        }
                        c => self.emit(value, c)?,
                    }
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::needs_escape8;

    #[test]
    fn swar_escape_scan_is_exact() {
        for b in 0..=255u8 {
            for pos in 0..8 {
                let mut w = [b'a'; 8];
                w[pos] = b;
                let want = b < 0x20 || b == b'"' || b == b'\\';
                assert_eq!(
                    needs_escape8(u64::from_le_bytes(w)),
                    want,
                    "{b:#x} at {pos}"
                );
                // With high Latin-1 bytes around it.
                let mut w = [0xE9; 8];
                w[pos] = b;
                assert_eq!(
                    needs_escape8(u64::from_le_bytes(w)),
                    want,
                    "{b:#x} at {pos}"
                );
            }
        }
    }
}
