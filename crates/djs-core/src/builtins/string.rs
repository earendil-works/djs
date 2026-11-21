//! String leaf intrinsics on the heap representation (one-byte and two-byte strings,
//! ropes walked in chunks where that avoids flattening).
use crate::PushOut;

use alloc::vec::Vec;

use crate::heap::{Heap, StrView};
use crate::interp::conv::Fault;
use crate::interp::{Intrinsic, realm};
use crate::value::{Obj, Str, Value};

/// IsLessThan for two strings, as -1, 0 or 1 (code unit order).
pub(crate) fn compare(heap: &Heap, a: Str, b: Str) -> f64 {
    let order = match (heap.str_view(a), heap.str_view(b)) {
        (Some(x), Some(y)) => view_cmp(x, y),
        _ => heap.str_units(a).cmp(heap.str_units(b)),
    };
    match order {
        core::cmp::Ordering::Less => -1.0,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => 1.0,
    }
}

/// Code unit order of two flat strings: bytes for two one-byte ones (most), units
/// otherwise.
pub(crate) fn view_cmp(a: StrView, b: StrView) -> core::cmp::Ordering {
    match (a, b) {
        (StrView::Latin1(x), StrView::Latin1(y)) => x.cmp(y),
        _ => a.units().cmp(b.units()),
    }
}

/// The code points of `s` as an array of one- or two-unit strings (lone surrogates on
/// their own), as the String iterator yields them.
pub(crate) fn code_points(heap: &mut Heap, s: Str) -> Result<Obj, Fault> {
    let units: Vec<u16> = heap.str_units(s).collect();
    let mut values = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let n = if (0xD800..0xDC00).contains(&units[i])
            && units
                .get(i + 1)
                .is_some_and(|u| (0xDC00..0xE000).contains(u))
        {
            2
        } else {
            1
        };
        values.push_out(Value::string(heap.alloc_utf16(&units[i..i + n])?));
        i += n;
    }
    let proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    Ok(heap.new_array_from_values(Some(proto), &values)?)
}

/// The code units of `s` (flattening a rope once).
fn units(heap: &mut Heap, s: Str) -> Result<Vec<u16>, Fault> {
    Ok(heap.str_view_flat(s)?.units().collect())
}

/// `s[start .. end)` in code units (the caller clamps the indexes).
pub(crate) fn substring(heap: &mut Heap, s: Str, start: u32, end: u32) -> Result<Str, Fault> {
    let len = heap.str_len(s);
    if start == 0 && end >= len {
        return Ok(s);
    }
    if start >= end {
        return Ok(heap.alloc_latin1(&[])?);
    }
    if let Some(slice) = heap.substring_slice(s, start, end)? {
        return Ok(slice);
    }
    if let Some(r) = heap.latin1_copy(s, start, end)? {
        return Ok(r);
    }
    join(heap, &mut [(s, start, end)])
}

/// The concatenation of the pieces `s[start .. end)` as one flat string, written once
/// (no ropes and no intermediate strings). One-byte pieces are copied within the arena;
/// pieces are read like `str_view_flat` reads them (an append rope in place).
pub(crate) fn join(heap: &mut Heap, pieces: &mut [(Str, u32, u32)]) -> Result<Str, Fault> {
    let mut total: u64 = 0;
    let mut two_byte = false;
    for p in pieces.iter_mut() {
        total += u64::from(p.2 - p.1);
        two_byte |= heap.str_is_two_byte(p.0);
    }
    if total > u64::from(crate::heap::string::MAX_LEN) {
        return Err(crate::heap::HeapError::StringTooLong.into());
    }
    let total = total as usize;
    if !two_byte {
        for &(s, _, _) in pieces.iter() {
            // (Flat pieces need nothing; ropes are flattened, append ropes read in place.)
            if heap.str_view(s).is_none() {
                heap.str_view_flat(s)?;
            }
        }
        return Ok(heap.alloc_latin1_pieces(pieces, total as u32)?);
    }
    let mut units: Vec<u16> = crate::buf::vec_with_capacity(total);
    for &(s, a, b) in pieces.iter() {
        let v = heap.str_view_flat(s)?;
        units.extend((a as usize..b as usize).map(|i| v.get(i)));
    }
    Ok(heap.alloc_utf16(&units)?)
}

