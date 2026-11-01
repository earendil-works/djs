//! Exports that keep djs-core's entry points alive in the wasm build: the lexer (with
//! the parser's rescans), number conversion, regexp compile and exec, JSON parse and
//! stringify, and the heap with its GC. The host passes input in a buffer obtained from
//! `probe_alloc`. Not meant to be called; only its size matters.

use djs_core::heap::Heap;
use djs_core::json::{ParseOptions, parse_utf8, stringify_fast};
use djs_core::lexer::{Lexer, TokenKind};
use djs_core::num::{string_to_number, to_fixed, to_precision, to_string, to_string_radix};
use djs_core::regexp::{ExecLimits, ExecResult, Flags, Input, compile, exec};

fn input<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    // SAFETY: the host passes a buffer it got from `probe_alloc` with `len` bytes.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_alloc(len: usize) -> *mut u8 {
    Vec::<u8>::with_capacity(len).leak().as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_lex(ptr: *const u8, len: usize) -> u32 {
    let Ok(mut lx) = Lexer::new(input(ptr, len)) else { return 0 };
    let mut n = 0;
    loop {
        let Ok(t) = lx.next_token() else { return n };
        let t = match t.kind {
            TokenKind::Slash | TokenKind::SlashAssign => match lx.rescan_as_regex(t) {
                Ok(t) => t,
                Err(_) => return n,
            },
            TokenKind::RBrace => match lx.rescan_template_continuation(t) {
                Ok(t) => t,
                Err(_) => t,
            },
            TokenKind::Gt => lx.rescan_greater(t),
            _ => t,
        };
        if t.kind == TokenKind::Eof {
            return n;
        }
        n += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_num(x: f64, ptr: *const u8, len: usize) -> f64 {
    let a = to_string(x).as_str().len() + to_string_radix(x, 7).map(|s| s.as_str().len()).unwrap_or(0);
    let b = to_fixed(x, 3).map(|s| s.as_str().len()).unwrap_or(0) + to_precision(x, Some(5)).map(|s| s.as_str().len()).unwrap_or(0);
    string_to_number(input(ptr, len)) + (a + b) as f64
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_regexp(pat: *const u8, pat_len: usize, text: *const u8, text_len: usize) -> i32 {
    let pattern: Vec<u16> = input(pat, pat_len).iter().map(|&b| b as u16).collect();
    let Ok(prog) = compile(&pattern, Flags::parse(&[b'g' as u16, b'i' as u16]).unwrap()) else { return -1 };
    let mut fuel = u64::MAX;
    match exec(&prog, Input::Latin1(input(text, text_len)), 0, &mut fuel, ExecLimits::default()) {
        ExecResult::Match(c) => c.get(0).map(|(s, _)| s as i32).unwrap_or(-1),
        _ => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_json(ptr: *const u8, len: usize) -> u32 {
    let Ok(mut h) = Heap::new() else { return 0 };
    let Ok(v) = parse_utf8(&mut h, input(ptr, len), &ParseOptions::default()) else { return 0 };
    let Ok(handle) = h.new_handle(v) else { return 0 };
    if h.collect().is_err() {
        return 0;
    }
    let v = h.get_handle(handle);
    match stringify_fast(&mut h, v, &[]) {
        Ok(Some(s)) => h.str_len(s),
        _ => 0,
    }
}
