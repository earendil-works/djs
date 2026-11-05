//! Allocation counts of work whose speed is mostly the host allocator's (0224): a
//! deterministic guard where timings are too noisy for the gate. Compiling a regexp
//! once cost twice the allocations when the follow sets of `STAR` loops were computed
//! for every concatenation, whether a loop used them or not.
//!
//! The ceilings are today's counts. Lower ones are welcome (lower the ceiling with
//! them); a higher one needs a reason, and the compile benchmarks
//! (`scripts/bench.sh regexp/compile`) to show it is worth it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use djs_core::regexp::{Flags, compile};

struct Counting;

std::thread_local! {
    // Per thread: the test harness runs tests in parallel.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn bump() {
    // `try_with`: allocations while the thread's locals are being torn down are not ours.
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

// SAFETY: forwards to the system allocator; the counter does not allocate.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        bump();
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations(f: impl FnOnce()) -> usize {
    let before = ALLOCS.with(Cell::get);
    f();
    ALLOCS.with(Cell::get) - before
}

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[test]
fn regexp_compile_allocations() {
    // The patterns of djs-bench's regexp/compile group, with their ceilings.
    let cases = [
        (r"\d+", "", 8),
        (r"[\w.+-]+@[\w-]+\.[\w.]+", "", 25),
        (r"https?://([^/\s]+)(/[^\s?#]*)?", "", 36),
        (r"(?<y>\d{4})-(?<m>\d{2})-(?<d>\d{2})", "", 38),
        (r"\bfox\b", "i", 10),
        (r"^\s+|\s+$", "g", 20),
    ];
    let mut over = Vec::new();
    for (pattern, flags, ceiling) in cases {
        let (p, f) = (u16s(pattern), Flags::parse(&u16s(flags)).unwrap());
        let n = allocations(|| drop(compile(&p, f).unwrap()));
        if n > ceiling {
            over.push(format!(
                "/{pattern}/{flags}: {n} allocations, ceiling {ceiling}"
            ));
        }
    }
    assert!(
        over.is_empty(),
        "regexp compile allocates more:\n{}",
        over.join("\n")
    );
}

#[test]
fn tmp_print_allocs() {
    for (pattern, flags) in [
        ("report|metric123", "i"),
        ("report|metric123", ""),
        (r"\s+", "g"),
        (r"^([^:]+):(\d+):(.*)$", ""),
        (r"\b(TODO|FIXME)\b", ""),
        (r"^export (?:async )?function (\w+)\((\w+)", "gm"),
    ] {
        let (p, f) = (u16s(pattern), Flags::parse(&u16s(flags)).unwrap());
        let n = allocations(|| drop(compile(&p, f).unwrap()));
        let parse = allocations(|| drop(djs_core::regexp::parse_only(&p, f)));
        println!("/{pattern}/{flags}: {n} (parse {parse})");
    }
}

#[test]
#[ignore]
fn tmp_compile_loop() {
    let n: usize = std::env::var("N").map(|s| s.parse().unwrap()).unwrap_or(0);
    let pats: Vec<Vec<u16>> = (0..1000)
        .map(|i| u16s(&format!("report|metric{i}")))
        .collect();
    let f = Flags::parse(&u16s(&std::env::var("F").unwrap_or_default())).unwrap();
    for i in 0..n {
        std::hint::black_box(compile(&pats[i % 1000], f).unwrap());
    }
}