pub(crate) use crate::swar::find_byte;

/// The first index ≥ `from` of the non-empty `needle` in `h` (one-byte strings). Eight
/// candidate positions at a time, testing the needle's first and last byte together
/// (0116): with the first byte alone, a common letter flagged most positions of a line.
/// The last candidates take one overlapping word, not a byte loop (0260: on a short line
/// the loop was a third of the search).
pub(crate) fn find_latin1(h: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    use crate::swar::{eq8, first8};
    let m = needle.len();
    let (&first, rest) = needle.split_first()?;
    let hay = h.get(from..)?;
    if rest.is_empty() {
        return find_byte(hay, first).map(|k| from + k);
    }
    let last = needle[m - 1];
    // Candidates 0..n, eight a time; the flags (maybe more than the matches, never fewer)
    // are checked in full.
    let n = (hay.len() + 1).checked_sub(m)?;
    let word = |i: usize| u64::from_le_bytes(hay[i..i + 8].try_into().unwrap());
    let flags = |i: usize| eq8(word(i), first) & eq8(word(i + m - 1), last);
    let mut i = 0;
    while i < n {
        let (mut f, base) = if i + 8 <= n {
            (flags(i), i)
        } else if n >= 8 {
            // The last 1 to 7 candidates: the block of the last eight, without those done.
            (flags(n - 8) & (!0u64 << (8 * (i + 8 - n))), n - 8)
        } else {
            // A haystack shorter than the needle plus seven.
            let mut f = 0;
            for k in 0..n {
                if hay[k] == first && hay[k + m - 1] == last {
                    f |= 0x80 << (8 * k);
                }
            }
            (f, 0)
        };
        while f != 0 {
            let at = base + first8(f);
            if hay[at..at + m] == *needle {
                return Some(from + at);
            }
            f &= f - 1;
        }
        i += 8;
    }
    None
}

/// The first index ≥ `from` (or, with `last`, the last index ≤ `from`) where `search`
/// occurs in `s`, or -1.
pub(crate) fn index_of(
    heap: &mut Heap,
    s: Str,
    search: Str,
    from: u32,
    last: bool,
) -> Result<f64, Fault> {
    // Both flat, then both read in place (0110: no copy of the needle).
    heap.str_view_flat(s)?;
    let search = heap.contiguous(search)?;
    let from = from as usize;
    if !last
        && let (Some(StrView::Latin1(h)), Some(StrView::Latin1(t))) =
            (heap.str_view(s), heap.str_view(search))
    {
        if t.is_empty() {
            return Ok(if from > h.len() { -1.0 } else { from as f64 });
        }
        return Ok(find_latin1(h, t, from).map_or(-1.0, |i| i as f64));
    }
    // The needle on the stack when short (no Rust heap allocation, 0081 F6).
    let m = heap.str_len(search) as usize;
    let mut stack = [0u16; 64];
    let vec: Vec<u16>;
    let needle: &[u16] = if m <= stack.len() {
        let v = heap.str_view(search).expect("flattened");
        for (i, u) in v.units().enumerate() {
            stack[i] = u;
        }
        &stack[..m]
    } else {
        vec = units(heap, search)?;
        &vec
    };
    let hay = heap.str_view_flat(s)?;
    let n = hay.len();
    if m > n {
        return Ok(-1.0);
    }
    let matches = |i: usize| (0..m).all(|j| hay.get(i + j) == needle[j]);
    if last {
        let mut i = from.min(n - m);
        loop {
            if matches(i) {
                return Ok(i as f64);
            }
            if i == 0 {
                return Ok(-1.0);
            }
            i -= 1;
        }
    }
    Ok((from..=n - m)
        .find(|&i| matches(i))
        .map_or(-1.0, |i| i as f64))
}

