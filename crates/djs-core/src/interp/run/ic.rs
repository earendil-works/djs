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
//! - property reads and writes: word 0 = entry index + 1 of an own data property of
//!   the receiver. Only keys that are not array indexes are cached: for those, an own
//!   property is always a table entry, whatever the object's class.

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

    /// A cached own data property read: `obj[key]` if slot `ic` still points at it.
    #[inline(always)]
    pub(super) fn ic_own_get(&self, obj: Obj, key: Atom, ic: u32) -> Option<Value> {
        let w0 = self.ic_read(ic)[0];
        if w0 == 0 {
            return None;
        }
        self.heap.data_at(obj, w0 - 1, key)
    }

    /// A cached own data property write: true if slot `ic` still points at a writable
    /// data property `key` of `obj` and `v` was written ([[Set]] on such a property only
    /// replaces the value).
    #[inline(always)]
    pub(super) fn ic_own_put(&mut self, obj: Obj, key: Atom, ic: u32, v: Value) -> bool {
        let w0 = self.ic_read(ic)[0];
        w0 != 0 && self.heap.set_data_at(obj, w0 - 1, key, v)
    }

    /// After a slow access to `obj[key]`: remembers the entry if `key` is an own data
    /// property of `obj` (and not an array index).
    #[inline(never)]
    pub(super) fn ic_own_fill(&mut self, obj: Obj, key: Atom, ic: u32) {
        if key.as_int().is_some() {
            return;
        }
        if let Some(i) = self.heap.own_entry_index(obj, key)
            && self.heap.data_at(obj, i, key).is_some()
        {
            self.ic_write(ic, [i + 1, 0, 0]);
        }
    }

    /// `fast_step` for `get_field`, `get_method` and `put_field` on an own data property
    /// the site's cache points at; false, with nothing changed, otherwise.
    #[inline(always)]
    pub(super) fn fast_field(&mut self, sp0: u32) -> bool {
        let op = self.byte(self.pc);
        let key = Atom::from_id(self.imm_u32(1));
        let ic = self.imm_u16(5);
        if op == fast::PUT_FIELD {
            let (o, v) = (self.top(1), self.top(0));
            let Some(obj) = o.as_object() else {
                return false;
            };
            if !self.ic_own_put(obj, key, ic, v) {
                return false;
            }
            self.sp -= 2;
            self.advance(Op::PutField, sp0);
            return true;
        }
        let o = self.top(0);
        let Some(obj) = o.as_object() else {
            return false;
        };
        let Some(v) = self.ic_own_get(obj, key, ic) else {
            return false;
        };
        if op == fast::GET_FIELD {
            self.set(self.sp - 1, v);
            self.advance(Op::GetField, sp0);
        } else {
            self.set(self.sp - 1, v);
            self.push(o);
            self.advance(Op::GetMethod, sp0);
        }
        true
    }
}
