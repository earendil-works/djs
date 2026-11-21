//! Byte scanning a word at a time (SWAR over `u64`): the search, JSON and split kernels.
//! Plain safe Rust, the same natively and in wasm, which has no SIMD in the shipped module
//! (D016).
//!
//! The lane tests below borrow across lanes, so they can flag a lane *above* a real match,
//! never one below it: the lowest flag of a mask (and of an `|` of masks) is exact, the
//! higher ones are only candidates. Every user takes the lowest flag or rechecks.

/// 0x01 in every byte lane.
pub(crate) const LO: u64 = 0x0101_0101_0101_0101;
/// The high bit of every byte lane.
pub(crate) const HI: u64 = 0x8080_8080_8080_8080;
/// 0x0001 in every 16-bit lane.
pub(crate) const LO16: u64 = 0x0001_0001_0001_0001;
/// The high bit of every 16-bit lane.
pub(crate) const HI16: u64 = 0x8000_8000_8000_8000;

/// The little-endian word at `b[i..i + 8]`, if there are 8 bytes.
#[inline(always)]
pub(crate) fn word_at(b: &[u8], i: usize) -> Option<u64> {
    let w = b.get(i..i.checked_add(8)?)?;
    Some(u64::from_le_bytes(w.try_into().unwrap()))
}

/// The high bit of each zero byte (exact for the lowest one).
#[inline(always)]
pub(crate) fn zero8(x: u64) -> u64 {
    x.wrapping_sub(LO) & !x & HI
}

/// The high bit of each byte equal to `b`.
#[inline(always)]
pub(crate) fn eq8(w: u64, b: u8) -> u64 {
    zero8(w ^ (LO * b as u64))
}

/// The high bit of each byte below `n` (`n` ≤ 0x80).
#[inline(always)]
pub(crate) fn lt8(w: u64, n: u8) -> u64 {
    w.wrapping_sub(LO * n as u64) & !w & HI
}

/// The high bit of each zero 16-bit lane (exact for the lowest one).
#[inline(always)]
pub(crate) fn zero16(x: u64) -> u64 {
    x.wrapping_sub(LO16) & !x & HI16
}

/// The high bit of each 16-bit lane equal to `u`.
#[inline(always)]
pub(crate) fn eq16(w: u64, u: u16) -> u64 {
    zero16(w ^ (LO16 * u as u64))
}

/// The high bit of each 16-bit lane below `n` (`n` ≤ 0x8000).
#[inline(always)]
pub(crate) fn lt16(w: u64, n: u16) -> u64 {
    w.wrapping_sub(LO16 * n as u64) & !w & HI16
}

/// The byte index of the lowest flag of a non-zero byte-lane mask.
#[inline(always)]
pub(crate) fn first8(m: u64) -> usize {
    (m.trailing_zeros() / 8) as usize
}

/// The lane index of the lowest flag of a non-zero 16-bit-lane mask.
#[inline(always)]
pub(crate) fn first16(m: u64) -> usize {
    (m.trailing_zeros() / 16) as usize
}

/// The index of the first `b` in `h`, eight bytes at a time (0110: a byte loop was most
/// of `includes`).
#[inline]
pub(crate) fn find_byte(h: &[u8], b: u8) -> Option<usize> {
    find_with(h, |w| eq8(w, b), |c| c == b)
}

/// The index of the first byte of `h` that is `a` or `b`: a regular expression's first
/// characters (`f`, `T|F`, `x` with `i`), line terminators.
pub(crate) fn find_either(h: &[u8], a: u8, b: u8) -> Option<usize> {
    find_with(h, |w| eq8(w, a) | eq8(w, b), |x| x == a || x == b)
}

