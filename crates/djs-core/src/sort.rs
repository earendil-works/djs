//! One small stable sort (0043): `core`'s slice sorts are fast but monomorphize several
//! kilobytes of wasm per element type (small-sort networks, pivot selection), and none of
//! djs's sorts is hot enough to pay for that.

use alloc::vec;
use alloc::vec::Vec;

/// Sorts `v` stably by `less`: a bottom-up merge sort of indexes, then one permutation.
pub(crate) fn sort_by<T>(v: &mut Vec<T>, mut less: impl FnMut(&T, &T) -> bool) {
    let n = v.len();
    if n < 2 {
        return;
    }
    let mut idx: Vec<u32> = (0..n as u32).collect();
    let mut tmp: Vec<u32> = vec![0; n];
    let mut width = 1;
    while width < n {
        let mut i = 0;
        while i < n {
            let mid = (i + width).min(n);
            let end = (i + 2 * width).min(n);
            let (mut a, mut b) = (i, mid);
            for slot in &mut tmp[i..end] {
                // Take from the right run only when strictly less: stable.
                let right = b < end && (a >= mid || less(&v[idx[b] as usize], &v[idx[a] as usize]));
                if right {
                    *slot = idx[b];
                    b += 1;
                } else {
                    *slot = idx[a];
                    a += 1;
                }
            }
            i = end;
        }
        core::mem::swap(&mut idx, &mut tmp);
        width *= 2;
    }
    let mut items: Vec<Option<T>> = v.drain(..).map(Some).collect();
    v.extend(
        idx.iter()
            .map(|&i| items[i as usize].take().expect("a permutation")),
    );
}

/// [`sort_by`] for code where speed does not matter (the dumper): the merge sort is one
/// copy behind a `dyn` comparison of indexes, only the permutation is per element type.
#[cfg_attr(not(feature = "dump"), allow(dead_code))]
pub(crate) fn sort_cold<T>(v: &mut Vec<T>, mut less: impl FnMut(&T, &T) -> bool) {
    let idx = order(v.len(), &mut |a, b| less(&v[a], &v[b]));
    for (a, b) in swaps(idx) {
        v.swap(a as usize, b as usize);
    }
}

/// The swaps that put element `idx[k]` at `k`, cycle by cycle.
#[cfg_attr(not(feature = "dump"), allow(dead_code))]
#[inline(never)]
fn swaps(idx: Vec<u32>) -> Vec<(u32, u32)> {
    let mut done = vec![false; idx.len()];
    let mut out = Vec::new();
    for s in 0..idx.len() {
        let mut k = s;
        while !done[k] {
            done[k] = true;
            let j = idx[k] as usize;
            if j == s {
                break;
            }
            out.push((k as u32, j as u32));
            k = j;
        }
    }
    out
}

/// The stable order of `0..n` by `less` on indexes.
#[cfg_attr(not(feature = "dump"), allow(dead_code))]
#[inline(never)]
fn order(n: usize, less: &mut dyn FnMut(usize, usize) -> bool) -> Vec<u32> {
    let mut idx: Vec<u32> = (0..n as u32).collect();
    let mut tmp: Vec<u32> = vec![0; n];
    let mut width = 1;
    while width < n {
        let mut i = 0;
        while i < n {
            let mid = (i + width).min(n);
            let end = (i + 2 * width).min(n);
            let (mut a, mut b) = (i, mid);
            for slot in &mut tmp[i..end] {
                let right = b < end && (a >= mid || less(idx[b] as usize, idx[a] as usize));
                if right {
                    *slot = idx[b];
                    b += 1;
                } else {
                    *slot = idx[a];
                    a += 1;
                }
            }
            i = end;
        }
        core::mem::swap(&mut idx, &mut tmp);
        width *= 2;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_stably() {
        for n in 0..70u32 {
            let mut v: Vec<(u32, u32)> = (0..n).map(|i| ((i * 7919) % 13, i)).collect();
            let mut want = v.clone();
            want.sort_by_key(|e| e.0);
            let mut w = v.clone();
            sort_by(&mut v, |a, b| a.0 < b.0);
            assert_eq!(v, want);
            sort_cold(&mut w, |a, b| a.0 < b.0);
            assert_eq!(w, want);
        }
    }
}
