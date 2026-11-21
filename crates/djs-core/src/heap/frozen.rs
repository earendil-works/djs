//! The frozen-builtins mode (D022, D2304): with `Roots::FLAGS` bit
//! [`ROOT_FROZEN_BUILTINS`](super::cells::ROOT_FROZEN_BUILTINS) set, every object of the
//! builtin prefix (below `Roots::BASE_END`) behaves as frozen without being frozen: it is
//! not extensible, its own properties read as non-writable and non-configurable, and the
//! property operations refuse to change it. Nothing in the objects changes, so one base
//! image serves both modes. Internal state the engine keeps in those cells (header bits,
//! shape counts, realm slots) is written as before: only the JS-visible property
//! operations of `object.rs` consult [`Heap::frozen`].
//!
//! No object is exempt: the per-VM objects (the global object, the global lexical record,
//! `Symbol.for`'s registry) live above the prefix (D2063). With the mode on, the prefix
//! write barrier (D2065) is armed in debug builds.

use super::Heap;
use super::cells::{self, Roots};
use crate::value::Obj;

impl Heap {
    /// Whether the frozen-builtins mode is on (part of the image).
    pub fn frozen_builtins(&self) -> bool {
        self.arena.len() >= Roots::LAYOUT.min_size
            && self.arena.get(0, Roots::FLAGS) & cells::ROOT_FROZEN_BUILTINS != 0
    }

    /// Turns the frozen-builtins mode on or off. It has no effect without a builtin prefix
    /// (`Roots::BASE_END` 0).
    pub fn set_frozen_builtins(&mut self, on: bool) {
        let f = self.arena.get(0, Roots::FLAGS);
        let f = if on {
            f | cells::ROOT_FROZEN_BUILTINS
        } else {
            f & !cells::ROOT_FROZEN_BUILTINS
        };
        self.arena.set(0, Roots::FLAGS, f);
        self.refresh_frozen();
    }

    /// Recomputes the cached boundary after anything that can move `Roots::BASE_END` or
    /// the flag: loading, collections, truncation.
    pub(crate) fn refresh_frozen(&mut self) {
        let (end, f) = if self.arena.len() >= Roots::LAYOUT.min_size {
            (
                self.arena.get(0, Roots::BASE_END),
                self.arena.get(0, Roots::FLAGS),
            )
        } else {
            (0, 0)
        };
        let frozen = f & cells::ROOT_FROZEN_BUILTINS != 0;
        let written = f & cells::ROOT_PREFIX_WRITTEN != 0;
        self.frozen_end = if frozen { end } else { 0 };
        self.gate_end = if frozen || !written { end } else { 0 };
        self.refresh_barrier();
    }

    /// Whether the builtin prefix was written ([`cells::ROOT_PREFIX_WRITTEN`], D3290).
    pub fn prefix_written(&self) -> bool {
        self.arena.len() >= Roots::LAYOUT.min_size
            && self.arena.get(0, Roots::FLAGS) & cells::ROOT_PREFIX_WRITTEN != 0
    }

    /// Whether the builtins are as the base image has them (D3290): there is a builtin
    /// prefix and the [`cells::ROOT_PREFIX_WRITTEN`] latch is not set (with frozen builtins
    /// the property operations cannot set it).
    #[inline(always)]
    pub fn builtins_pristine(&self) -> bool {
        // (`gate_end` is 0 without a prefix, and latched unless the builtins are frozen.)
        self.gate_end != 0 && self.arena.get(0, Roots::FLAGS) & cells::ROOT_PREFIX_WRITTEN == 0
    }

    /// The write gate of the JS-visible property operations (D2304, D3290), before they
    /// change `o`: true if the change must be refused (a frozen builtin). A write to
    /// another object in the builtin prefix sets the [`cells::ROOT_PREFIX_WRITTEN`] latch
    /// first: from then on collections walk the prefix for references into the session's
    /// cells.
    #[inline(always)]
    pub(crate) fn refuse_write(&mut self, o: Obj) -> bool {
        o.0 < self.gate_end && self.prefix_write()
    }

    /// The fast writers' gate (D2304, D3290): true if `o` is in the builtin prefix while
    /// writes there are refused or would set the latch. They decline then (with nothing
    /// changed), and the general path's property operations decide in `refuse_write`.
    #[inline(always)]
    pub(crate) fn gated(&self, o: Obj) -> bool {
        o.0 < self.gate_end
    }

    /// `refuse_write`'s out-of-line part: refuses with frozen builtins, else sets the latch.
    #[cold]
    #[inline(never)]
    fn prefix_write(&mut self) -> bool {
        if self.frozen_end != 0 {
            return true;
        }
        self.set_prefix_written();
        false
    }

    /// Sets the [`cells::ROOT_PREFIX_WRITTEN`] latch (D3290): the one place it is set.
    /// Collections walk the builtin prefix from now on, and the builtins no longer count
    /// as pristine.
    pub(crate) fn set_prefix_written(&mut self) {
        let f = self.arena.get(0, Roots::FLAGS);
        self.arena
            .set(0, Roots::FLAGS, f | cells::ROOT_PREFIX_WRITTEN);
        // Frozen builtins keep refusing writes.
        self.gate_end = self.frozen_end;
        // The known builtins are no longer the base's (D3300).
        crate::interp::known::unpristine(self);
    }

    /// For writers that checked [`Heap::frozen`] already (a frozen builtin was refused):
    /// sets the latch if `o` is in the prefix.
    #[inline(always)]
    pub(crate) fn note_write(&mut self, o: Obj) {
        if o.0 < self.gate_end {
            self.prefix_write();
        }
    }

    /// Whether `o` is a frozen builtin: in the prefix with the mode on. The per-VM
    /// objects (the global object, the global lexical record, the `Symbol.for` registry)
    /// are above the prefix (D2063), so there are no exemptions.
    #[inline(always)]
    pub fn frozen(&self, o: Obj) -> bool {
        o.0 < self.frozen_end
    }
}
