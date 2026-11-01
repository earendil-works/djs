//! The module's Rust heap allocator, in one place so it can be replaced as a unit.
//!
//! [`Base`] is the allocator underneath everything. It is the global allocator, unless the
//! `alloc-stats` feature wraps it in a counting allocator (lib.rs).
//! - **wasm32:** djs-wasm is `no_std` there (D2332), and [`Base`] is [`PageAlloc`] over
//!   the module's linear memory (D2053).
//! - **natively:** std's system allocator.
//!
//! # Why a page allocator (D2053)
//!
//! A VM's arena is one `Vec<u8>` that grows to megabytes. With dlmalloc alone, a growth
//! that cannot extend in place copies the arena into a new block, and the old and the new
//! block exist at the same time. Most peaks of wasm memory were such a moment, and since
//! whether a block can extend depends on exact sizes, the peaks were a chaotic function
//! of the heap's sizes (D2047). Wasm memory never shrinks, so the peak is what a run
//! costs.
//!
//! So the linear memory is managed in 64 KiB pages ([`Pages`]):
//! - **Huge blocks** (at least [`HUGE`] bytes: in practice the arena, big strings and
//!   side tables) are runs of whole pages. A huge block grows in place when the pages
//!   after it are free or when it ends at the end of memory, where `memory.grow` extends
//!   it. Freed pages go back to a coalescing free list.
//! - **Everything smaller** is dlmalloc's, as before, but dlmalloc gets its memory from
//!   the same pages instead of growing memory itself. Pages a huge block freed serve
//!   dlmalloc too, and dlmalloc's released segments serve huge blocks.
//!
//! (Not named `alloc`: that is the `alloc` crate inside this crate.)

