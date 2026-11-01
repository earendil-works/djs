// The kernels of kernels.c in idiomatic safe Rust, the way djs writes it (0123): slices
// and indexing, `Result` with an error struct and `?`, `Vec` for the growable buffer.
// no_std cdylib with an imported allocator, so only the kernels' own code is measured.
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};

unsafe extern "C" {
    fn host_realloc(p: *mut u8, n: usize) -> *mut u8;
    fn host_free(p: *mut u8);
}

struct Host;
unsafe impl GlobalAlloc for Host {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe { host_realloc(core::ptr::null_mut(), l.size()) }
    }
    unsafe fn dealloc(&self, p: *mut u8, _: Layout) {
        unsafe { host_free(p) }
    }
    unsafe fn realloc(&self, p: *mut u8, _: Layout, n: usize) -> *mut u8 {
        unsafe { host_realloc(p, n) }
    }
}

#[global_allocator]
static A: Host = Host;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// The exports take raw parts (the host ABI), the kernels themselves are safe Rust.
fn bytes<'a>(p: *const u8, n: usize) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(p, n) }
}

// 1. UTF-8 to code points; the count, None on a bad sequence.
fn utf8(s: &[u8], out: &mut [u32]) -> Option<usize> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i] as u32;
        i += 1;
        let c = if c < 0x80 {
            c
        } else if c & 0xe0 == 0xc0 && i < s.len() {
            i += 1;
            ((c & 0x1f) << 6) | (s[i - 1] as u32 & 0x3f)
        } else if c & 0xf0 == 0xe0 && s.len() - i >= 2 {
            i += 2;
            ((c & 0x0f) << 12) | ((s[i - 2] as u32 & 0x3f) << 6) | (s[i - 1] as u32 & 0x3f)
        } else if c & 0xf8 == 0xf0 && s.len() - i >= 3 {
            i += 3;
            ((c & 0x07) << 18)
                | ((s[i - 3] as u32 & 0x3f) << 12)
                | ((s[i - 2] as u32 & 0x3f) << 6)
                | (s[i - 1] as u32 & 0x3f)
        } else {
            return None;
        };
        out[n] = c;
        n += 1;
    }
    Some(n)
}

#[unsafe(no_mangle)]
pub extern "C" fn k_utf8(p: *const u8, n: usize, out: *mut u32) -> i32 {
    let out = unsafe { core::slice::from_raw_parts_mut(out, n) };
    utf8(bytes(p, n), out).map_or(-1, |n| n as i32)
}

// 2. Open-addressing lookup.
fn lookup(keys: &[u32], mask: u32, key: u32) -> Option<usize> {
    let mut h = key.wrapping_mul(0x9e3779b1) & mask;
    loop {
        let k = keys[h as usize];
        if k == key {
            return Some(h as usize);
        }
        if k == 0 {
            return None;
        }
        h = (h + 1) & mask;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn k_lookup(keys: *const u32, mask: u32, key: u32) -> i32 {
    let keys = unsafe { core::slice::from_raw_parts(keys, mask as usize + 1) };
    lookup(keys, mask, key).map_or(-1, |i| i as i32)
}

// 3. Number scanner.
fn scan(s: &[u8], mut i: usize) -> (f64, usize) {
    let mut v = 0.0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v * 10.0 + (s[i] - b'0') as f64;
        i += 1;
    }
    if i < s.len() && s[i] == b'.' {
        let mut f = 0.1;
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            v += (s[i] - b'0') as f64 * f;
            f *= 0.1;
            i += 1;
        }
    }
    (v, i)
}

#[unsafe(no_mangle)]
pub extern "C" fn k_scan(p: *const u8, n: usize, i: usize, out: *mut f64) -> usize {
    let (v, end) = scan(bytes(p, n), i);
    unsafe { *out = v };
    end
}

// 4. Escaping writer into a Vec.
fn escape(out: &mut Vec<u8>, s: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(b'"');
    for &c in s {
        if c == b'"' || c == b'\\' {
            out.push(b'\\');
            out.push(c);
        } else if c < 0x20 {
            out.extend_from_slice(b"\\u00");
            out.push(HEX[(c >> 4) as usize]);
            out.push(HEX[(c & 15) as usize]);
        } else {
            out.push(c);
        }
    }
    out.push(b'"');
}

