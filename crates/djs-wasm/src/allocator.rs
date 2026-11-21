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
//! - **Huge blocks** (at least `HUGE` bytes: in practice the arena and big strings) are
//!   runs of whole pages. A huge block grows in place when the pages after it are free or
//!   when it ends at the end of memory, where `memory.grow` extends it. Otherwise it moves
//!   to the best-fitting free run, and only if there is none, to memory's end. Freed pages
//!   go back to a coalescing free list.
//! - **Everything smaller** is dlmalloc's, as before, but dlmalloc gets its memory from
//!   the same pages instead of growing memory itself, so the pages a huge block freed
//!   serve it too. It keeps its segments, as it did before.
//! - **Medium blocks** (16 KiB up to `HUGE`) are dlmalloc's while its segments have room,
//!   else page runs of their own, and dlmalloc's first segment comes before the arena:
//!   both keep the arena at memory's end (D2541, [`PageAlloc`]).
//! - **A compile's small blocks** come from a bump zone taken from dlmalloc's free room for
//!   the compile and given back after it (D3361).
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
    /// The side tables of a collection and most Rust buffers stay below it; the arena is
    /// above from its start (the base image alone is about 180 KB, and a VM's arena starts
    /// at the image in whole pages, D3141).
    pub(crate) const HUGE: usize = 128 * 1024;

    /// Free page ranges kept: the allocator cannot allocate for itself, so the list is a
    /// fixed array. A run freed while it is full is leaked; in practice a VM has a handful
    /// of free ranges.
    const FREE_CAP: usize = 64;

    /// Pages an allocation leaves to the top block's growth (see [`PageAlloc`]): a free run
    /// that starts at one of `near` (right after the top block) is used only from page
    /// `from` on.
    #[derive(Clone, Copy)]
    pub(crate) struct Keep {
        near: [usize; 2],
        from: usize,
    }

    const NO_KEEP: Keep = Keep {
        near: [usize::MAX; 2],
        from: 0,
    };

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

        /// `n` pages: the best-fitting free run (from its start), else memory's end (taking
        /// a free run that ends there and growing the rest).
        pub(crate) fn alloc(&mut self, n: usize) -> Option<usize> {
            self.alloc_keep(n, NO_KEEP)
        }

        /// [`Pages::alloc`], leaving pages free for the top block's growth ([`Keep`]).
        pub(crate) fn alloc_keep(&mut self, n: usize, keep: Keep) -> Option<usize> {
            // The best-fitting free run (the part of it `keep` allows).
            let usable = |(s, len): (usize, usize)| {
                let u = if keep.near.contains(&s) {
                    keep.from.max(s)
                } else {
                    s
                };
                (u, (s + len).saturating_sub(u))
            };
            let (mut best, mut at) = (usize::MAX, 0);
            for i in 0..self.n {
                let (_, len) = usable(self.free[i]);
                if len >= n && len < best {
                    (best, at) = (len, i);
                }
            }
            if best != usize::MAX {
                let (s, len) = self.free[at];
                let (u, _) = usable((s, len));
                self.remove(at);
                if u > s {
                    self.free(s, u - s);
                }
                if u + n < s + len {
                    self.free(u + n, s + len - u - n);
                }
                return Some(u);
            }
            // Else at memory's end, taking (the usable part of) a free run that ends there.
            let end = self.mem.pages();
            let trailing = self.n > 0 && self.free[self.n - 1].0 + self.free[self.n - 1].1 == end;
            let tail = if trailing {
                self.free[self.n - 1].0
            } else {
                end
            };
            let (start, _) = usable((tail, 0));
            // No free run had `n` usable pages, so the block reaches past `end`.
            self.mem.grow(start + n - end)?;
            if trailing {
                self.n -= 1;
            }
            if start > tail {
                self.free(tail, start - tail);
            }
            Some(start)
        }

        /// Grows the block of `len` pages at `start` to `want` pages without moving it:
        /// into the free run after it, and at memory's end by growing memory. False if it
        /// cannot; then nothing changed.
        pub(crate) fn extend(&mut self, start: usize, len: usize, want: usize) -> bool {
            let need = want - len;
            let after = (0..self.n).find(|&i| self.free[i].0 == start + len);
            let after_len = after.map_or(0, |i| self.free[i].1);
            if let Some(i) = after
                && after_len > need
            {
                self.free[i] = (start + len + need, after_len - need);
                return true;
            }
            let fits = after_len == need
                || (start + len + after_len == self.mem.pages()
                    && self.mem.grow(need - after_len).is_some());
            if fits && let Some(i) = after {
                self.remove(i);
            }
            fits
        }

        /// Returns `len` pages at `start` to the free list.
        pub(crate) fn free(&mut self, start: usize, len: usize) {
            if len == 0 || self.n == FREE_CAP {
                // A full list leaks the run (never seen in practice).
                return;
            }
            let mut i = 0;
            while i < self.n && self.free[i].0 < start {
                i += 1;
            }
            self.free.copy_within(i..self.n, i + 1);
            self.free[i] = (start, len);
            self.n += 1;
            // Coalesce with the next run, then with the previous one.
            if i + 1 < self.n && start + len == self.free[i + 1].0 {
                self.free[i].1 += self.free[i + 1].1;
                self.remove(i + 1);
            }
            if i > 0 && self.free[i - 1].0 + self.free[i - 1].1 == start {
                self.free[i - 1].1 += self.free[i].1;
                self.remove(i);
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

    /// Blocks from this size up to `HUGE` go to dlmalloc only while its segments have room;
    /// otherwise they are page runs of their own (see [`PageAlloc`]).
    pub(crate) const MEDIUM: usize = 16 * 1024;

    /// Medium page runs live at once: a collection's side tables are four.
    const MEDIUM_CAP: usize = 16;

    /// dlmalloc's system allocator: segments are pages of the [`Pages`] manager.
    pub(crate) struct Sys<M> {
        pages: *mut Pages<M>,
        /// Refuse new segments (a medium block that dlmalloc cannot place without growing).
        no_grow: core::cell::Cell<bool>,
        /// Bytes `(address, length)` right before the first free page (the rest of the
        /// static data's last page, D3281): the first segment, if it starts at that page,
        /// starts here instead.
        head: core::cell::Cell<(usize, usize)>,
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
            if self.no_grow.get() {
                return (ptr::null_mut(), 0, 0);
            }
            let n = size.div_ceil(PAGE);
            match pages.alloc(n) {
                Some(p) => {
                    let (at, len) = self.head.take();
                    if len > 0 && at + len == pages.addr(p) {
                        return (at as *mut u8, len + n * PAGE, 0);
                    }
                    (pages.addr(p) as *mut u8, n * PAGE, 0)
                }
                None => (ptr::null_mut(), 0, 0),
            }
        }

        fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
            ptr::null_mut()
        }

        fn free_part(&self, _ptr: *mut u8, _old: usize, _new: usize) -> bool {
            false
        }

        fn free(&self, _ptr: *mut u8, _size: usize) -> bool {
            false
        }

        fn can_release_part(&self, _flags: u32) -> bool {
            // dlmalloc keeps its segments, as it did over memory.grow alone: releasing
            // them was measured to save no memory and cost code (D2053); a segment that
            // blocks the arena is still in use when it matters (0158).
            false
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
    ///
    /// The arena should end where memory ends, so that it grows in place; whatever lands
    /// right after it blocks that until it is freed, and then the arena moves and leaves
    /// its old pages behind as a hole (D2541). So:
    /// - dlmalloc gets its first segment before anything else (`prime`), below the arena;
    /// - a medium block (`MEDIUM` up to `HUGE`) that dlmalloc's segments cannot hold is a
    ///   page run of its own instead of a new segment. dlmalloc never returns a segment,
    ///   so one taken right after the arena would block it for good, while a page run is
    ///   freed and the arena grows over it. Medium blocks are mostly a collection's side
    ///   tables, which scale with the heap and live for one collection;
    /// - the largest huge block is the *top* block (in practice the arena). A buffer that
    ///   grows into a page run right behind it starts its own size further on (`Keep`).
    pub(crate) struct PageAlloc<M> {
        pages: Pages<M>,
        dl: dlmalloc::Dlmalloc<Sys<M>>,
        /// The live medium page runs `(first page, pages)`; `(usize::MAX, 0)` is a free slot.
        medium: [(usize, usize); MEDIUM_CAP],
        /// The largest huge block `(first page, pages)`, usually the arena; `(usize::MAX, 0)`:
        /// none.
        top: (usize, usize),
        /// What a page run allocated now leaves to the top block.
        keep: Keep,
        /// The compile zone (D3361).
        zone: Zone,
    }

    /// The compile zone's size.
    const ZONE: usize = 32 * 1024;

    /// A bump region for the small allocations of a compile (D3361): a compile makes
    /// 400-700 short-lived ones (code buffers, a function's state, its tables), which
    /// cost it about a tenth of its time in dlmalloc. Allocating bumps `top`; freeing the
    /// last block moves it back, and the last free of all resets it. A block that
    /// outlives the compile only delays that; what does not fit goes to dlmalloc.
    struct Zone {
        start: usize,
        end: usize,
        top: usize,
        live: usize,
        on: bool,
    }

    impl Zone {
        fn holds(&self, p: *mut u8) -> bool {
            (p as usize).wrapping_sub(self.start) < self.end - self.start
        }

        fn alloc(&mut self, l: Layout) -> Option<*mut u8> {
            let p = self.top.next_multiple_of(l.align());
            let end = p.checked_add(l.size())?;
            (end <= self.end).then(|| {
                self.top = end;
                self.live += 1;
                p as *mut u8
            })
        }

        fn free(&mut self, p: *mut u8, size: usize) {
            self.live -= 1;
            if self.live == 0 {
                self.top = self.start;
            } else if p as usize + size == self.top {
                self.top = p as usize;
            }
        }
    }

    /// The least room dlmalloc's first segment has, the rest of the static data's last
    /// page (`PageAlloc::seed`) and the pages `PageAlloc::prime` adds (D3362). Whatever
    /// dlmalloc needs beyond it becomes a segment at memory's end, right behind the arena,
    /// which then cannot grow in place and moves. Measured on the codemode workloads, warm
    /// and first instances: 102.4 KB keeps every row as with D3360's 104.3 KB; 94 KB put
    /// a collection's tables of heavy into page runs (+64 KiB), 84 KB moved text's arena
    /// (+384 KiB), 76 KB also fanout's and json's.
    pub(crate) const FLOOR: usize = 100 * 1024;

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
                    no_grow: core::cell::Cell::new(false),
                    head: core::cell::Cell::new((0, 0)),
                }),
                medium: [(usize::MAX, 0); MEDIUM_CAP],
                top: (usize::MAX, 0),
                keep: NO_KEEP,
                zone: Zone {
                    start: 0,
                    end: 0,
                    top: 0,
                    live: 0,
                    on: false,
                },
            }
        }

        /// Turns the compile zone on (`true`) or off; returns whether it was on. Its memory
        /// comes from dlmalloc while it is on or holds a block, and goes back when neither
        /// is true any more.
        pub(crate) fn set_zone(&mut self, on: bool) -> bool {
            if on && self.zone.end == 0 {
                // Only if dlmalloc has the room: a new segment would land right after the
                // arena and keep it from growing in place (D2541).
                self.dl.allocator().no_grow.set(true);
                // SAFETY: a valid size and alignment.
                let p = unsafe { self.dl.malloc(ZONE, 16) } as usize;
                self.dl.allocator().no_grow.set(false);
                if p != 0 {
                    self.zone = Zone {
                        start: p,
                        end: p + ZONE,
                        top: p,
                        live: 0,
                        on: false,
                    };
                }
            }
            let was = core::mem::replace(&mut self.zone.on, on && self.zone.end != 0);
            self.release_zone();
            was
        }

        /// Gives the zone's memory back to dlmalloc when it is off and empty.
        fn release_zone(&mut self) {
            if !self.zone.on && self.zone.live == 0 && self.zone.end != 0 {
                // SAFETY: the block `set_zone` took, with its size and alignment.
                unsafe { self.dl.free(self.zone.start as *mut u8, ZONE, 16) };
                self.zone.start = 0;
                self.zone.end = 0;
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
            self.dl = dlmalloc::Dlmalloc::new_with_allocator(Sys {
                pages,
                no_grow: core::cell::Cell::new(false),
                head: core::cell::Cell::new((0, 0)),
            });
        }

        /// Gives dlmalloc its first segment now (the `seed` and the pages that make it `FLOOR`),
        /// below everything else: the first block an instance allocates is usually the
        /// arena, and a segment right after it would keep it from growing in place. It
        /// holds what a VM keeps in dlmalloc and what most compiles need.
        pub(crate) fn prime(&mut self) {
            // The pages that make the seed at least FLOOR.
            let seed = self.dl.allocator().head.get().1;
            let pages = FLOOR.saturating_sub(seed).div_ceil(PAGE);
            if pages == 0 {
                return;
            }
            // SAFETY: a valid size and alignment; the block is freed right away (dlmalloc
            // keeps the segment).
            unsafe {
                let n = pages * PAGE - 4096;
                let p = self.dl.malloc(n, 8);
                if !p.is_null() {
                    self.dl.free(p, n, 8);
                }
            }
        }

        /// Makes the next page runs placed right after the top block start `bytes` further
        /// on (D3142), for a buffer whose contents are about to be copied into the top
        /// block (the arena): it grows into those pages in place instead of moving. Returns
        /// the setting it replaced, for [`PageAlloc::restore_keep`].
        pub(crate) fn leave_top_room(&mut self, bytes: usize) -> Keep {
            let old = self.keep;
            if self.top.0 != usize::MAX {
                let top_end = self.top.0 + self.top.1;
                self.keep = Keep {
                    near: [top_end, usize::MAX],
                    from: top_end + bytes.div_ceil(PAGE),
                };
            }
            old
        }

        /// Puts back what [`PageAlloc::leave_top_room`] replaced.
        pub(crate) fn restore_keep(&mut self, keep: Keep) {
            self.keep = keep;
        }

        /// A huge block at `start` now has `len` pages: it becomes the top block if there is
        /// none or it is at least as large.
        fn note_huge(&mut self, start: usize, len: usize) {
            if self.top.0 == usize::MAX || self.top.0 == start || len >= self.top.1 {
                self.top = (start, len);
            }
        }

        /// The slot of the medium page run starting at `p`, if it is one.
        fn medium_slot(&self, p: *mut u8) -> Option<usize> {
            let addr = p as usize;
            if addr < self.pages.addr(0) || !(addr - self.pages.addr(0)).is_multiple_of(PAGE) {
                return None;
            }
            let page = self.pages.page(addr);
            self.medium.iter().position(|&(s, _)| s == page)
        }

        /// A medium block: in dlmalloc's segments if they have room, else a page run (or,
        /// with no slot left, a new dlmalloc segment after all).
        unsafe fn alloc_medium(&mut self, l: Layout) -> *mut u8 {
            self.dl.allocator().no_grow.set(true);
            // SAFETY: the caller's contract.
            let p = unsafe { self.dl.malloc(l.size(), l.align()) };
            self.dl.allocator().no_grow.set(false);
            if !p.is_null() {
                return p;
            }
            if let Some(slot) = self.medium.iter().position(|&(s, _)| s == usize::MAX) {
                let n = l.size().div_ceil(PAGE);
                return match self.pages.alloc_keep(n, self.keep) {
                    Some(start) => {
                        self.medium[slot] = (start, n);
                        self.pages.addr(start) as *mut u8
                    }
                    None => ptr::null_mut(),
                };
            }
            // SAFETY: the caller's contract.
            unsafe { self.dl.malloc(l.size(), l.align()) }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::alloc`; `init` has run.
        pub(crate) unsafe fn alloc(&mut self, l: Layout) -> *mut u8 {
            if huge(l.size(), l.align()) {
                let n = l.size().div_ceil(PAGE);
                match self.pages.alloc_keep(n, self.keep) {
                    Some(p) => {
                        self.note_huge(p, n);
                        self.pages.addr(p) as *mut u8
                    }
                    None => ptr::null_mut(),
                }
            } else if l.size() >= MEDIUM && l.align() <= PAGE {
                // SAFETY: the caller's contract.
                unsafe { self.alloc_medium(l) }
            } else {
                if self.zone.on
                    && let Some(p) = self.zone.alloc(l)
                {
                    return p;
                }
                // SAFETY: the caller's contract.
                unsafe { self.dl.malloc(l.size(), l.align()) }
            }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::dealloc`; `init` has run.
        pub(crate) unsafe fn dealloc(&mut self, p: *mut u8, l: Layout) {
            if huge(l.size(), l.align()) {
                let start = self.pages.page(p as usize);
                if self.top.0 == start {
                    self.top = (usize::MAX, 0);
                }
                self.pages.free(start, l.size().div_ceil(PAGE));
            } else if let Some(slot) = self.medium_slot(p) {
                let (start, n) = self.medium[slot];
                self.medium[slot] = (usize::MAX, 0);
                self.pages.free(start, n);
            } else if self.zone.holds(p) {
                self.zone.free(p, l.size());
                self.release_zone();
            } else {
                // SAFETY: the caller's contract.
                unsafe { self.dl.free(p, l.size(), l.align()) }
            }
        }

        /// # Safety
        ///
        /// The contract of `GlobalAlloc::realloc`; `init` has run.
        pub(crate) unsafe fn realloc(&mut self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            if self.zone.holds(p) {
                let z = &mut self.zone;
                // The last block grows or shrinks in place.
                if p as usize + l.size() == z.top && p as usize + new <= z.end {
                    z.top = p as usize + new;
                    return p;
                }
                // SAFETY: the caller's contract, as for the moves below.
                unsafe {
                    let q = self.alloc(Layout::from_size_align_unchecked(new, l.align()));
                    if !q.is_null() {
                        ptr::copy_nonoverlapping(p, q, l.size().min(new));
                        self.zone.free(p, l.size());
                        self.release_zone();
                    }
                    return q;
                }
            }
            let (old_huge, new_huge) = (huge(l.size(), l.align()), huge(new, l.align()));
            if !old_huge && let Some(slot) = self.medium_slot(p) {
                let (start, n) = self.medium[slot];
                let want = new.div_ceil(PAGE);
                if !new_huge && want <= n {
                    return p;
                }
                if !new_huge && self.pages.extend(start, n, want) {
                    self.medium[slot] = (start, want);
                    return p;
                }
            } else if !old_huge && !new_huge && new >= MEDIUM && l.align() <= PAGE {
                // A medium block in dlmalloc: grows there if its segments have room.
                self.dl.allocator().no_grow.set(true);
                // SAFETY: the caller's contract.
                let q = unsafe { self.dl.realloc(p, l.size(), l.align(), new) };
                self.dl.allocator().no_grow.set(false);
                if !q.is_null() {
                    return q;
                }
            } else if !old_huge && !new_huge {
                // SAFETY: the caller's contract.
                return unsafe { self.dl.realloc(p, l.size(), l.align(), new) };
            }
            if old_huge && new_huge {
                let start = self.pages.page(p as usize);
                let (len, want) = (l.size().div_ceil(PAGE), new.div_ceil(PAGE));
                if want <= len {
                    if self.top.0 == start {
                        self.top.1 = want;
                    }
                    self.pages.free(start + want, len - want);
                    return p;
                }
                if self.pages.extend(start, len, want) {
                    self.note_huge(start, want);
                    return p;
                }
            }
            if old_huge && self.top.0 == self.pages.page(p as usize) {
                // The top block moves: its new place is the top.
                self.top = (usize::MAX, 0);
            } else if self.top.0 != usize::MAX {
                // A growing buffer, which may well be copied into the arena next (a
                // JSON.stringify result): if it becomes a page run right behind the top
                // block, it starts its own size further on, leaving the top block room for
                // the copy. Its old block, a page run right behind the top block, counts
                // as free there.
                let top_end = self.top.0 + self.top.1;
                let start = self.pages.page(p as usize);
                let old_run = match self.medium_slot(p) {
                    Some(slot) => Some(self.medium[slot].1),
                    None if old_huge => Some(l.size().div_ceil(PAGE)),
                    None => None,
                };
                let after = match old_run {
                    Some(len) if start == top_end => top_end + len,
                    _ => usize::MAX,
                };
                self.keep = Keep {
                    near: [top_end, after],
                    from: top_end + new.div_ceil(PAGE),
                };
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
                self.keep = NO_KEEP;
                q
            }
        }

        /// Adds pages the memory already has (wasm: those after the static data) to the
        /// free list.
        pub(crate) fn free_pages(&mut self, start: usize, n: usize) {
            self.pages.free(start, n);
        }

        /// The `len` bytes at `at`, right before the first free page, become part of
        /// dlmalloc's first segment (`prime`, D3281): in wasm the rest of the static data's
        /// last page, which would otherwise stay unused.
        pub(crate) fn seed(&mut self, at: usize, len: usize) {
            self.dl.allocator().head.set((at, len));
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
                // dlmalloc's chunks are 16-byte aligned.
                let base = (&raw const __heap_base as usize).next_multiple_of(16);
                let first = base.div_ceil(PAGE);
                let end = wasm32::memory_size(0);
                if first < end {
                    a.free_pages(first, end - first);
                }
                a.seed(base, first * PAGE - base);
                a.prime();
            }
            a
        }
    }

    /// Runs `f`, whose page runs placed right after the arena start `bytes` further on
    /// ([`PageAlloc::leave_top_room`]).
    pub(crate) fn with_top_room<R>(bytes: usize, f: impl FnOnce() -> R) -> R {
        // SAFETY: one thread; the reference ends before `f` allocates.
        let old = unsafe { get().leave_top_room(bytes) };
        let r = f();
        // SAFETY: as above.
        unsafe { get().restore_keep(old) };
        r
    }

    /// Runs `f` (a compile) with its small allocations in the compile zone (D3361).
    pub(crate) fn with_zone<R>(f: impl FnOnce() -> R) -> R {
        // SAFETY: one thread; the reference ends before `f` allocates.
        let was = unsafe { get().set_zone(true) };
        let r = f();
        // SAFETY: as above.
        unsafe { get().set_zone(was) };
        r
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
pub(crate) use wasm::{Base, with_top_room, with_zone};

/// Natively there is no compile zone: just `f`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn with_zone<R>(f: impl FnOnce() -> R) -> R {
    f()
}

/// Natively there is no page allocator: just `f`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn with_top_room<R>(_bytes: usize, f: impl FnOnce() -> R) -> R {
    f()
}
// Used natively only by the `alloc-stats` wrapper.
#[cfg(all(not(target_arch = "wasm32"), feature = "alloc-stats"))]
pub(crate) use std::alloc::System as Base;

// Natively std's allocator is already the global one.
#[cfg(all(target_arch = "wasm32", not(feature = "alloc-stats")))]
#[global_allocator]
static ALLOC: Base = Base;

#[cfg(test)]
mod tests;
