//! The heap: all VM state in one position-independent arena (ARCH "The Core Idea").
//!
//! - [`arena`]: the byte buffer, bounds-checked access, the memory cap.
//! - [`layout`] and [`cells`]: the layout description. Every cell kind and its fields are
//!   declared once in `cells.rs`; accessors, the layout table and the trace description
//!   are generated from it.
//! - [`string`], [`atom`], [`object`]: strings (Latin-1, UTF-16, ropes), atoms and symbols,
//!   objects with properties and dense elements.
//! - [`walk`]: the cell walker, tracing and heap verification.
//!
//! # Conventions
//!
//! - References are `u32` arena offsets of a cell start; 0 is null. Cells are 8-aligned.
//! - The arena is little-endian, contains no host pointers, and can be copied byte for byte
//!   into a fresh [`Heap`] ([`Heap::from_bytes`]): that is the precursor of snapshots.
//! - Allocation never collects. It bumps, may fail with [`HeapError::OutOfMemory`], and
//!   sets [`Heap::gc_pending`] once the heap has grown past its threshold. Rust locals
//!   holding references stay valid until the next [`Heap::collect`], which the owner calls
//!   only at safepoints (ARCH "Generated state machines"). Across a collection, only
//!   [`Handle`]s and the arena's own roots survive.
//! - Allocated memory is zeroed, and zero is a valid non-reference state for every traced
//!   field, so a cell can be traced at any point after allocation.
//!
//! # Roots
//!
//! The [`Roots`] cell lives at offset 0 and holds everything the GC starts
//! from: the atom table, the realm, the [`handle`] table, and (once they exist) the value
//! stack, the job queue and the pending host call table. Rust code holds no roots: the
//! interpreter (0006) keeps live values on the arena value stack, and the embedder holds
//! values through the handle table. A copying GC copies the roots cell first and then
//! scans the to-space linearly from 0 (Cheney), so the roots cell is traced like any
//! other cell.

pub mod arena;
mod arrays;
pub mod atom;
pub mod bigint;
pub mod cells;
pub mod collection;
mod compact;
pub mod gc;
pub mod handle;
pub mod layout;
pub mod object;
pub mod proxy;
pub mod string;
pub mod typed;
pub mod walk;
pub mod weakref;

use core::fmt;

pub use arena::Arena;
pub use atom::{Atom, atoms};
pub use cells::CellKind;
pub use gc::GcStats;
pub use handle::Handle;
pub use layout::Ref;
pub use object::{Lookup, ObjectClass, PropFlags, Property, PropertyDescriptor};
pub use string::StrView;

use cells::{Header, Roots};

use crate::value::Value;

/// Size of the cell header in bytes.
pub const HEADER_SIZE: u32 = 8;

/// Rounds up to a multiple of 8.
#[inline]
pub const fn align8(n: u32) -> u32 {
    (n + 7) & !7
}

/// Errors from heap operations that the caller turns into JS exceptions or a VM status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeapError {
    /// The arena hit its memory cap (or the host allocator failed).
    OutOfMemory,
    /// A string would exceed [`string::MAX_LEN`] code units (a RangeError in JS).
    StringTooLong,
    /// An array length that is not a valid `u32` Number (a RangeError in JS).
    InvalidArrayLength,
}

impl fmt::Display for HeapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HeapError::OutOfMemory => "out of memory",
            HeapError::StringTooLong => "invalid string length",
            HeapError::InvalidArrayLength => "invalid array length",
        })
    }
}

/// Why a heap image was rejected by [`Heap::from_bytes`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoadError {
    /// Bad magic, wrong layout version, or a malformed root cell.
    BadHeader,
    /// The image is larger than the memory cap.
    TooLarge,
    /// The cell structure is broken (see [`walk::VerifyError`]).
    Corrupt(walk::VerifyError),
    /// A VM image from a different build: its bytecode, intrinsic ids or layout do not
    /// match this one (`Roots::BUILD`, D1320).
    BuildMismatch { found: u32, expected: u32 },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::BadHeader => f.write_str("not a djs heap image of this layout"),
            LoadError::TooLarge => f.write_str("the image is larger than the memory limit"),
            LoadError::Corrupt(e) => write!(f, "corrupt heap image: {e:?}"),
            LoadError::BuildMismatch { found, expected } => write!(
                f,
                "the image is from another djs build (build hash {found:08x}, this build \
                 {expected:08x})"
            ),
        }
    }
}

/// The out-of-memory reserve for a cap (D1600): 1/32 of it, at most 1 MiB.
fn reserve_for(limit: u32) -> u32 {
    (limit / 32).min(1 << 20) & !7
}

/// The default memory cap: the whole `u32` offset space.
pub const DEFAULT_LIMIT: usize = arena::MAX_ARENA as usize;