/// The code point at `i` (a surrogate pair combined), as a Number.
pub(crate) fn code_point_at(heap: &Heap, s: Str, i: u32) -> Option<f64> {
    let hi = heap.char_code_at(s, i)?;
    if (0xD800..0xDC00).contains(&hi)
        && let Some(lo) = heap.char_code_at(s, i + 1)
        && (0xDC00..0xE000).contains(&lo)
    {
        return Some((0x10000 + ((hi as u32 - 0xD800) << 10) + (lo as u32 - 0xDC00)) as f64);
    }
    Some(hi as f64)
}

/// Code points of `u` (lone surrogates as themselves), with their unit positions.
fn code_points_of(u: &[u16]) -> Vec<(usize, u32)> {
    let mut out = crate::buf::vec_with_capacity(u.len());
    let mut i = 0;
    while i < u.len() {
        let c = u[i] as u32;
        if (0xD800..0xDC00).contains(&c)
            && u.get(i + 1).is_some_and(|l| (0xDC00..0xE000).contains(l))
        {
            out.push_out((
                i,
                0x10000 + ((c - 0xD800) << 10) + (u[i + 1] as u32 - 0xDC00),
            ));
            i += 2;
        } else {
            out.push_out((i, c));
            i += 1;
        }
    }
    out
}

fn push_code_point(out: &mut Vec<u16>, c: u32) {
    if c >= 0x10000 {
        let c = c - 0x10000;
        out.push_out(0xD800 + (c >> 10) as u16);
        out.push_out(0xDC00 + (c & 0x3FF) as u16);
    } else {
        out.push_out(c as u16);
    }
}

/// toUpperCase / toLowerCase: the full Unicode mappings (SpecialCasing without
/// language-specific rules) and Final_Sigma for lowercase.
pub(crate) fn to_case(heap: &mut Heap, s: Str, upper: bool) -> Result<Str, Fault> {
    use crate::unicode::case;
    // One-byte strings in one pass into the arena (lower case always stays one-byte).
    if let Some(r) = heap.latin1_case(s, upper)? {
        return Ok(r);
    }
    let u = units(heap, s)?;
    let cps = code_points_of(&u);
    let mut out = crate::buf::vec_with_capacity(u.len());
    for (k, &(_, c)) in cps.iter().enumerate() {
        if !upper && c == 0x03A3 {
            // Final_Sigma: preceded by a cased letter and not followed by one (case
            // ignorables skipped on both sides).
            let before = cps[..k]
                .iter()
                .rev()
                .map(|&(_, c)| c)
                .find(|&c| !case::is_case_ignorable(c))
                .is_some_and(case::is_cased);
            let after = cps[k + 1..]
                .iter()
                .map(|&(_, c)| c)
                .find(|&c| !case::is_case_ignorable(c))
                .is_some_and(case::is_cased);
            push_code_point(&mut out, if before && !after { 0x03C2 } else { 0x03C3 });
            continue;
        }
        let m = if upper {
            case::to_upper_full(c)
        } else {
            case::to_lower_full(c)
        };
        for &d in m.as_slice() {
            push_code_point(&mut out, d);
        }
    }
    Ok(heap.alloc_utf16(&out)?)
}

fn is_trim_space(c: u16) -> bool {
    let c = c as u32;
    crate::unicode::is_js_whitespace(c) || crate::unicode::is_js_line_terminator(c)
}

