//! The byte-level encoding of a logical snapshot: LEB128 varints, zigzag integers,
//! little-endian f64s, and length-prefixed byte strings.

use alloc::vec::Vec;

/// References to strings and records: 0 for the next one not referred to yet in this
/// stream (a graph written breadth-first mostly refers to new records and strings), else
/// the id + 1. Both forms repeat (keys refer to the same strings, objects to the same
/// prototypes), so they compress.
#[derive(Default)]
pub(super) struct Writer {
    pub buf: Vec<u8>,
    last_str: i64,
    last_rec: i64,
}

impl Writer {
    fn next_or_abs(&mut self, id: u32, next: fn(&mut Writer) -> &mut i64) {
        let n = next(self);
        if i64::from(id) == *n {
            *n += 1;
            self.u(0);
        } else {
            if i64::from(id) > *n {
                *n = i64::from(id) + 1;
            }
            self.u(u64::from(id) + 1);
        }
    }

    pub fn str_ref(&mut self, id: u32) {
        self.next_or_abs(id, |w| &mut w.last_str);
    }

    pub fn rec_ref(&mut self, id: u32) {
        self.next_or_abs(id, |w| &mut w.last_rec);
    }

    pub fn u(&mut self, mut v: u64) {
        loop {
            let b = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                self.buf.push(b);
                return;
            }
            self.buf.push(b | 0x80);
        }
    }

    pub fn i(&mut self, v: i64) {
        self.u(((v << 1) ^ (v >> 63)) as u64);
    }

    pub fn byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    pub fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
    }

    pub fn u64_raw(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.u(b.len() as u64);
        self.buf.extend_from_slice(b);
    }

    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

pub(super) struct Reader<'a> {
    pub buf: &'a [u8],
    pub at: usize,
    last_str: i64,
    last_rec: i64,
}

/// A truncated or malformed snapshot.
#[derive(Debug)]
pub(super) struct Short;

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader {
            buf,
            at: 0,
            last_str: 0,
            last_rec: 0,
        }
    }

    /// A new stream of references starts (a section the exporter wrote separately).
    pub fn reset_refs(&mut self) {
        self.last_str = 0;
        self.last_rec = 0;
    }

    fn next_or_abs(&mut self, str: bool) -> Result<u32, Short> {
        let u = self.u()?;
        let n = if str {
            &mut self.last_str
        } else {
            &mut self.last_rec
        };
        let id = if u == 0 {
            let id = *n;
            *n += 1;
            id
        } else {
            let id = (u - 1) as i64;
            if id > *n {
                *n = id + 1;
            }
            id
        };
        u32::try_from(id).map_err(|_| Short)
    }

    pub fn str_ref(&mut self) -> Result<u32, Short> {
        self.next_or_abs(true)
    }

    pub fn rec_ref(&mut self) -> Result<u32, Short> {
        self.next_or_abs(false)
    }

    pub fn u(&mut self) -> Result<u64, Short> {
        let mut v = 0u64;
        let mut shift = 0;
        loop {
            let b = *self.buf.get(self.at).ok_or(Short)?;
            self.at += 1;
            if shift >= 64 {
                return Err(Short);
            }
            v |= u64::from(b & 0x7F) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
        }
    }

    pub fn u32(&mut self) -> Result<u32, Short> {
        u32::try_from(self.u()?).map_err(|_| Short)
    }

    pub fn i(&mut self) -> Result<i64, Short> {
        let u = self.u()?;
        Ok(((u >> 1) as i64) ^ -((u & 1) as i64))
    }

    pub fn byte(&mut self) -> Result<u8, Short> {
        let b = *self.buf.get(self.at).ok_or(Short)?;
        self.at += 1;
        Ok(b)
    }

    pub fn raw(&mut self, n: usize) -> Result<&'a [u8], Short> {
        let end = self.at.checked_add(n).ok_or(Short)?;
        let s = self.buf.get(self.at..end).ok_or(Short)?;
        self.at = end;
        Ok(s)
    }

    pub fn f64(&mut self) -> Result<f64, Short> {
        Ok(f64::from_bits(self.u64_raw()?))
    }

    pub fn u64_raw(&mut self) -> Result<u64, Short> {
        let b = self.raw(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], Short> {
        let n = self.u()? as usize;
        self.raw(n)
    }

    pub fn str(&mut self) -> Result<&'a str, Short> {
        core::str::from_utf8(self.bytes()?).map_err(|_| Short)
    }
}

/// FNV-1a, 64 bits: the bytecode and schema digests.
#[derive(Clone, Copy)]
pub(super) struct Fnv(pub u64);

impl Fnv {
    pub fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    pub fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= u64::from(x);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    /// A string with its length, so that concatenations differ.
    pub fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes(s.as_bytes());
    }
}