/// The page manager and the allocator over it, generic over the memory so that native
/// tests can run them.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod pages {
    use core::alloc::Layout;
    use core::ptr;

    /// Bytes per page: a wasm page.
    pub(crate) const PAGE: usize = 64 * 1024;

    /// Blocks at least this large are whole pages of their own (not dlmalloc's).
    pub(crate) const HUGE: usize = 256 * 1024;

    /// Free page ranges kept: the allocator cannot allocate for itself, so the list is a
    /// fixed array. Past it, the smallest range is dropped (leaked); in practice a VM has a
    /// handful of free ranges.
    const FREE_CAP: usize = 64;

    /// Linear memory: a run of pages that can only grow.
    pub(crate) trait Memory {
        /// The address of page 0.
        fn base(&self) -> usize;
        /// The current size in pages.
        fn pages(&self) -> usize;
        /// Grows by `n` pages; the old size, or `None` if the host refused.
        fn grow(&mut self, n: usize) -> Option<usize>;
    }

    /// The page manager: which pages of the memory are free. Pages not free belong to a huge
    /// block or to one of dlmalloc's segments.
    pub(crate) struct Pages<M> {
        mem: M,
        /// Free runs `(first page, pages)`, sorted by address, never adjacent (coalesced).
        free: [(usize, usize); FREE_CAP],
        n: usize,
    }

    impl<M: Memory> Pages<M> {
        pub(crate) const fn new(mem: M) -> Pages<M> {
            Pages {
                mem,
                free: [(0, 0); FREE_CAP],
                n: 0,
            }
        }

        pub(crate) fn addr(&self, page: usize) -> usize {
            self.mem.base() + page * PAGE
        }

        pub(crate) fn page(&self, addr: usize) -> usize {
            (addr - self.mem.base()) / PAGE
        }

        /// `n` pages: the best-fitting free run (from its start), else memory's end (taking a
        /// free run that ends there and growing the rest).
        pub(crate) fn alloc(&mut self, n: usize) -> Option<usize> {
            let best = (0..self.n)
                .filter(|&i| self.free[i].1 >= n)
                .min_by_key(|&i| self.free[i].1);
            if let Some(i) = best {
                let (start, len) = self.free[i];
                if len == n {
                    self.remove(i);
                } else {
                    self.free[i] = (start + n, len - n);
                }
                return Some(start);
            }
            let end = self.mem.pages();
            if self.n > 0 && self.free[self.n - 1].0 + self.free[self.n - 1].1 == end {
                let (start, len) = self.free[self.n - 1];
                self.mem.grow(n - len)?;
                self.n -= 1;
                return Some(start);
            }
            self.mem.grow(n)
        }

        /// Grows the run of `len` pages at `start` to `new` pages without moving it: into the
        /// free pages after it, and at memory's end by growing memory. False if it cannot.
        pub(crate) fn extend(&mut self, start: usize, len: usize, new: usize) -> bool {
            let after = start + len;
            let need = new - len;
            let end = self.mem.pages();
            if after == end {
                return self.mem.grow(need).is_some();
            }
            let Some(i) = (0..self.n).find(|&i| self.free[i].0 == after) else {
                return false;
            };
            let (fs, fl) = self.free[i];
            if fl > need {
                self.free[i] = (fs + need, fl - need);
                return true;
            }
            // All of the free run, and memory's end for the rest if the run ends there.
            if fl == need || (fs + fl == end && self.mem.grow(need - fl).is_some()) {
                self.remove(i);
                return true;
            }
            false
        }

        /// Returns `len` pages at `start` to the free list.
        pub(crate) fn free(&mut self, start: usize, len: usize) {
            if len == 0 {
                return;
            }
            let i = (0..self.n)
                .find(|&i| self.free[i].0 > start)
                .unwrap_or(self.n);
            let joins_prev = i > 0 && self.free[i - 1].0 + self.free[i - 1].1 == start;
            let joins_next = i < self.n && start + len == self.free[i].0;
            match (joins_prev, joins_next) {
                (true, true) => {
                    self.free[i - 1].1 += len + self.free[i].1;
                    self.remove(i);
                }
                (true, false) => self.free[i - 1].1 += len,
                (false, true) => self.free[i] = (start, len + self.free[i].1),
                (false, false) => {
                    if self.n == FREE_CAP {
                        // Full: keep the larger runs. Dropped pages are leaked.
                        let small = (0..self.n).min_by_key(|&j| self.free[j].1).unwrap();
                        if self.free[small].1 >= len {
                            return;
                        }
                        self.remove(small);
                        return self.free(start, len);
                    }
                    self.free.copy_within(i..self.n, i + 1);
                    self.free[i] = (start, len);
                    self.n += 1;
                }
            }
        }

        fn remove(&mut self, i: usize) {
            self.free.copy_within(i + 1..self.n, i);
            self.n -= 1;
        }

        /// The free runs, for tests.
        #[cfg(test)]
        pub(crate) fn free_runs(&self) -> &[(usize, usize)] {
            &self.free[..self.n]
        }

        #[cfg(test)]
        pub(crate) fn memory(&self) -> &M {
            &self.mem
        }
    }

    /// dlmalloc's system allocator: segments are pages of the [`Pages`] manager.
    pub(crate) struct Sys<M> {
        pages: *mut Pages<M>,
    }

    // SAFETY: wasm32 has one thread; natively only tests use it, each on its own instance.
    unsafe impl<M> Send for Sys<M> {}

    // SAFETY: the segments are page runs that only dlmalloc uses until it frees them; the
    // pointer to the manager outlives the allocator (it is next to it in `PageAlloc`, which
    // does not move once initialized).
    unsafe impl<M: Memory> dlmalloc::Allocator for Sys<M> {
        fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
            // SAFETY: the manager outlives the allocator and nothing else is borrowing it
            // (the allocator is not reentrant).
            let pages = unsafe { &mut *self.pages };
            let n = size.div_ceil(PAGE);
            match pages.alloc(n) {
                Some(p) => (pages.addr(p) as *mut u8, n * PAGE, 0),
                None => (ptr::null_mut(), 0, 0),
            }
        }

        fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
            ptr::null_mut()
        }

        fn free_part(&self, ptr: *mut u8, old: usize, new: usize) -> bool {
            // Only whole pages go back.
            if !old.is_multiple_of(PAGE) || !new.is_multiple_of(PAGE) {
                return false;
            }
            // SAFETY: as in `alloc`.
            let pages = unsafe { &mut *self.pages };
            let start = pages.page(ptr as usize);
            pages.free(start + new / PAGE, (old - new) / PAGE);
            true
        }

        fn free(&self, ptr: *mut u8, size: usize) -> bool {
            if !size.is_multiple_of(PAGE) {
                return false;
            }
            // SAFETY: as in `alloc`.
            let pages = unsafe { &mut *self.pages };
            pages.free(pages.page(ptr as usize), size / PAGE);
            true
        }

        fn can_release_part(&self, _flags: u32) -> bool {
            true
        }

        fn allocates_zeros(&self) -> bool {
            // Reused pages are not zeroed.
            false
        }

        fn page_size(&self) -> usize {
            PAGE
        }
    }

    /// The allocator: huge blocks as page runs, the rest dlmalloc's on pages too.
    pub(crate) struct PageAlloc<M> {
        pages: Pages<M>,
        dl: dlmalloc::Dlmalloc<Sys<M>>,
    }

    /// Whether a block of `layout` is huge (its own pages).
    fn huge(size: usize, align: usize) -> bool {
        size >= HUGE && align <= PAGE
    }

    impl<M: Memory> PageAlloc<M> {
        pub(crate) const fn new(mem: M) -> PageAlloc<M> {
            PageAlloc {
                pages: Pages::new(mem),
                dl: dlmalloc::Dlmalloc::new_with_allocator(Sys {
                    pages: ptr::null_mut(),
                }),
            }
        }

        /// Points dlmalloc at the page manager. Must run before the first allocation, once
        /// `self` is at its final address.
        ///
        /// # Safety
        ///
        /// `self` must not move afterwards.
        pub(crate) unsafe fn init(&mut self) {
            let pages: *mut Pages<M> = &mut self.pages;
            self.dl = dlmalloc::Dlmalloc::new_with_allocator(Sys { pages });
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::alloc`; `init` has run.
        pub(crate) unsafe fn alloc(&mut self, l: Layout) -> *mut u8 {
            if huge(l.size(), l.align()) {
                match self.pages.alloc(l.size().div_ceil(PAGE)) {
                    Some(p) => self.pages.addr(p) as *mut u8,
                    None => ptr::null_mut(),
                }
            } else {
                // SAFETY: the caller's contract.
                unsafe { self.dl.malloc(l.size(), l.align()) }
            }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::alloc_zeroed`; `init` has run.
        pub(crate) unsafe fn alloc_zeroed(&mut self, l: Layout) -> *mut u8 {
            if huge(l.size(), l.align()) {
                // SAFETY: the caller's contract; the block holds `l.size()` bytes.
                unsafe {
                    let p = self.alloc(l);
                    if !p.is_null() {
                        ptr::write_bytes(p, 0, l.size());
                    }
                    p
                }
            } else {
                // SAFETY: the caller's contract.
                unsafe { self.dl.calloc(l.size(), l.align()) }
            }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::dealloc`; `init` has run.
        pub(crate) unsafe fn dealloc(&mut self, p: *mut u8, l: Layout) {
            if huge(l.size(), l.align()) {
                let start = self.pages.page(p as usize);
                self.pages.free(start, l.size().div_ceil(PAGE));
            } else {
                // SAFETY: the caller's contract.
                unsafe { self.dl.free(p, l.size(), l.align()) }
            }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::realloc`; `init` has run.
        pub(crate) unsafe fn realloc(&mut self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            let (old_huge, new_huge) = (huge(l.size(), l.align()), huge(new, l.align()));
            if !old_huge && !new_huge {
                // SAFETY: the caller's contract.
                return unsafe { self.dl.realloc(p, l.size(), l.align(), new) };
            }
            if old_huge && new_huge {
                let start = self.pages.page(p as usize);
                let (len, want) = (l.size().div_ceil(PAGE), new.div_ceil(PAGE));
                if want <= len {
                    self.pages.free(start + want, len - want);
                    return p;
                }
                if self.pages.extend(start, len, want) {
                    return p;
                }
            }
            // Moves (between dlmalloc and pages, or a huge block that cannot extend).
            // SAFETY: the caller's contract: `new` is a valid size for `l.align()`; both
            // blocks hold at least `min(old, new)` bytes and do not overlap.
            unsafe {
                let nl = Layout::from_size_align_unchecked(new, l.align());
                let q = self.alloc(nl);
                if !q.is_null() {
                    ptr::copy_nonoverlapping(p, q, l.size().min(new));
                    self.dealloc(p, l);
                }
                q
            }
        }

        /// Adds pages the memory already has (wasm: those after the static data) to the
        /// free list.
        pub(crate) fn free_pages(&mut self, start: usize, n: usize) {
            self.pages.free(start, n);
        }

        #[cfg(test)]
        pub(crate) fn pages(&self) -> &Pages<M> {
            &self.pages
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::pages::{Memory, PAGE, PageAlloc};
    use core::alloc::{GlobalAlloc, Layout};
    use core::arch::wasm32;
    use core::cell::UnsafeCell;

    /// The module's linear memory.
    pub(crate) struct Linear;

    impl Memory for Linear {
        fn base(&self) -> usize {
            0
        }
        fn pages(&self) -> usize {
            wasm32::memory_size(0)
        }
        fn grow(&mut self, n: usize) -> Option<usize> {
            match wasm32::memory_grow(0, n) {
                usize::MAX => None,
                old => Some(old),
            }
        }
    }

    unsafe extern "C" {
        /// The first byte after the module's static data (the linker's).
        static __heap_base: u8;
    }

    struct State {
        a: UnsafeCell<PageAlloc<Linear>>,
        ready: UnsafeCell<bool>,
    }

    // SAFETY: wasm32 without threads: one caller at a time.
    unsafe impl Sync for State {}

    static STATE: State = State {
        a: UnsafeCell::new(PageAlloc::new(Linear)),
        ready: UnsafeCell::new(false),
    };

    /// The allocator, initialized on first use: dlmalloc is pointed at the page manager,
    /// and the whole pages between the static data and the initial memory end are free.
    ///
    /// # Safety
    ///
    /// Not reentrant (the allocator never allocates).
    unsafe fn get() -> &'static mut PageAlloc<Linear> {
        // SAFETY: one thread; the static never moves; nothing else holds a reference.
        unsafe {
            let a = &mut *STATE.a.get();
            if !*STATE.ready.get() {
                *STATE.ready.get() = true;
                a.init();
                let first = (&raw const __heap_base as usize).div_ceil(PAGE);
                let end = wasm32::memory_size(0);
                if first < end {
                    a.free_pages(first, end - first);
                }
            }
            a
        }
    }

    /// The global allocator on wasm32 (allocator.rs docs).
    pub(crate) struct Base;

    // SAFETY: forwards to `PageAlloc`, which keeps the `GlobalAlloc` contracts (blocks
    // are disjoint, live until freed, aligned as asked).
    unsafe impl GlobalAlloc for Base {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            // SAFETY: the caller's contract.
            unsafe { get().alloc(l) }
        }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
            // SAFETY: the caller's contract.
            unsafe { get().alloc_zeroed(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            // SAFETY: the caller's contract.
            unsafe { get().dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            // SAFETY: the caller's contract.
            unsafe { get().realloc(p, l, new) }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::Base;
// Used natively only by the `alloc-stats` wrapper.
#[cfg(all(not(target_arch = "wasm32"), feature = "alloc-stats"))]
pub(crate) use std::alloc::System as Base;

// Natively std's allocator is already the global one.
#[cfg(all(target_arch = "wasm32", not(feature = "alloc-stats")))]
#[global_allocator]
static ALLOC: Base = Base;

#[cfg(test)]
mod tests;
