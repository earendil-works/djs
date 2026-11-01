//! The page allocator natively, over a fake linear memory (D2053).

extern crate std;

use super::pages::*;
use core::alloc::Layout;
use std::boxed::Box;
use std::vec::Vec;

/// A linear memory of up to `cap` pages, in one host allocation (lazily committed).
struct Fake {
    base: *mut u8,
    pages: usize,
    cap: usize,
}

impl Fake {
    fn new(cap: usize) -> Fake {
        let l = Layout::from_size_align(cap * PAGE, PAGE).unwrap();
        // SAFETY: a non-zero size.
        let base = unsafe { std::alloc::alloc_zeroed(l) };
        assert!(!base.is_null());
        Fake {
            base,
            pages: 0,
            cap,
        }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let l = Layout::from_size_align(self.cap * PAGE, PAGE).unwrap();
        // SAFETY: allocated in `new` with this layout.
        unsafe { std::alloc::dealloc(self.base, l) }
    }
}

impl Memory for Fake {
    fn base(&self) -> usize {
        self.base as usize
    }
    fn pages(&self) -> usize {
        self.pages
    }
    fn grow(&mut self, n: usize) -> Option<usize> {
        if self.pages + n > self.cap {
            return None;
        }
        let old = self.pages;
        self.pages += n;
        Some(old)
    }
}

#[test]
fn pages_coalesce_and_fit() {
    let mut p = Pages::new(Fake::new(64));
    let a = p.alloc(2).unwrap();
    let b = p.alloc(3).unwrap();
    let c = p.alloc(1).unwrap();
    assert_eq!((a, b, c), (0, 2, 5));
    assert_eq!(p.memory().pages(), 6);
    p.free(a, 2);
    p.free(c, 1);
    assert_eq!(p.free_runs(), &[(0, 2), (5, 1)]);
    // Best fit: the 1-page run, not the 2-page one.
    assert_eq!(p.alloc(1), Some(5));
    p.free(5, 1);
    p.free(b, 3);
    assert_eq!(p.free_runs(), &[(0, 6)], "coalesced on both sides");
    // A run that ends at memory's end grows into a larger request.
    assert_eq!(p.alloc(8), Some(0));
    assert_eq!(p.memory().pages(), 8);
    assert!(p.free_runs().is_empty());
}

#[test]
fn pages_extend_in_place() {
    let mut p = Pages::new(Fake::new(64));
    let a = p.alloc(2).unwrap();
    // At memory's end: grows memory.
    assert!(p.extend(a, 2, 5));
    assert_eq!(p.memory().pages(), 5);
    let b = p.alloc(1).unwrap();
    assert_eq!(b, 5);
    // Blocked by `b`.
    assert!(!p.extend(a, 5, 6));
    p.free(b, 1);
    // Into the free page and memory's end.
    assert!(p.extend(a, 5, 9));
    assert_eq!(p.memory().pages(), 9);
    assert!(p.free_runs().is_empty());
    // Refused by the host: no change.
    assert!(!p.extend(a, 9, 100));
    assert_eq!(p.memory().pages(), 9);
}

#[test]
fn a_full_free_list_keeps_the_larger_runs() {
    let mut p = Pages::new(Fake::new(1024));
    let blocks: Vec<usize> = (0..300).map(|i| p.alloc(1 + i % 3).unwrap()).collect();
    for (i, &b) in blocks.iter().enumerate().step_by(2) {
        p.free(b, 1 + i % 3);
    }
    let runs = p.free_runs();
    assert!(runs.len() <= 64);
    assert!(
        runs.windows(2).all(|w| w[0].0 + w[0].1 < w[1].0),
        "sorted, apart"
    );
    assert!(
        runs.iter().all(|r| r.1 >= 2),
        "the 1-page runs were dropped first"
    );
}

/// A PageAlloc on a fake memory of `cap` pages, at a fixed address (it is
/// self-referential once initialized).
fn page_alloc(cap: usize) -> &'static mut PageAlloc<Fake> {
    let mut mem = Fake::new(cap);
    // As in wasm: memory that exists already (two pages) starts out free.
    mem.grow(2);
    let a = Box::leak(Box::new(PageAlloc::new(mem)));
    // SAFETY: leaked: it never moves.
    unsafe { a.init() };
    a.free_pages(0, 2);
    a
}

/// A block of the stress test: its address, layout and fill byte.
struct Block {
    p: *mut u8,
    l: Layout,
    fill: u8,
}

fn check(b: &Block) {
    // SAFETY: a live block of `b.l.size()` bytes.
    let s = unsafe { core::slice::from_raw_parts(b.p, b.l.size()) };
    let bad = s.iter().position(|&x| x != b.fill);
    assert_eq!(bad, None, "block {:p} of {} bytes damaged", b.p, b.l.size());
}

fn fill(b: &Block) {
    // SAFETY: a live block of `b.l.size()` bytes.
    unsafe { core::ptr::write_bytes(b.p, b.fill, b.l.size()) }
}

