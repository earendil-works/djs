//! Code point sets as sorted ranges, and the spec's Canonicalize for case-insensitive
//! matching.

use alloc::vec::Vec;

use crate::unicode::case;

pub(crate) const MAX_CP: u32 = 0x10FFFF;

/// A set of code points: sorted, disjoint, non-adjacent inclusive ranges.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CharSet {
    ranges: Vec<(u32, u32)>,
}

impl CharSet {
    pub(crate) fn new() -> CharSet {
        CharSet { ranges: Vec::new() }
    }

    pub(crate) fn from_ranges(ranges: &[(u32, u32)]) -> CharSet {
        let mut s = CharSet::new();
        for &(lo, hi) in ranges {
            s.add_range(lo, hi);
        }
        s
    }

    pub(crate) fn ranges(&self) -> &[(u32, u32)] {
        &self.ranges
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub(crate) fn contains(&self, c: u32) -> bool {
        let i = self.ranges.partition_point(|&(lo, _)| lo <= c);
        i > 0 && c <= self.ranges[i - 1].1
    }

    pub(crate) fn add(&mut self, c: u32) {
        self.add_range(c, c);
    }

    pub(crate) fn add_range(&mut self, lo: u32, hi: u32) {
        debug_assert!(lo <= hi);
        // Fast path: appending in order, as the parser and the canonicalizer mostly do.
        if let Some(last) = self.ranges.last_mut() {
            if lo > last.1.saturating_add(1) {
                self.ranges.push((lo, hi));
                return;
            }
            if lo >= last.0 {
                last.1 = last.1.max(hi);
                return;
            }
        } else {
            self.ranges.push((lo, hi));
            return;
        }
        let other = CharSet {
            ranges: alloc::vec![(lo, hi)],
        };
        *self = self.union(&other);
    }

    pub(crate) fn union(&self, other: &CharSet) -> CharSet {
        let mut all: Vec<(u32, u32)> = Vec::with_capacity(self.ranges.len() + other.ranges.len());
        let (mut i, mut j) = (0, 0);
        while i < self.ranges.len() || j < other.ranges.len() {
            let take_a = j >= other.ranges.len()
                || (i < self.ranges.len() && self.ranges[i].0 <= other.ranges[j].0);
            let r = if take_a {
                i += 1;
                self.ranges[i - 1]
            } else {
                j += 1;
                other.ranges[j - 1]
            };
            match all.last_mut() {
                Some(last) if r.0 <= last.1.saturating_add(1) => last.1 = last.1.max(r.1),
                _ => all.push(r),
            }
        }
        CharSet { ranges: all }
    }

    pub(crate) fn intersect(&self, other: &CharSet) -> CharSet {
        let mut out = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.ranges.len() && j < other.ranges.len() {
            let (a0, a1) = self.ranges[i];
            let (b0, b1) = other.ranges[j];
            let lo = a0.max(b0);
            let hi = a1.min(b1);
            if lo <= hi {
                out.push((lo, hi));
            }
            if a1 < b1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        CharSet { ranges: out }
    }

    /// The complement within 0..=0x10FFFF.
    pub(crate) fn invert(&self) -> CharSet {
        let mut out = Vec::with_capacity(self.ranges.len() + 1);
        let mut next = 0u32;
        for &(lo, hi) in &self.ranges {
            if lo > next {
                out.push((next, lo - 1));
            }
            next = hi + 1;
        }
        if next <= MAX_CP {
            out.push((next, MAX_CP));
        }
        CharSet { ranges: out }
    }

    pub(crate) fn subtract(&self, other: &CharSet) -> CharSet {
        self.intersect(&other.invert())
    }

    /// `{ canonicalize(c) | c in self }`.
    pub(crate) fn canonicalize(&self, unicode: bool) -> CharSet {
        // Only code points whose canonical form differs from themselves move; walk those
        // (about 1500) instead of the set, which can be huge.
        let mut moved = CharSet::new();
        let mut images: Vec<u32> = Vec::new();
        let mut visit = |c: u32| {
            let m = canonicalize(c, unicode);
            if m != c && self.contains(c) {
                moved.add(c);
                images.push(m);
            }
        };
        if unicode {
            case::for_each_fold(|c, _| visit(c));
        } else {
            case::for_each_upper(|c, _| visit(c));
        }
        if moved.is_empty() {
            return self.clone();
        }
        images.sort_unstable();
        let mut img = CharSet::new();
        for c in images {
            img.add(c);
        }
        self.subtract(&moved).union(&img)
    }
}

/// The spec's Canonicalize(rer, ch): simple case folding in unicode modes; otherwise
/// toUppercase, unless that gives several characters or maps a non-ASCII character to
/// ASCII.
pub(crate) fn canonicalize(c: u32, unicode: bool) -> u32 {
    if c < 0x80 {
        if unicode {
            if (0x41..=0x5A).contains(&c) {
                return c + 0x20;
            }
        } else if (0x61..=0x7A).contains(&c) {
            return c - 0x20;
        }
        return c;
    }
    if unicode {
        return case::simple_fold(c);
    }
    match case::to_upper_full(c).single() {
        Some(u) if u >= 0x80 => u,
        _ => c,
    }
}

/// `\d`
pub(crate) fn digits() -> CharSet {
    CharSet::from_ranges(&[(0x30, 0x39)])
}

/// `\s`: WhiteSpace and LineTerminator.
pub(crate) fn spaces() -> CharSet {
    CharSet::from_ranges(&[
        (0x09, 0x0D),
        (0x20, 0x20),
        (0xA0, 0xA0),
        (0x1680, 0x1680),
        (0x2000, 0x200A),
        (0x2028, 0x2029),
        (0x202F, 0x202F),
        (0x205F, 0x205F),
        (0x3000, 0x3000),
        (0xFEFF, 0xFEFF),
    ])
}

/// `\w` (the basic word characters; see `is_word_char` for the `ui` extras).
pub(crate) fn word() -> CharSet {
    CharSet::from_ranges(&[(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)])
}

/// IsWordChar for `\b`: ASCII word characters, plus U+017F and U+212A when case-insensitive
/// in a unicode mode (they canonicalize to `s` and `k`).
pub(crate) fn is_word_char(c: u32, unicode_icase: bool) -> bool {
    match c {
        0x30..=0x39 | 0x41..=0x5A | 0x5F | 0x61..=0x7A => true,
        0x17F | 0x212A => unicode_icase,
        _ => false,
    }
}

pub(crate) fn is_line_terminator(c: u32) -> bool {
    matches!(c, 0x0A | 0x0D | 0x2028 | 0x2029)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(r: &[(u32, u32)]) -> CharSet {
        CharSet::from_ranges(r)
    }

    #[test]
    fn set_ops() {
        let a = set(&[(1, 5), (10, 20)]);
        let b = set(&[(4, 12), (30, 30)]);
        assert_eq!(a.union(&b).ranges(), &[(1, 20), (30, 30)]);
        assert_eq!(a.intersect(&b).ranges(), &[(4, 5), (10, 12)]);
        assert_eq!(a.subtract(&b).ranges(), &[(1, 3), (13, 20)]);
        assert_eq!(set(&[]).invert().ranges(), &[(0, MAX_CP)]);
        assert_eq!(set(&[(0, 3)]).invert().ranges(), &[(4, MAX_CP)]);
        assert_eq!(a.invert().invert(), a);
        let mut c = CharSet::new();
        for (lo, hi) in [(10, 12), (1, 2), (3, 4), (13, 13), (20, 30), (5, 25)] {
            c.add_range(lo, hi);
        }
        assert_eq!(c.ranges(), &[(1, 30)]);
        assert!(c.contains(1) && c.contains(30) && !c.contains(0) && !c.contains(31));
    }

    #[test]
    fn canonicalize_chars() {
        assert_eq!(canonicalize('a' as u32, false), 'A' as u32);
        assert_eq!(canonicalize('A' as u32, true), 'a' as u32);
        assert_eq!(canonicalize(0x17F, false), 0x17F); // upper is 'S': ASCII, so unchanged
        assert_eq!(canonicalize(0x17F, true), 's' as u32);
        assert_eq!(canonicalize(0xDF, false), 0xDF); // upper is "SS"
        assert_eq!(canonicalize(0xE9, false), 0xC9);
        assert_eq!(canonicalize(0x1F80, false), 0x1F80); // multi-char upper
        assert_eq!(canonicalize(0x3C2, true), 0x3C3);
    }

    #[test]
    fn canonicalize_sets() {
        let s = set(&[('a' as u32, 'c' as u32)]).canonicalize(false);
        assert_eq!(s.ranges(), &[('A' as u32, 'C' as u32)]);
        let s = set(&[('A' as u32, 'Z' as u32)]).canonicalize(true);
        assert_eq!(s.ranges(), &[('a' as u32, 'z' as u32)]);
        // Inverting first, then canonicalizing keeps everything except the moved ones.
        let s = set(&[('a' as u32, 'a' as u32)]).invert().canonicalize(true);
        assert!(s.contains('a' as u32)); // 'A' canonicalizes to 'a'
        assert!(!s.contains('A' as u32));
    }
}