/// trim (where 0), trimStart (1), trimEnd (2).
pub(crate) fn trim(heap: &mut Heap, s: Str, where_: i32) -> Result<Str, Fault> {
    // One-byte strings: the bytes directly (JS white space and line terminators below
    // 0x100 are 0x09-0x0D, 0x20 and 0xA0).
    if let Some(StrView::Latin1(b)) = heap.str_view(s) {
        let ws = |c: &u8| matches!(*c, 9..=13 | 0x20 | 0xA0);
        let start = if where_ != 2 {
            b.iter().position(|c| !ws(c)).unwrap_or(b.len())
        } else {
            0
        };
        let end = if where_ != 1 {
            b.iter()
                .rposition(|c| !ws(c))
                .map_or(start, |i| i + 1)
                .max(start)
        } else {
            b.len()
        };
        return substring(heap, s, start as u32, end as u32);
    }
    let view = heap.str_view_flat(s)?;
    let n = view.len();
    let mut start = 0;
    let mut end = n;
    if where_ != 2 {
        while start < n && is_trim_space(view.get(start)) {
            start += 1;
        }
    }
    if where_ != 1 {
        while end > start && is_trim_space(view.get(end - 1)) {
            end -= 1;
        }
    }
    substring(heap, s, start as u32, end as u32)
}

/// IsStringWellFormedUnicode, or (with `fix`) toWellFormed's result.
pub(crate) fn well_formed(heap: &mut Heap, s: Str, fix: bool) -> Result<Value, Fault> {
    let u = units(heap, s)?;
    let mut out = Vec::new();
    let mut ok = true;
    for (_, c) in code_points_of(&u) {
        let lone = (0xD800..0xE000).contains(&c);
        ok &= !lone;
        if fix {
            push_code_point(&mut out, if lone { 0xFFFD } else { c });
        }
    }
    if !fix {
        return Ok(Value::bool(ok));
    }
    Ok(Value::string(if ok { s } else { heap.alloc_utf16(&out)? }))
}

/// `s` padded with `fill` units of `filler` cycled, or of spaces when `filler` is `None`
/// (StringPad's last steps). A one-byte string padded with one one-byte unit (a space or
/// a zero, nearly always) is written once into the result cell; anything else repeats,
/// cuts and joins.
pub(crate) fn pad(
    heap: &mut Heap,
    s: Str,
    filler: Option<Str>,
    fill: u32,
    at_start: bool,
) -> Result<Str, Fault> {
    let s = heap.contiguous(s)?;
    let n = heap.str_len(s);
    let unit = match filler {
        None => Some(b' '),
        Some(f) if heap.str_len(f) == 1 => {
            heap.char_code_at(f, 0).and_then(|c| u8::try_from(c).ok())
        }
        Some(_) => None,
    };
    if fill <= 65536
        && !heap.str_is_two_byte(s)
        && let Some(c) = unit
    {
        let out = heap.alloc_latin1_pieces(&[(s, 0, n)], n + fill)?;
        let d = Heap::str_data_offset(out);
        let a = heap.arena_mut();
        let run = if at_start {
            a.copy_within(d, n, d + fill);
            d
        } else {
            d + n
        };
        a.slice_mut(run, fill).fill(c);
        return Ok(out);
    }
    let filler = match filler {
        Some(f) => f,
        None => heap.alloc_latin1(b" ")?,
    };
    let fl = heap.str_len(filler);
    let rep = repeat(heap, filler, fill.div_ceil(fl))?;
    let run = substring(heap, rep, 0, fill)?;
    Ok(if at_start {
        heap.concat(run, s)?
    } else {
        heap.concat(s, run)?
    })
}

/// `s` repeated `n` times, by doubling (ropes keep it O(log n) concatenations).
pub(crate) fn repeat(heap: &mut Heap, s: Str, n: u32) -> Result<Str, Fault> {
    let len = heap.str_len(s) as u64;
    if len * n as u64 > crate::heap::string::MAX_LEN as u64 {
        return Err(crate::heap::HeapError::StringTooLong.into());
    }
    // Short results are written once (no ropes, no intermediate strings).
    if len * n as u64 <= 4096 && n <= 4096 {
        let l = len as u32;
        let mut pieces: Vec<(Str, u32, u32)> = (0..n).map(|_| (s, 0, l)).collect();
        return join(heap, &mut pieces);
    }
    let mut result = heap.alloc_latin1(&[])?;
    let mut part = s;
    let mut k = n;
    while k > 0 {
        if k & 1 == 1 {
            result = heap.concat(result, part)?;
        }
        k >>= 1;
        if k > 0 {
            part = heap.concat(part, part)?;
        }
    }
    Ok(result)
}