/// The first byte where `hit` flags a word (exact for its lowest flag) or, in a haystack
/// shorter than a word, where `is` holds.
#[inline(always)]
fn find_with(h: &[u8], hit: impl Fn(u64) -> u64, is: impl Fn(u8) -> bool) -> Option<usize> {
    let Some(last) = h.len().checked_sub(8) else {
        return h.iter().position(|&c| is(c));
    };
    let mut i = 0;
    loop {
        // The last word overlaps the one before: its bytes done already are masked out
        // (they hold no match, so nothing borrows into the rest).
        let s = i.min(last);
        let z = hit(word_at(h, s)?) & (!0u64 << (8 * (i - s)));
        if z != 0 {
            return Some(s + first8(z));
        }
        i += 8;
        if i >= h.len() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every lane value against every position: the lowest flag is exact.
    #[test]
    fn lowest_flag_is_exact() {
        let fill = [0x00u8, 0x01, 0x1F, 0x20, 0x22, 0x5B, 0x5C, 0x7F, 0x80, 0xFF];
        for &bg in &fill {
            for &x in &fill {
                for pos in 0..8 {
                    let mut b = [bg; 8];
                    b[pos] = x;
                    let w = u64::from_le_bytes(b);
                    for (m, f) in [
                        (eq8(w, 0x5C), &(|c: u8| c == 0x5C) as &dyn Fn(u8) -> bool),
                        (eq8(w, 0), &|c: u8| c == 0),
                        (lt8(w, 0x20), &|c: u8| c < 0x20),
                    ] {
                        let want = b.iter().position(|&c| f(c));
                        assert_eq!((m != 0).then(|| first8(m)), want, "{b:?}");
                    }
                }
            }
        }
    }

    /// The same for 16-bit lanes.
    #[test]
    fn lowest_flag_is_exact_16() {
        let units = [
            0x0000u16, 0x001F, 0x0020, 0x0022, 0x005C, 0x00FF, 0x0100, 0x5C5C, 0xFFFF,
        ];
        for &bg in &units {
            for &x in &units {
                for pos in 0..4 {
                    let mut u = [bg; 4];
                    u[pos] = x;
                    let w = u[0] as u64
                        | (u[1] as u64) << 16
                        | (u[2] as u64) << 32
                        | (u[3] as u64) << 48;
                    let m = eq16(w, 0x5C);
                    assert_eq!(
                        (m != 0).then(|| first16(m)),
                        u.iter().position(|&c| c == 0x5C)
                    );
                    let m = lt16(w, 0x20);
                    assert_eq!(
                        (m != 0).then(|| first16(m)),
                        u.iter().position(|&c| c < 0x20)
                    );
                }
            }
        }
    }

    #[test]
    fn find_either_every_offset() {
        let set = [b'\r', 0xE9];
        for n in 0..40 {
            for at in 0..n {
                for (x, other) in [(b'\r', b'\n'), (0xE9, 0xEA), (b'\n', 0x0B), (b'\r', 0xE9)] {
                    let mut h = alloc::vec![0x0Bu8; n];
                    h[at] = x;
                    if at + 1 < n {
                        h[at + 1] = other;
                    }
                    let want = h.iter().position(|c| set.contains(c));
                    assert_eq!(find_either(&h, set[0], set[1]), want, "{h:?}");
                }
            }
            assert_eq!(find_either(&alloc::vec![0u8; n], set[0], set[1]), None);
        }
    }

    #[test]
    fn find_byte_every_offset() {
        for n in 0..40 {
            let mut h = alloc::vec![b'a'; n];
            assert_eq!(find_byte(&h, b'x'), None);
            for at in 0..n {
                h[at] = b'x';
                assert_eq!(find_byte(&h, b'x'), Some(at));
                if at + 1 < n {
                    h[at + 1] = b'x';
                    assert_eq!(find_byte(&h, b'x'), Some(at));
                    h[at + 1] = b'a';
                }
                h[at] = 0xF8;
                assert_eq!(find_byte(&h, b'x'), None);
                h[at] = b'a';
            }
        }
    }
}