#[unsafe(no_mangle)]
pub extern "C" fn k_escape(out: *mut Vec<u8>, p: *const u8, n: usize) -> i32 {
    escape(unsafe { &mut *out }, bytes(p, n));
    0
}

// 5. Error propagation through a call chain, with an error struct and `?`.
pub struct Error {
    pub msg: &'static str,
    pub pos: usize,
}

macro_rules! calc_parser {
    ($m:ident, $export:ident, $err:ty, $mk:expr) => {
        mod $m {
            use super::*;
            struct Parser<'a> {
                s: &'a [u8],
                i: usize,
                depth: u32,
            }
            
            type R<T> = Result<T, $err>;
            
            impl Parser<'_> {
                fn fail<T>(&self, msg: &'static str) -> R<T> {
                    Err($mk(msg, self.i))
                }
                fn peek(&self) -> Option<u8> {
                    self.s.get(self.i).copied()
                }
                fn unary(&mut self) -> R<i32> {
                    if self.peek() == Some(b'-') {
                        self.i += 1;
                        return Ok(self.unary()?.wrapping_neg());
                    }
                    self.primary()
                }
                fn mul(&mut self) -> R<i32> {
                    let mut v = self.unary()?;
                    while let Some(op @ (b'*' | b'/')) = self.peek() {
                        self.i += 1;
                        let r = self.unary()?;
                        if op == b'/' && r == 0 {
                            return self.fail("division by zero");
                        }
                        v = if op == b'*' { v.wrapping_mul(r) } else { v.wrapping_div(r) };
                    }
                    Ok(v)
                }
                fn add(&mut self) -> R<i32> {
                    let mut v = self.mul()?;
                    while let Some(op @ (b'+' | b'-')) = self.peek() {
                        self.i += 1;
                        let r = self.mul()?;
                        v = if op == b'+' { v.wrapping_add(r) } else { v.wrapping_sub(r) };
                    }
                    Ok(v)
                }
                fn primary(&mut self) -> R<i32> {
                    match self.peek() {
                        None => self.fail("unexpected end"),
                        Some(b'(') => {
                            self.depth += 1;
                            if self.depth > 100 {
                                return self.fail("too deep");
                            }
                            self.i += 1;
                            let v = self.add()?;
                            if self.peek() != Some(b')') {
                                return self.fail("expected )");
                            }
                            self.i += 1;
                            self.depth -= 1;
                            Ok(v)
                        }
                        Some(c) if c.is_ascii_digit() => {
                            let mut v: i32 = 0;
                            while let Some(c @ b'0'..=b'9') = self.peek() {
                                v = v.wrapping_mul(10).wrapping_add((c - b'0') as i32);
                                self.i += 1;
                            }
                            Ok(v)
                        }
                        Some(_) => self.fail("expected a number"),
                    }
                }
            }
            
            #[unsafe(no_mangle)]
            pub extern "C" fn $export(p: *const u8, n: usize, out: *mut i32) -> i32 {
                let mut ps = Parser { s: bytes(p, n), i: 0, depth: 0 };
                match ps.add() {
                    Ok(v) => {
                        unsafe { *out = v };
                        0
                    }
                    Err(_) => -1,
                }
            }
            
            
        }
    };
}

// The error inline (12 bytes: returned through memory) and boxed (one pointer: returned
// in a register), as djs's compiler did before and after D1572.
calc_parser!(inline, k_calc, Error, |msg, pos| Error { msg, pos });
calc_parser!(boxed, k_calc_boxed, alloc::boxed::Box<Error>, |msg, pos| alloc::boxed::Box::new(Error { msg, pos }));

// 6. Gather-sum with checked indexing.
fn gather(vals: &[f64], idx: &[u32]) -> f64 {
    idx.iter().map(|&i| vals[i as usize]).sum()
}

#[unsafe(no_mangle)]
pub extern "C" fn k_gather(vals: *const f64, nvals: usize, idx: *const u32, n: usize) -> f64 {
    let vals = unsafe { core::slice::from_raw_parts(vals, nvals) };
    let idx = unsafe { core::slice::from_raw_parts(idx, n) };
    gather(vals, idx)
}