#[test]
fn random_allocations_keep_their_contents() {
    // Small (dlmalloc), medium and huge (page runs) blocks, allocated, grown, shrunk and
    // freed at random; every block's bytes are checked before it changes, and no two live
    // blocks overlap.
    let a = page_alloc(16 * 1024); // 1 GiB
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let size = |r: u64| -> usize {
        match r % 10 {
            0..=4 => 1 + (r >> 8) as usize % 4096,
            5..=7 => 4096 + (r >> 8) as usize % (300 * 1024),
            _ => HUGE + (r >> 8) as usize % (3 << 20),
        }
    };
    let mut live: Vec<Block> = Vec::new();
    for step in 0..3000u32 {
        let r = next();
        let op = if live.len() > 48 { r % 4 } else { r % 2 };
        match op {
            0 => {
                let align = 1usize << ((r >> 32) % 13); // up to 4096
                let l = Layout::from_size_align(size(next()), align).unwrap();
                // SAFETY: a non-zero size.
                let p = unsafe {
                    if r & (1 << 40) != 0 {
                        let p = a.alloc_zeroed(l);
                        let z = core::slice::from_raw_parts(p, l.size());
                        assert!(z.iter().all(|&x| x == 0), "alloc_zeroed");
                        p
                    } else {
                        a.alloc(l)
                    }
                };
                assert!(!p.is_null());
                assert_eq!(p as usize % align, 0);
                let b = Block {
                    p,
                    l,
                    fill: step as u8,
                };
                fill(&b);
                live.push(b);
            }
            1 if !live.is_empty() => {
                let i = (r >> 16) as usize % live.len();
                let b = &mut live[i];
                check(b);
                let new = size(next());
                // SAFETY: a live block with its layout; a non-zero size.
                let q = unsafe { a.realloc(b.p, b.l, new) };
                assert!(!q.is_null());
                let keep = b.l.size().min(new);
                // SAFETY: the kept prefix of the moved block.
                let s = unsafe { core::slice::from_raw_parts(q, keep) };
                assert!(s.iter().all(|&x| x == b.fill), "realloc lost contents");
                b.p = q;
                b.l = Layout::from_size_align(new, b.l.align()).unwrap();
                b.fill = b.fill.wrapping_add(1);
                fill(b);
            }
            _ if !live.is_empty() => {
                let i = (r >> 16) as usize % live.len();
                let b = live.swap_remove(i);
                check(&b);
                // SAFETY: a live block with its layout.
                unsafe { a.dealloc(b.p, b.l) };
            }
            _ => {}
        }
        if step % 250 == 0 {
            let mut spans: Vec<(usize, usize)> = live
                .iter()
                .map(|b| (b.p as usize, b.p as usize + b.l.size()))
                .collect();
            spans.sort();
            assert!(spans.windows(2).all(|w| w[0].1 <= w[1].0), "overlap");
            live.iter().for_each(check);
        }
    }
    for b in live.drain(..) {
        check(&b);
        // SAFETY: a live block with its layout.
        unsafe { a.dealloc(b.p, b.l) };
    }
}

#[test]
fn a_growing_arena_stays_in_place_and_memory_tracks_it() {
    // An arena-like block grows by quarters to 48 MiB while small allocations come and go
    // (the Rust heap between collections): memory ends near the arena's size plus the small
    // heap, not near twice the arena, as with copying growth.
    let a = page_alloc(16 * 1024);
    let mut l = Layout::from_size_align(1 << 20, 8).unwrap();
    // SAFETY: a non-zero size.
    let mut arena = unsafe { a.alloc(l) };
    let mut small: Vec<(*mut u8, Layout)> = Vec::new();
    let mut moves = 0;
    while l.size() < 48 << 20 {
        for i in 0..50 {
            let sl = Layout::from_size_align(64 + i * 37, 8).unwrap();
            // SAFETY: a non-zero size.
            small.push((unsafe { a.alloc(sl) }, sl));
        }
        // The collector's side tables: big but not huge, freed again.
        let side = Layout::from_size_align(l.size() / 64, 8).unwrap();
        // SAFETY: a non-zero size.
        let t = unsafe { a.alloc(side) };
        // SAFETY: allocated above with `side`.
        unsafe { a.dealloc(t, side) };
        for (p, sl) in small.drain(..small.len() / 2) {
            // SAFETY: live, with its layout.
            unsafe { a.dealloc(p, sl) };
        }
        let new = l.size() + l.size() / 4;
        // SAFETY: live, with its layout; a non-zero size.
        let q = unsafe { a.realloc(arena, l, new) };
        moves += usize::from(q != arena);
        arena = q;
        l = Layout::from_size_align(new, 8).unwrap();
    }
    let mem = a.pages().memory().pages() * PAGE;
    assert!(
        mem < l.size() + (8 << 20),
        "memory {} MiB for a {} MiB arena ({moves} moves)",
        mem >> 20,
        l.size() >> 20
    );
}
