//! String leaf intrinsics on the heap representation (one-byte and two-byte strings,
//! ropes walked in chunks where that avoids flattening).

use alloc::vec::Vec;

use crate::heap::Heap;
use crate::interp::conv::Fault;
use crate::interp::{Intrinsic, realm};
use crate::value::{Obj, Str, Value};

/// IsLessThan for two strings, as -1, 0 or 1 (code unit order).
pub(crate) fn compare(heap: &Heap, a: Str, b: Str) -> f64 {
    match heap.str_units(a).cmp(heap.str_units(b)) {
        core::cmp::Ordering::Less => -1.0,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => 1.0,
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
        values.push(Value::string(heap.alloc_utf16(&units[i..i + n])?));
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
    let view = heap.str_view_flat(s)?;
    if let crate::heap::StrView::Latin1(b) = view {
        let part: Vec<u8> = b[start as usize..end as usize].to_vec();
        return Ok(heap.alloc_latin1(&part)?);
    }
    let part: Vec<u16> = (start..end).map(|i| view.get(i as usize)).collect();
    Ok(heap.alloc_utf16(&part)?)
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
    let needle = units(heap, search)?;
    let hay = heap.str_view_flat(s)?;
    let (n, m) = (hay.len(), needle.len());
    if m > n {
        return Ok(-1.0);
    }
    let matches = |i: usize| (0..m).all(|j| hay.get(i + j) == needle[j]);
    let from = from as usize;
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
    let mut out = Vec::with_capacity(u.len());
    let mut i = 0;
    while i < u.len() {
        let c = u[i] as u32;
        if (0xD800..0xDC00).contains(&c)
            && u.get(i + 1).is_some_and(|l| (0xDC00..0xE000).contains(l))
        {
            out.push((
                i,
                0x10000 + ((c - 0xD800) << 10) + (u[i + 1] as u32 - 0xDC00),
            ));
            i += 2;
        } else {
            out.push((i, c));
            i += 1;
        }
    }
    out
}

fn push_code_point(out: &mut Vec<u16>, c: u32) {
    if c >= 0x10000 {
        let c = c - 0x10000;
        out.push(0xD800 + (c >> 10) as u16);
        out.push(0xDC00 + (c & 0x3FF) as u16);
    } else {
        out.push(c as u16);
    }
}

/// toUpperCase / toLowerCase: the full Unicode mappings (SpecialCasing without
/// language-specific rules) and Final_Sigma for lowercase.
pub(crate) fn to_case(heap: &mut Heap, s: Str, upper: bool) -> Result<Str, Fault> {
    use crate::unicode::case;
    let u = units(heap, s)?;
    if u.iter().all(|&c| c < 0x80) {
        let mapped: Vec<u8> = u
            .iter()
            .map(|&c| {
                let b = c as u8;
                if upper {
                    b.to_ascii_uppercase()
                } else {
                    b.to_ascii_lowercase()
                }
            })
            .collect();
        if mapped.iter().zip(&u).all(|(&m, &c)| m as u16 == c) {
            return Ok(s);
        }
        return Ok(heap.alloc_latin1(&mapped)?);
    }
    let cps = code_points_of(&u);
    let mut out = Vec::with_capacity(u.len());
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

/// `s` repeated `n` times, by doubling (ropes keep it O(log n) concatenations).
pub(crate) fn repeat(heap: &mut Heap, s: Str, n: u32) -> Result<Str, Fault> {
    let len = heap.str_len(s) as u64;
    if len * n as u64 > crate::heap::string::MAX_LEN as u64 {
        return Err(crate::heap::HeapError::StringTooLong.into());
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
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let v = heap
            .get_element_fast(list, i)
            .and_then(|v| v.as_number())
            .unwrap_or(0.0);
        if points {
            push_code_point(&mut out, v as u32);
        } else {
            out.push(crate::interp::conv::to_int32(v) as u16);
        }
    }
    Ok(heap.alloc_utf16(&out)?)
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
        let out = norm::normalize(&u, form);
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
        let nfc = |u: Vec<u16>| {
            if norm::is_normalized_quick(&u, Form::Nfc) {
                u
            } else {
                norm::normalize(&u, Form::Nfc)
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
