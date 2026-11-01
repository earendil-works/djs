//! Inline caches (0062, D1620): one slot per property or global access site, in the
//! template's `IC` array (a U32Array of three words per site, allocated when the first
//! slot is filled). A slot remembers where the property was found last time: the index of
//! its entry in a property table. A hit re-checks the key at that index (and that it is a
//! data property), so a stale slot is only a miss and caches never change semantics. Slots
//! hold no arena offsets that could outlive a collection, so they need no GC work and
//! images stay deterministic (a VM restored with warm caches behaves like a cold one).
//!
//! Slot layouts (word 0 = 0: empty):
//!
//! - globals: word 0 = entry index + 1, with [`IN_LEX`] set when the entry is in the
//!   global lexical record; word 1 = the realm's `GlobalLexEpoch` when the entry is in
//!   the global object (a newer lexical binding could shadow it).

use super::*;
use crate::bytecode::template::NO_CACHE;
use crate::heap::cells::U32Array;

/// Words per slot.
const WORDS: u32 = 3;

/// Global slot word 0: the entry is in the global lexical record.
const IN_LEX: u32 = 1 << 31;

impl Exec<'_> {
    /// The words of cache slot `ic` of the current function (zeros if empty).
    #[inline(always)]
    fn ic_read(&self, ic: u32) -> [u32; 3] {
        let t: Ref = self.heap.arena().get(self.template, FuncTemplate::IC);
        match t.get() {
            Some(a) if ic != NO_CACHE as u32 => {
                let at = U32Array::DATA.at(a, ic * WORDS);
                let arena = self.heap.arena();
                [arena.read(at), arena.read(at + 4), arena.read(at + 8)]
            }
            _ => [0; 3],
        }
    }

    /// Fills cache slot `ic`, allocating the function's cache array on first use. Out of
    /// memory just leaves the slot empty.
    fn ic_write(&mut self, ic: u32, words: [u32; 3]) {
        if ic == NO_CACHE as u32 {
            return;
        }
        let count = self.heap.arena().get(self.template, FuncTemplate::IC_COUNT) as u32;
        if ic >= count {
            return;
        }
        let a = match self
            .heap
            .arena()
            .get::<Ref>(self.template, FuncTemplate::IC)
            .get()
        {
            Some(a) => a,
            None => {
                let Ok(a) = self.heap.alloc_u32_array(count * WORDS) else {
                    return;
                };
                self.heap
                    .arena_mut()
                    .set(self.template, FuncTemplate::IC, Ref(a));
                a
            }
        };
        for (k, w) in words.into_iter().enumerate() {
            let at = U32Array::DATA.at(a, ic * WORDS + k as u32);
            self.heap.arena_mut().write(at, w);
        }
    }

    fn lex_epoch(&self) -> u32 {
        intrinsic(self.heap, Intrinsic::GlobalLexEpoch)
            .as_int()
            .unwrap_or(0) as u32
    }

    /// A cached global read: the value of data property `key` of the global lexical
    /// record or the global object, if slot `ic` still points at it. An uninitialized
    /// lexical binding is a miss (the slow path throws).
    #[inline(never)]
    pub(super) fn ic_global_get(&self, key: Atom, ic: u32) -> Option<Value> {
        let [w0, w1, _] = self.ic_read(ic);
        if w0 == 0 {
            return None;
        }
        let i = (w0 & !IN_LEX) - 1;
        if w0 & IN_LEX != 0 {
            let v = self.heap.data_at(realm::global_lex(self.heap), i, key)?;
            return (!v.is_uninitialized()).then_some(v);
        }
        if w1 != self.lex_epoch() {
            return None;
        }
        self.heap
            .data_at(intrinsic_obj(self.heap, Intrinsic::Global), i, key)
    }

    /// A cached global store: true if slot `ic` still points at the writable data
    /// property `key` (an initialized lexical binding, or a global object property) and
    /// `v` was written.
    #[inline(never)]
    pub(super) fn ic_global_put(&mut self, key: Atom, ic: u32, v: Value) -> bool {
        let [w0, w1, _] = self.ic_read(ic);
        if w0 == 0 {
            return false;
        }
        let i = (w0 & !IN_LEX) - 1;
        if w0 & IN_LEX != 0 {
            let lex = realm::global_lex(self.heap);
            return match self.heap.data_at(lex, i, key) {
                Some(old) if !old.is_uninitialized() => self.heap.set_data_at(lex, i, key, v),
                _ => false,
            };
        }
        if w1 != self.lex_epoch() {
            return false;
        }
        let g = intrinsic_obj(self.heap, Intrinsic::Global);
        self.heap.set_data_at(g, i, key, v)
    }

    /// After a slow global access found `key`: remembers where (an own data property of
    /// the lexical record or of the global object; anything else is not cached).
    #[inline(never)]
    pub(super) fn ic_global_fill(&mut self, key: Atom, ic: u32) {
        let lex = realm::global_lex(self.heap);
        if let Some(i) = self.heap.own_entry_index(lex, key) {
            if self.heap.data_at(lex, i, key).is_some() {
                self.ic_write(ic, [(i + 1) | IN_LEX, 0, 0]);
            }
            return;
        }
        let g = intrinsic_obj(self.heap, Intrinsic::Global);
        if let Some(i) = self.heap.own_entry_index(g, key)
            && self.heap.data_at(g, i, key).is_some()
        {
            let epoch = self.lex_epoch();
            self.ic_write(ic, [i + 1, epoch, 0]);
        }
    }
}