/// The VM heap. All state is in the arena; this struct only adds methods.
pub struct Heap {
    arena: Arena,
    /// Allocation sets `gc_pending` once the arena reaches this size. A copy of
    /// `Roots::GC_THRESHOLD` (D1601), kept here for the allocation fast path.
    gc_threshold: u32,
    gc_pending: bool,
    /// The memory cap. The arena's own limit is `hard_limit - reserve` while the
    /// out-of-memory reserve is closed, and `hard_limit` while it is open (D1600).
    hard_limit: u32,
    reserve: u32,
    /// Test hook: make the next collection fail to reserve its to-space.
    #[cfg(test)]
    pub(crate) fail_next_gc_reserve: bool,
}

impl Heap {
    /// A fresh heap with the default memory cap.
    pub fn new() -> Result<Heap, HeapError> {
        Heap::with_limit(DEFAULT_LIMIT)
    }

    /// A fresh heap: the roots cell, the atom table with the well-known atoms and the
    /// well-known symbols.
    pub fn with_limit(limit: usize) -> Result<Heap, HeapError> {
        let mut heap = Heap::over(Arena::new(limit));
        let roots = heap.alloc_cell(CellKind::Roots, Roots::LAYOUT.min_size)?;
        debug_assert_eq!(roots, 0);
        heap.arena.set(0, Roots::MAGIC, cells::MAGIC);
        heap.arena.set(0, Roots::VERSION, cells::LAYOUT_HASH);
        heap.arena.set(0, Roots::REALM, Value::UNDEFINED);
        let t = heap.gc_threshold;
        heap.set_gc_threshold(t);
        heap.init_atoms()?;
        Ok(heap)
    }

    /// Restores a heap from an image produced by [`Heap::bytes`] of the same build. The
    /// whole image is verified (cell structure and every reference).
    pub fn from_bytes(bytes: alloc::vec::Vec<u8>, limit: usize) -> Result<Heap, LoadError> {
        let heap = Heap::from_bytes_trusted(bytes, limit)?;
        heap.verify().map_err(LoadError::Corrupt)?;
        Ok(heap)
    }

    /// Like [`Heap::from_bytes`] but checks only the header, not every cell: for images
    /// that cannot be corrupt, such as the base snapshot embedded at build time (verified
    /// when it was built). Restoring the 36 KB base takes 8 µs with verification and
    /// 1 µs without (D1321). Still memory safe: a bad cell is a panic, not UB.
    pub fn from_bytes_trusted(bytes: alloc::vec::Vec<u8>, limit: usize) -> Result<Heap, LoadError> {
        let heap = Heap::over(Arena::from_bytes(bytes, limit));
        let a = &heap.arena;
        if a.len() > heap.hard_limit {
            return Err(LoadError::TooLarge);
        }
        if (a.len() as usize) < Roots::LAYOUT.min_size as usize
            || !a.len().is_multiple_of(8)
            || a.get(0, Header::KIND) != CellKind::Roots as u8
            || a.get(0, Roots::MAGIC) != cells::MAGIC
            || a.get(0, Roots::VERSION) != cells::LAYOUT_HASH
        {
            return Err(LoadError::BadHeader);
        }
        Ok(heap)
    }

    fn over(arena: Arena) -> Heap {
        let hard_limit = arena.limit();
        let mut heap = Heap {
            arena,
            gc_threshold: 0,
            gc_pending: false,
            hard_limit,
            reserve: reserve_for(hard_limit),
            #[cfg(test)]
            fail_next_gc_reserve: false,
        };
        heap.apply_reserve();
        heap
    }

    /// Whether the out-of-memory reserve is open (`Roots::FLAGS`, so it survives
    /// snapshots).
    pub fn memory_reserve_open(&self) -> bool {
        self.arena.len() >= Roots::LAYOUT.min_size
            && self.arena.get(0, Roots::FLAGS) & cells::ROOT_RESERVE_OPEN != 0
    }

    /// Sets the arena's cap and the GC threshold from the reserve state.
    fn apply_reserve(&mut self) {
        let limit = if self.memory_reserve_open() {
            self.hard_limit
        } else {
            self.hard_limit - self.reserve
        };
        self.arena.set_limit(limit);
        // A restored image keeps the threshold it had (the same collection points as
        // without the snapshot), unless it is beyond what this cap allows.
        let fresh = gc::next_threshold(self.arena.len(), self.arena.limit());
        let stored = self.stored_gc_threshold();
        let t = if stored == 0 {
            fresh
        } else {
            stored.min(fresh)
        };
        self.set_gc_threshold(t);
        self.gc_pending = self.arena.len() >= self.gc_threshold;
    }

    fn stored_gc_threshold(&self) -> u32 {
        if self.arena.len() >= Roots::LAYOUT.min_size {
            self.arena.get(0, Roots::GC_THRESHOLD)
        } else {
            0
        }
    }

