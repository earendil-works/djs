//! The frozen-builtins mode (D022, D2304): with `Roots::FLAGS` bit
//! [`ROOT_FROZEN_BUILTINS`](super::cells::ROOT_FROZEN_BUILTINS) set, every object of the
//! builtin prefix (below `Roots::BASE_END`) behaves as frozen without being frozen: it is
//! not extensible, its own properties read as non-writable and non-configurable, and the
//! property operations refuse to change it. Nothing in the objects changes, so one base
//! image serves both modes. Internal state the engine keeps in those cells (header bits,
//! shape counts, realm slots) is written as before: only the JS-visible property
//! operations of `object.rs` consult [`Heap::frozen`].
//!
//! Exempt: the per-VM objects in the prefix (until 0147 moves them above it): the global
//! object, the global lexical record and the hidden registries of `FROZEN_EXEMPT` in
//! interp/realm.rs.

use super::Heap;
use super::cells::{self, Realm, Roots};
use crate::value::{Obj, Value};

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
        self.frozen_end = if self.frozen_builtins() {
            self.arena.get(0, Roots::BASE_END)
        } else {
            0
        };
    }

    /// Whether `o` is a frozen builtin: in the prefix with the mode on, and not exempt.
    #[inline(always)]
    pub fn frozen(&self, o: Obj) -> bool {
        o.0 < self.frozen_end && self.frozen_in_prefix(o)
    }

    /// [`Heap::frozen`] for an object below the boundary: false for the exempt per-VM
    /// objects.
    #[inline(never)]
    fn frozen_in_prefix(&self, o: Obj) -> bool {
        debug_assert_eq!(self.frozen_end, self.arena.get(0, Roots::BASE_END));
        let Some(r) = self.realm().as_internal() else {
            return true;
        };
        let lex: super::Ref = self.arena.get(r, Realm::GLOBAL_LEX);
        o.0 != lex.0
            && crate::interp::realm::FROZEN_EXEMPT.iter().all(|&i| {
                let v: Value = self.arena.read(Realm::INTRINSICS.at(r, i as u32));
                v.as_object() != Some(o)
            })
    }
}