/// A string of the code units (`points` false: ToUint16 of each Number) or code points
/// (already validated) in the dense array `list`.
pub(crate) fn from_codes(heap: &mut Heap, list: Obj, points: bool) -> Result<Str, Fault> {
    let n = heap.array_length(list);
    let code = |v: Value| match v.as_int() {
        Some(i) => i as u32,
        None => {
            let x = v.as_number().unwrap_or(0.0);
            if points {
                x as u32
            } else {
                crate::interp::conv::to_int32(x) as u32
            }
        }
    };
    let emit = |out: &mut Units, v: Value| {
        if points {
            let c = code(v);
            if c >= 0x10000 {
                let c = c - 0x10000;
                out.push(0xD800 + (c >> 10) as u16);
                out.push(0xDC00 + (c & 0x3FF) as u16);
                return;
            }
            out.push(c as u16);
        } else {
            out.push(code(v) as u16);
        }
    };
    // A few units (`String.fromCharCode(c)` in a loop) go through a stack buffer: no Rust
    // allocation per call (D1871).
    let mut out = if n <= 32 {
        Units::Small([0; 64], 0)
    } else {
        Units::Big(crate::buf::vec_with_capacity(n as usize))
    };
    match heap.dense_array_values(list) {
        // One pass over the dense store (D1784).
        Some(vs) => vs.for_each(|v| emit(&mut out, v)),
        None => {
            for i in 0..n {
                let v = heap.get_element_fast(list, i).unwrap_or(Value::int(0));
                emit(&mut out, v);
            }
        }
    }
    Ok(heap.alloc_utf16(out.as_slice())?)
}

/// Code units collected on the stack while there are few, else in a Vec.
enum Units {
    Small([u16; 64], usize),
    Big(Vec<u16>),
}

impl Units {
    fn push(&mut self, u: u16) {
        match self {
            Units::Small(buf, n) if *n < buf.len() => {
                buf[*n] = u;
                *n += 1;
            }
            Units::Small(buf, n) => {
                let mut v = buf[..*n].to_vec();
                v.push_out(u);
                *self = Units::Big(v);
            }
            Units::Big(v) => v.push_out(u),
        }
    }

    fn as_slice(&self) -> &[u16] {
        match self {
            Units::Small(buf, n) => &buf[..*n],
            Units::Big(v) => v,
        }
    }
}

/// `normalize(form)` (form 0 NFC, 1 NFD, 2 NFKC, 3 NFKD). With the
/// `unicode-normalization` feature every string normalizes; strings that pass the quick
/// check come back unchanged. Without the tables only strings whose code units are all
/// below U+0300 (for NFD/NFKD/NFKC below U+00A0) are known to be normalized; `None` for
/// anything else (D1760).
pub(crate) fn normalize(heap: &mut Heap, s: Str, form: u8) -> Result<Option<Str>, Fault> {
    // Nothing below U+0300 (NFC) or U+00A0 (other forms) changes: checked in place,
    // without copying the units out.
    let limit = if form == 0 { 0x300 } else { 0xA0 };
    if heap.str_view_flat(s)?.units().all(|u| u < limit) {
        return Ok(Some(s));
    }
    #[cfg(not(feature = "unicode-normalization"))]
    return Ok(None);
    #[cfg(feature = "unicode-normalization")]
    {
        use crate::unicode::normalize::{self as norm, Form};
        let form = match form {
            0 => Form::Nfc,
            1 => Form::Nfd,
            2 => Form::Nfkc,
            _ => Form::Nfkd,
        };
        let u = units(heap, s)?;
        if norm::is_normalized_quick(&u, form) {
            return Ok(Some(s));
        }
        let out = norm::normalize(&u, form, &mut heap.compositions);
        if out == u {
            return Ok(Some(s));
        }
        Ok(Some(crate::builtins::regexp::string_from_units(
            heap, &out,
        )?))
    }
}