    /// Sets the GC threshold, in the roots cell too (D1601).
    pub(crate) fn set_gc_threshold(&mut self, t: u32) {
        self.gc_threshold = t;
        if self.arena.len() >= Roots::LAYOUT.min_size {
            self.arena.set(0, Roots::GC_THRESHOLD, t);
        }
    }

    /// Opens the out-of-memory reserve (D1600): the interpreter calls this when it turns
    /// an allocation failure into an exception, so that the handlers have room to run.
    /// It closes again in the first collection that leaves twice the reserve free.
    pub fn open_memory_reserve(&mut self) {
        if !self.memory_reserve_open() {
            let f = self.arena.get(0, Roots::FLAGS);
            self.arena
                .set(0, Roots::FLAGS, f | cells::ROOT_RESERVE_OPEN);
            self.apply_reserve();
        }
    }

    /// After a collection: closes the reserve if the live data left room for it.
    fn maybe_close_reserve(&mut self) {
        if self.memory_reserve_open()
            && self.arena.len() <= self.hard_limit.saturating_sub(2 * self.reserve)
        {
            let f = self.arena.get(0, Roots::FLAGS);
            self.arena
                .set(0, Roots::FLAGS, f & !cells::ROOT_RESERVE_OPEN);
            self.arena.set_limit(self.hard_limit - self.reserve);
            let t = gc::next_threshold(self.arena.len(), self.arena.limit());
            self.set_gc_threshold(t);
        }
    }

    /// The arena bytes: a complete, position-independent image of the heap.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        self.arena.bytes()
    }

    #[inline]
    pub fn arena(&self) -> &Arena {
        &self.arena
    }

    #[inline]
    pub fn arena_mut(&mut self) -> &mut Arena {
        &mut self.arena
    }

    /// Allocates a zeroed cell of at least `size` bytes and writes its header.
    pub fn alloc_cell(&mut self, kind: CellKind, size: u32) -> Result<u32, HeapError> {
        let size = align8(size.max(kind.layout().min_size));
        let off = self.arena.alloc(size)?;
        if self.arena.len() >= self.gc_threshold {
            self.gc_pending = true;
        }
        self.arena.set(off, Header::KIND, kind as u8);
        self.arena.set(off, Header::SIZE, size);
        Ok(off)
    }

    /// Like [`Heap::alloc_cell`], with `data` written at offset `at` of the cell instead
    /// of zeros (written once; see [`Arena::alloc_init`]).
    pub fn alloc_cell_init(
        &mut self,
        kind: CellKind,
        size: u32,
        at: u32,
        data: &[u8],
    ) -> Result<u32, HeapError> {
        let size = align8(size.max(kind.layout().min_size));
        let off = self.arena.alloc_init(size, at, data)?;
        if self.arena.len() >= self.gc_threshold {
            self.gc_pending = true;
        }
        self.arena.set(off, Header::KIND, kind as u8);
        self.arena.set(off, Header::SIZE, size);
        Ok(off)
    }

    /// Like [`Heap::alloc_cell_init`] with `n` bytes of data from an iterator (see
    /// [`Arena::alloc_from_iter`]).
    pub fn alloc_cell_from_iter(
        &mut self,
        kind: CellKind,
        size: u32,
        at: u32,
        n: u32,
        data: impl Iterator<Item = u8>,
    ) -> Result<u32, HeapError> {
        let size = align8(size.max(kind.layout().min_size));
        let off = self.arena.alloc_from_iter(size, at, n, data)?;
        if self.arena.len() >= self.gc_threshold {
            self.gc_pending = true;
        }
        self.arena.set(off, Header::KIND, kind as u8);
        self.arena.set(off, Header::SIZE, size);
        Ok(off)
    }

    /// The kind of the cell at `off`. Panics if the header is invalid.
    #[inline]
    pub fn cell_kind(&self, off: u32) -> CellKind {
        let k = self.arena.get(off, Header::KIND);
        CellKind::from_u8(k).unwrap_or_else(|| panic!("invalid cell kind {k} at {off}"))
    }

    #[inline]
    pub fn cell_size(&self, off: u32) -> u32 {
        self.arena.get(off, Header::SIZE)
    }

    #[inline]
    pub fn cell_flags(&self, off: u32) -> u16 {
        self.arena.get(off, Header::FLAGS)
    }

    #[inline]
    pub fn set_cell_flags(&mut self, off: u32, flags: u16) {
        self.arena.set(off, Header::FLAGS, flags)
    }

    /// The realm root value (`undefined` until a realm is created).
    pub fn realm(&self) -> Value {
        self.arena.get(0, Roots::REALM)
    }

    pub fn set_realm(&mut self, v: Value) {
        self.arena.set(0, Roots::REALM, v)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod gc_tests;

#[cfg(test)]
mod append_tests;