/// localeCompare without Intl, as QuickJS computes it: both strings NFC-normalized (with
/// the `unicode-normalization` feature), then the difference of the first differing code
/// points, else -1/0/1 by length.
pub(crate) fn locale_compare(heap: &mut Heap, a: Str, b: Str) -> Result<f64, Fault> {
    let (ua, ub) = (units(heap, a)?, units(heap, b)?);
    #[cfg(feature = "unicode-normalization")]
    let (ua, ub) = {
        use crate::unicode::normalize::{self as norm, Form};
        let mut nfc = |u: Vec<u16>| {
            if norm::is_normalized_quick(&u, Form::Nfc) {
                u
            } else {
                norm::normalize(&u, Form::Nfc, &mut heap.compositions)
            }
        };
        (nfc(ua), nfc(ub))
    };
    let (ca, cb) = (code_points_of(&ua), code_points_of(&ub));
    for (x, y) in ca.iter().zip(&cb) {
        if x.1 != y.1 {
            return Ok(x.1 as f64 - y.1 as f64);
        }
    }
    Ok(match ca.len().cmp(&cb.len()) {
        core::cmp::Ordering::Less => -1.0,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::{find_byte, find_latin1};

    /// Against the naive searches, on random strings over small alphabets (so that
    /// matches, near misses and bytes with the high bit set all occur) at every
    /// alignment and length around the 8-byte chunks.
    #[test]
    fn word_at_a_time_search_matches_the_naive_one() {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n) as u8
        };
        let alphabets: [&[u8]; 3] = [b"ab", b"abc\x80\xff\x00", b"\x01\x02\x7f\x80\x81"];
        for round in 0..8000 {
            let al = alphabets[round % 3];
            let n = next(90) as usize;
            let h: alloc::vec::Vec<u8> =
                (0..n).map(|_| al[next(al.len() as u64) as usize]).collect();
            let b = al[next(al.len() as u64) as usize];
            assert_eq!(
                find_byte(&h, b),
                h.iter().position(|&c| c == b),
                "{h:?} {b}"
            );
            let m = 1 + next(6) as usize;
            let t: alloc::vec::Vec<u8> =
                (0..m).map(|_| al[next(al.len() as u64) as usize]).collect();
            let from = next(n as u64 + 2) as usize;
            let naive = (from..(n + 1).saturating_sub(m)).find(|&i| h[i..i + m] == t[..]);
            assert_eq!(find_latin1(&h, &t, from), naive, "{h:?} {t:?} {from}");
        }
    }

    /// Needles of 1 to 9 bytes at every offset of haystacks of 0 to 24 bytes, from every
    /// start, with near misses (first and last byte right, a middle one wrong) and
    /// non-ASCII bytes around them (0260: the word loop, the overlapping last word and the
    /// byte loop of short haystacks).
    #[test]
    fn find_latin1_every_offset() {
        let naive = |h: &[u8], t: &[u8], from: usize| {
            (from..(h.len() + 1).saturating_sub(t.len())).find(|&i| h[i..i + t.len()] == *t)
        };
        for m in 1..=9usize {
            let t: alloc::vec::Vec<u8> = (0..m).map(|i| [0xE9, b'x', 0xFF][i % 3]).collect();
            let mut miss = t.clone();
            if m > 2 {
                miss[1] = b'y';
            }
            for n in 0..=24usize {
                for at in 0..=n.saturating_sub(m) {
                    for (pat, other) in [(&t, None), (&miss, Some(n / 2))] {
                        let mut h = alloc::vec![0x80u8; n];
                        if at + m <= n {
                            h[at..at + m].copy_from_slice(pat);
                        }
                        // A second copy of the needle, or nothing.
                        if let Some(o) = other
                            && o + m <= n
                        {
                            h[o..o + m].copy_from_slice(&t);
                        }
                        for from in 0..=n + 1 {
                            assert_eq!(
                                find_latin1(&h, &t, from),
                                naive(&h, &t, from),
                                "{h:?} {t:?} {from}"
                            );
                        }
                    }
                }
            }
        }
    }
}
