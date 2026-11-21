//! Inline caches (0062, D1620, D1621, 0083): one slot per property or global access
//! site, in the template's `IC` array (a U32Array of [`WORDS`] words per site, allocated
//! when the first slot is filled). A slot remembers where the property was found last
//! time. A hit re-checks what the slot relies on (a key at an index, a shape, the
//! prototype epoch), so a stale slot is only a miss and caches never change semantics.
//! Slots that hold arena offsets set [`IC_OFFSET_SLOT`] and are emptied by the collector,
//! so a VM restored with warm caches behaves like a cold one and images stay
//! deterministic.
//!
//! Slot layouts (word 0 = 0: empty). Offset slots carry a kind in bits 24..30 of word 0:
//! 0 an own property of a shape-mode receiver, 1 to 62 a prototype property at that
//! depth, `KIND_ADD` an addition.
//!
//! - globals: word 0 = entry index + 1, with `IN_LEX` set when the entry is in the
//!   global lexical record; word 1 = the realm's `GlobalLexEpoch` when the entry is in
//!   the global object (a newer lexical binding could shadow it).
//! - own property, dictionary-mode receiver: word 0 = entry index + 1 of an own data
//!   property. Only keys that are not array indexes are cached: for those, an own
//!   property is always a named property, whatever the object's class.
//! - own property, shape-mode receiver (0083): word 0 = offset slot, kind 0, slot + 1;
//!   word 1 = the receiver's shape; word 2 = 1 if writable. A hit is a shape compare: the
//!   shape fixes the key and its attributes. With kind `KIND_OWN_ACCESSOR` the slot holds
//!   an accessor (D3470): a read yields its getter.
//! - prototype property reads (D1621): word 0 = offset slot, kind = depth of the holder,
//!   entry index + 1 in the holder; word 1 = the offset of the receiver's prototype (an
//!   object's, or the realm's prototype of a primitive); word 2 = `Roots::PROTO_EPOCH`;
//!   word 3 = the receiver's shape word (`Heap::receiver_shape`, 0083), or 0. The objects
//!   from that prototype to the holder are marked `OBJ_IS_PROTO`, so a key added to one
//!   of them or a prototype change bumps the epoch. The receiver must not have an own
//!   `key`: an equal shape word proves that, otherwise the hit looks it up. The holder's
//!   entry may be an accessor (D3470): the hit then yields the getter to call
//!   ([`Hit::Getter`]).
//! - absent reads (D3470): a prototype-read slot of kind `KIND_ABSENT` (as a depth, more
//!   than any chain a slot is filled for) without an index: no object from the receiver's
//!   prototype (word 1) to the end of the chain has the key, and none is a proxy or a
//!   typed array. Every object of the chain is marked `OBJ_IS_PROTO`, so a key added to
//!   one of them bumps the epoch. The hit walks to the end of the chain, which is what
//!   the depth says, and is `undefined` there.
//! - additions (0083): word 0 = offset slot, kind `KIND_ADD`, `ADD_SET` for a
//!   `put_field` site ([[Set]]) or 0 for `define_field` (CreateDataProperty); word 1 =
//!   the receiver's shape word before, word 2 = the shape after. A [[Set]] addition also
//!   relies on the prototype chain having no `key` (nor a proxy or a typed array): word
//!   3 = the receiver's prototype (0 for none), word 4 = the epoch, with every object of
//!   the chain marked `OBJ_IS_PROTO`.
//!
//! The hit checks are [`cached_get`] and [`cached_put`]: every fast path uses them.

use super::*;
use crate::bytecode::template::NO_CACHE;
use crate::heap::cells::{IC_OFFSET_SLOT, IC_WORDS as WORDS, U32Array};

/// The words of a cache slot.
pub(crate) type Slot = [u32; WORDS as usize];

/// Offset slot word 0: the kind (bits 24..30) and an index + 1 (bits 0..24).
const KIND_SHIFT: u32 = 24;
const INDEX_MASK: u32 = (1 << KIND_SHIFT) - 1;
const KIND_MASK: u32 = 63;
/// The kind of an addition slot; prototype depths stay below it.
const KIND_ADD: u32 = 63;
/// The kind of an own accessor's slot and of an absent read (D3470).
const KIND_OWN_ACCESSOR: u32 = 62;
const KIND_ABSENT: u32 = 61;
const MAX_DEPTH: u32 = KIND_ABSENT - 1;
/// Addition slot word 0: a `put_field` site.
const ADD_SET: u32 = 1;

/// Global slot word 0: the entry is in the global lexical record.
const IN_LEX: u32 = 1 << 31;

#[inline(always)]
fn kind(w0: u32) -> u32 {
    w0 >> KIND_SHIFT & KIND_MASK
}

/// Whether `obj` could have an own property `key` that its named properties don't show,
/// so that a shape compare cannot stand in for a lookup: a typed array's numeric keys, a
/// proxy's traps, an array's `length`, a closure's lazy `length` and `name` (D2445).
#[inline(always)]
fn exotic_own(heap: &Heap, obj: Obj, key: Atom) -> bool {
    match heap.object_class(obj) {
        ObjectClass::Proxy | ObjectClass::TypedArray => true,
        ObjectClass::Array => key == crate::heap::atoms::LENGTH,
        ObjectClass::Function => {
            key == crate::heap::atoms::LENGTH || key == crate::heap::atoms::NAME
        }
        _ => false,
    }
}

/// The prototype a cached prototype read on `o` starts from, if `o` cannot have an own
/// property `key` that the cache would skip; `shape` is the receiver shape word the slot
/// remembers (0: none). `None` for a proxy or a typed array, an object with its own
/// `key`, nullish values, and a string's `length` and indexes.
#[inline(always)]
fn proto_start(heap: &Heap, o: Value, key: Atom, shape: u32) -> Option<Obj> {
    match o.as_object() {
        Some(obj) => {
            if exotic_own(heap, obj, key)
                || ((shape == 0 || heap.receiver_shape(obj) != shape)
                    && heap.has_own_property(obj, key))
            {
                return None;
            }
            heap.get_prototype_of(obj)
        }
        None => {
            if o.as_string().is_some()
                && (key == crate::heap::atoms::LENGTH || key.as_int().is_some())
            {
                return None;
            }
            realm::primitive_proto(heap, o)
        }
    }
}

/// The arena offset of cache slot `ic` of `template`, or 0 when the function has no
/// cache array yet (or the site has no slot): what [`cached_get`] and [`cached_put`]
/// take.
#[inline(always)]
pub(crate) fn slot_at(heap: &Heap, template: u32, ic: u32) -> u32 {
    match heap.arena().get::<Ref>(template, FuncTemplate::IC).get() {
        Some(a) if ic != NO_CACHE as u32 => U32Array::DATA.at(a, ic * WORDS),
        _ => 0,
    }
}

/// Word `k` of the slot at `at` (see [`slot_at`]).
#[inline(always)]
fn word(heap: &Heap, at: u32, k: u32) -> u32 {
    heap.arena().read(at + 4 * k)
}

/// The first three words of the slot at `at`, with one bounds check: every hit needs
/// them (words 3 and 4 only prototype reads and [[Set]] additions).
#[inline(always)]
fn head(heap: &Heap, at: u32) -> [u32; 3] {
    let b = heap.arena().slice(at, 12);
    let w = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    [w(0), w(4), w(8)]
}

/// A get site's cache hit: the value of `o[key]` if the slot at `at` (0: none) still
/// applies, else `None`.
#[inline(always)]
pub(crate) fn cached_get(heap: &Heap, at: u32, o: Value, key: Atom) -> Option<Hit> {
    if at == 0 {
        return None;
    }
    let [w0, w1, w2] = head(heap, at);
    if w0 & IC_OFFSET_SLOT == 0 {
        // Empty, or a verified index into a dictionary.
        return match (w0, o.as_object()) {
            (0, _) | (_, None) => None,
            (_, Some(obj)) => heap.data_at(obj, w0 - 1, key).map(Hit::Value),
        };
    }
    match kind(w0) {
        0 => heap
            .shaped_get(o.as_object()?, w1, (w0 & INDEX_MASK) - 1)
            .map(Hit::Value),
        KIND_ADD => None,
        KIND_OWN_ACCESSOR => {
            let obj = o.as_object()?;
            let i = (w0 & INDEX_MASK) - 1;
            heap.shaped_get(obj, w1, i)?;
            accessor_at(heap, obj, i, key)
        }
        depth => {
            if w2 != heap.proto_epoch() {
                return None;
            }
            let proto = proto_start(heap, o, key, word(heap, at, 3))?;
            if proto.offset() != w1 {
                return None;
            }
            let mut h = proto;
            for _ in 1..depth {
                let Some(p) = heap.get_prototype_of(h) else {
                    // The end of the chain: where an absent read's depth leads.
                    return (depth == KIND_ABSENT).then_some(Hit::Value(Value::UNDEFINED));
                };
                h = p;
            }
            if depth == KIND_ABSENT {
                return None;
            }
            let i = (w0 & INDEX_MASK) - 1;
            match heap.data_at(h, i, key) {
                Some(v) => Some(Hit::Value(v)),
                None => accessor_at(heap, h, i, key),
            }
        }
    }
}

/// [`cached_get`] for the tier-0 loop, read from its arena slice `b`: the two hits a loop
/// meets most, an own property of a shape-mode receiver and a prototype property whose
/// receiver has the remembered shape word (or is a string or a number). `None` for
/// everything else, which the caller then asks [`cached_get`] about (a dictionary-mode
/// receiver, a receiver with another shape, the other primitives, `length`).
///
/// The same checks in the same order as [`cached_get`], [`proto_start`] and
/// `Heap::data_at`, on plain loads: through `&Heap` every access loads the arena's
/// pointer again, the slot's words are a checked slice and the result comes back through
/// memory, which made an own-property hit 132 machine instructions in V8's code and a
/// prototype hit over 200 (D3393). Reads only. Debug builds compare every hit with
/// [`cached_get`] (hot.rs).
#[inline(always)]
pub(super) fn hot_get(b: &[u8], template: u32, ic: u32, o: Value, key: Atom) -> Option<Hit> {
    use crate::heap::cells::flags::OBJ_DICTIONARY;
    use crate::heap::cells::{Header, Object, PropValues};
    use crate::heap::layout::arena_read;
    use crate::heap::object::EMPTY_SHAPE;
    let r32 = |off: u32| u32::from_le_bytes(arena_read(b, off as usize));
    let dictionary = |obj: u32| {
        u16::from_le_bytes(arena_read(b, (obj + Header::FLAGS.offset) as usize)) & OBJ_DICTIONARY
            != 0
    };
    let a = r32(template + FuncTemplate::IC.offset);
    if a == 0 || ic == NO_CACHE as u32 {
        return None;
    }
    let at = U32Array::DATA.at(a, ic * WORDS);
    let w0 = r32(at);
    if w0 & IC_OFFSET_SLOT == 0 {
        return None;
    }
    // (An absent read has no index: 0 here, which no object has as a slot.)
    let index = (w0 & INDEX_MASK).wrapping_sub(1);
    let depth = kind(w0);
    // The object whose slot `index` holds the property: the receiver (an accessor of its
    // own), or the holder on its prototype chain.
    let h = if depth == 0 {
        // `Heap::shaped_get`.
        let obj = o.as_object()?.offset();
        let v = r32(obj + Object::PROPS.offset);
        if dictionary(obj) || v == 0 || r32(v + PropValues::SHAPE.offset) != r32(at + 4) {
            return None;
        }
        return Some(Hit::Value(super::hot::ld(
            b,
            PropValues::VALUES.at(v, index),
        )));
    } else if depth >= KIND_OWN_ACCESSOR {
        // The receiver's own accessor (an addition slot is no read).
        let obj = o.as_object()?.offset();
        let v = r32(obj + Object::PROPS.offset);
        if depth != KIND_OWN_ACCESSOR
            || dictionary(obj)
            || v == 0
            || r32(v + PropValues::SHAPE.offset) != r32(at + 4)
        {
            return None;
        }
        obj
    } else {
        if r32(at + 8) != r32(Roots::PROTO_EPOCH.offset) {
            return None;
        }
        // `proto_start`, without its own-property lookup: the receiver's shape word must be
        // the remembered one.
        let proto = match o.as_object() {
            Some(obj) => {
                let obj = obj.offset();
                let class = r32(obj + Object::CLASS.offset);
                let length = key == crate::heap::atoms::LENGTH;
                if class == ObjectClass::Proxy as u32
                    || class == ObjectClass::TypedArray as u32
                    || (class == ObjectClass::Array as u32 && length)
                    || (class == ObjectClass::Function as u32
                        && (length || key == crate::heap::atoms::NAME))
                {
                    return None;
                }
                let shape = if dictionary(obj) {
                    0
                } else {
                    match r32(obj + Object::PROPS.offset) {
                        0 => EMPTY_SHAPE,
                        v => r32(v + PropValues::SHAPE.offset),
                    }
                };
                if shape == 0 || shape != r32(at + 12) {
                    return None;
                }
                r32(obj + Object::PROTO.offset)
            }
            None => {
                let i = if o.as_string().is_some() {
                    if key == crate::heap::atoms::LENGTH || key.as_int().is_some() {
                        return None;
                    }
                    Intrinsic::StringProto
                } else if o.is_number() {
                    Intrinsic::NumberProto
                } else {
                    return None;
                };
                let realm = super::hot::ld(b, Roots::REALM.offset).as_internal()?;
                let p =
                    super::hot::ld(b, crate::heap::cells::Realm::INTRINSICS.at(realm, i as u32));
                p.as_object()?.offset()
            }
        };
        if proto == 0 || proto != r32(at + 4) {
            return None;
        }
        let mut h = proto;
        for _ in 1..depth {
            h = r32(h + Object::PROTO.offset);
            if h == 0 {
                // The end of the chain: where an absent read's depth leads.
                return (depth == KIND_ABSENT).then_some(Hit::Value(Value::UNDEFINED));
            }
        }
        h
    };
    hot_data_at(b, h, index, key)
}

/// Slot `i` of `h` if it is the accessor `key` (D3470), read as the loop reads it: the
/// getter, or undefined for an accessor without one. For a slot `Heap::data_at` refused.
/// Out of line: [`cached_get`]'s two uses share it.
#[inline(never)]
fn accessor_at(heap: &Heap, h: Obj, i: u32, key: Atom) -> Option<Hit> {
    hot_data_at(heap.arena().bytes(), h.offset(), i, key)
}

/// What [`hot_get`] found.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Hit {
    /// The property's value.
    Value(Value),
    /// An accessor on the prototype chain: its getter, to call with the receiver.
    Getter(Value),
}

/// `Heap::data_at` on the tier-0 loop's slice: the value of slot `index` of the object at
/// `h` if that slot is the data property `key`, or the getter if it is the accessor `key`
/// (an accessor without a getter reads as undefined).
#[inline(always)]
fn hot_data_at(b: &[u8], h: u32, index: u32, key: Atom) -> Option<Hit> {
    use crate::heap::cells::flags::OBJ_DICTIONARY;
    use crate::heap::cells::{Header, Object, PropEntry, PropTable, PropValues, Shape};
    use crate::heap::layout::arena_read;
    use crate::heap::object::STORE_ACCESSOR;
    let r32 = |off: u32| u32::from_le_bytes(arena_read(b, off as usize));
    let flags: u16 = u16::from_le_bytes(arena_read(b, (h + Header::FLAGS.offset) as usize));
    let p = r32(h + Object::PROPS.offset);
    if p == 0 {
        return None;
    }
    let (k, fl, value) = if flags & OBJ_DICTIONARY != 0 {
        if index >= r32(p + PropTable::COUNT.offset) {
            return None;
        }
        let e = PropTable::ENTRIES.at(p, index);
        (
            r32(e + PropEntry::KEY.offset),
            r32(e + PropEntry::FLAGS.offset),
            e + PropEntry::VALUE.offset,
        )
    } else {
        if index >= r32(p + PropValues::LEN.offset) {
            return None;
        }
        let d = r32(r32(p + PropValues::SHAPE.offset) + Shape::DESCS.offset);
        let e = U32Array::DATA.at(d, 1 + 2 * index);
        (r32(e), r32(e + 4), PropValues::VALUES.at(p, index))
    };
    if k != key.id() {
        return None;
    }
    let v = super::hot::ld(b, value);
    if fl & STORE_ACCESSOR == 0 {
        return Some(Hit::Value(v));
    }
    let getter = super::hot::ld(
        b,
        v.as_internal()? + crate::heap::cells::AccessorPair::GETTER.offset,
    );
    Some(if getter.is_undefined() {
        Hit::Value(getter)
    } else {
        Hit::Getter(getter)
    })
}

/// A put or define site's cache hit: true if the slot at `at` (0: none) still applies to
/// `obj[key] = v` and `v` was stored (an own writable data property, or a cached
/// addition).
#[inline(always)]
pub(crate) fn cached_put(heap: &mut Heap, at: u32, obj: Obj, key: Atom, v: Value) -> bool {
    if at == 0 {
        return false;
    }
    let [w0, w1, w2] = head(heap, at);
    if w0 & IC_OFFSET_SLOT == 0 {
        return w0 != 0 && heap.set_data_at(obj, w0 - 1, key, v);
    }
    match kind(w0) {
        0 => w2 & 1 != 0 && heap.shaped_set(obj, w1, (w0 & INDEX_MASK) - 1, v),
        KIND_ADD => cached_add(heap, at, w0, obj, v),
        _ => false,
    }
}

/// A `define_field` site's cache hit (its slots are additions without [`ADD_SET`]).
#[inline(always)]
fn cached_define(heap: &mut Heap, at: u32, obj: Obj, v: Value) -> bool {
    let w0 = word(heap, at, 0);
    w0 & (IC_OFFSET_SLOT | KIND_MASK << KIND_SHIFT) == IC_OFFSET_SLOT | KIND_ADD << KIND_SHIFT
        && cached_add(heap, at, w0, obj, v)
}

/// The addition half of [`cached_put`], out of line: it may allocate, and keeping it out
/// of the hot own-property path keeps that path small.
#[inline(never)]
fn cached_add(heap: &mut Heap, at: u32, w0: u32, obj: Obj, v: Value) -> bool {
    if w0 & ADD_SET != 0
        && (word(heap, at, 4) != heap.proto_epoch()
            || heap.get_prototype_of(obj).map_or(0, |p| p.offset()) != word(heap, at, 3))
    {
        return false;
    }
    let (old, new) = (word(heap, at, 1), word(heap, at, 2));
    heap.shaped_add(obj, old, new, v)
}

impl Exec<'_> {
    /// The first words of cache slot `ic` of the current function (zeros if empty), for
    /// the global caches.
    #[inline(always)]
    fn ic_read(&self, ic: u32) -> [u32; 2] {
        match slot_at(self.heap, self.template, ic) {
            0 => [0; 2],
            at => [word(self.heap, at, 0), word(self.heap, at, 1)],
        }
    }

    /// Fills cache slot `ic`, allocating the function's cache array on first use. Out of
    /// memory just leaves the slot empty. (Out of line: seven fills call it.)
    #[inline(never)]
    fn ic_write(&mut self, ic: u32, words: Slot) {
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

    /// `Heap::own_entry_index`, out of line: the fills are cold and five of them ask.
    #[inline(never)]
    fn entry_index(&self, o: Obj, key: Atom) -> Option<u32> {
        self.heap.own_entry_index(o, key)
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
        let [w0, w1] = self.ic_read(ic);
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
        let [w0, w1] = self.ic_read(ic);
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
        if let Some(i) = self.entry_index(lex, key) {
            if self.heap.data_at(lex, i, key).is_some() {
                self.ic_write(ic, [(i + 1) | IN_LEX, 0, 0, 0, 0]);
            }
            return;
        }
        let g = intrinsic_obj(self.heap, Intrinsic::Global);
        if let Some(i) = self.entry_index(g, key)
            && self.heap.data_at(g, i, key).is_some()
        {
            let epoch = self.lex_epoch();
            self.ic_write(ic, [i + 1, epoch, 0, 0, 0]);
        }
    }

    /// A cached read: `o[key]` if slot `ic` applies.
    #[inline(always)]
    pub(super) fn ic_get(&self, o: Value, key: Atom, ic: u32) -> Option<Hit> {
        cached_get(self.heap, slot_at(self.heap, self.template, ic), o, key)
    }

    /// A cached `define_field` (CreateDataProperty of a new key): true if slot `ic`
    /// applies and `v` was stored. A define site only ever holds an addition
    /// (`ic_add_fill` with `set` false), so this skips `cached_put`'s other kinds.
    #[inline(always)]
    pub(super) fn ic_define(&mut self, obj: Obj, ic: u32, v: Value) -> bool {
        let at = slot_at(self.heap, self.template, ic);
        at != 0 && cached_define(self.heap, at, obj, v)
    }

    /// A cached store: true if slot `ic` applies to `obj[key] = v` and `v` was stored.
    #[inline(always)]
    pub(super) fn ic_put(&mut self, obj: Obj, key: Atom, ic: u32, v: Value) -> bool {
        let at = slot_at(self.heap, self.template, ic);
        cached_put(self.heap, at, obj, key, v)
    }

    /// After a slow lookup found data property `key` on `holder`, on the prototype chain
    /// of `o`: remembers it (D1621), with the receiver's shape word (0083).
    /// With `accessor`, the holder's `key` is an accessor (D3470).
    #[inline(never)]
    pub(super) fn ic_proto_fill(
        &mut self,
        o: Value,
        key: Atom,
        holder: Obj,
        accessor: bool,
        ic: u32,
    ) {
        if key.as_int().is_some() {
            return;
        }
        let Some(proto) = proto_start(self.heap, o, key, 0) else {
            return;
        };
        let mut depth = 1;
        let mut h = proto;
        while h != holder {
            match self.heap.get_prototype_of(h) {
                Some(p) if depth < MAX_DEPTH => {
                    h = p;
                    depth += 1;
                }
                _ => return,
            }
        }
        let Some(i) = self.entry_index(holder, key) else {
            return;
        };
        if i + 1 > INDEX_MASK || (!accessor && self.heap.data_at(holder, i, key).is_none()) {
            return;
        }
        let mut h = proto;
        loop {
            self.heap.mark_proto(h);
            if h == holder {
                break;
            }
            h = self.heap.get_prototype_of(h).expect("walked above");
        }
        let w0 = IC_OFFSET_SLOT | depth << KIND_SHIFT | (i + 1);
        let epoch = self.heap.proto_epoch();
        let shape = o.as_object().map_or(0, |obj| self.heap.receiver_shape(obj));
        self.ic_write(ic, [w0, proto.offset(), epoch, shape, 0]);
    }

    /// After a slow lookup found `key` neither on `obj` nor on its prototype chain:
    /// remembers that (D3470), for a shape-mode receiver with a prototype whose chain has
    /// no proxy and no typed array. The chain is marked `OBJ_IS_PROTO`, so that a key
    /// added to it bumps the epoch; the receiver's shape word stands for its own keys.
    #[inline(never)]
    pub(super) fn ic_absent_fill(&mut self, obj: Obj, key: Atom, ic: u32) {
        if key.as_int().is_some()
            || exotic_own(self.heap, obj, key)
            || self.heap.canonical_numeric_index(key).is_some()
        {
            return;
        }
        let shape = self.heap.receiver_shape(obj);
        let Some(proto) = self.heap.get_prototype_of(obj) else {
            return;
        };
        if shape == 0 || !self.chain_lacks_marked(Some(proto), key) {
            return;
        }
        let w0 = IC_OFFSET_SLOT | KIND_ABSENT << KIND_SHIFT;
        let epoch = self.heap.proto_epoch();
        self.ic_write(ic, [w0, proto.offset(), epoch, shape, 0]);
    }

    /// After a slow lookup found the accessor `key` on `obj` itself, a shape-mode object:
    /// remembers its slot (D3470).
    #[inline(never)]
    pub(super) fn ic_own_accessor_fill(&mut self, obj: Obj, key: Atom, ic: u32) {
        let shape = self.heap.receiver_shape(obj);
        if key.as_int().is_some()
            || exotic_own(self.heap, obj, key)
            || shape == 0
            || shape == crate::heap::object::EMPTY_SHAPE
        {
            return;
        }
        if let Some(slot) = self.entry_index(obj, key)
            && slot < INDEX_MASK
        {
            let w0 = IC_OFFSET_SLOT | KIND_OWN_ACCESSOR << KIND_SHIFT | (slot + 1);
            self.ic_write(ic, [w0, shape, 0, 0, 0]);
        }
    }

    /// After a slow access to `obj[key]`: remembers the property if `key` is an own data
    /// property of `obj` (and not an array index).
    #[inline(never)]
    pub(super) fn ic_own_fill(&mut self, obj: Obj, key: Atom, ic: u32) {
        if key.as_int().is_some() {
            return;
        }
        if let Some((shape, slot, writable)) = self.heap.shape_slot(obj, key) {
            if slot < INDEX_MASK {
                let w0 = IC_OFFSET_SLOT | (slot + 1);
                self.ic_write(ic, [w0, shape, writable as u32, 0, 0]);
            }
            return;
        }
        if let Some(i) = self.entry_index(obj, key)
            && self.heap.data_at(obj, i, key).is_some()
        {
            self.ic_write(ic, [i + 1, 0, 0, 0, 0]);
        }
    }

    /// Whether no object from `proto` up has `key`, a proxy or a typed array (which
    /// could intercept it).
    fn chain_lacks(&self, proto: Option<Obj>, key: Atom) -> bool {
        let mut p = proto;
        while let Some(q) = p {
            if matches!(
                self.heap.object_class(q),
                ObjectClass::Proxy | ObjectClass::TypedArray
            ) || self.heap.has_own_property(q, key)
            {
                return false;
            }
            p = self.heap.get_prototype_of(q);
        }
        true
    }

    /// [`Self::chain_lacks`], and if so every object of the chain marked `OBJ_IS_PROTO`:
    /// a key added to one of them then bumps the epoch (what an addition slot and an
    /// absent-read slot rely on).
    fn chain_lacks_marked(&mut self, proto: Option<Obj>, key: Atom) -> bool {
        if !self.chain_lacks(proto, key) {
            return false;
        }
        let mut p = proto;
        while let Some(q) = p {
            self.heap.mark_proto(q);
            p = self.heap.get_prototype_of(q);
        }
        true
    }

    /// Whether `obj[key] = v` ([[Set]] with `obj` as the receiver) is just
    /// CreateDataProperty: `obj` is extensible and has no own `key`, nothing on its
    /// prototype chain has `key` or could intercept it, and `key` is not one an exotic
    /// object handles itself. The case an addition cache repeats (0083).
    pub(super) fn plain_addition(&self, obj: Obj, key: Atom) -> bool {
        key.as_int().is_none()
            && !exotic_own(self.heap, obj, key)
            && self.heap.canonical_numeric_index(key).is_none()
            && self.heap.is_extensible(obj)
            && !self.heap.has_own_property(obj, key)
            && self.chain_lacks(self.heap.get_prototype_of(obj), key)
    }

    /// After a slow `put_field` (`set`) or `define_field` that may have added `key` to
    /// `obj`, whose receiver shape word was `old` before: caches the addition (0083). A
    /// [[Set]] addition is only cached when no object of the prototype chain has `key`
    /// and none is a proxy or a typed array; the chain is then marked `OBJ_IS_PROTO`, so
    /// that a change to it bumps the epoch.
    #[inline(never)]
    pub(super) fn ic_add_fill(&mut self, obj: Obj, key: Atom, old: u32, set: bool, ic: u32) {
        if key.as_int().is_some()
            || exotic_own(self.heap, obj, key)
            || self.heap.canonical_numeric_index(key).is_some()
        {
            return;
        }
        let Some(new) = self.heap.added_shape(obj, key, old) else {
            return;
        };
        if !set {
            let w0 = IC_OFFSET_SLOT | KIND_ADD << KIND_SHIFT;
            self.ic_write(ic, [w0, old, new, 0, 0]);
            return;
        }
        let proto = self.heap.get_prototype_of(obj);
        if !self.chain_lacks_marked(proto, key) {
            return;
        }
        let w0 = IC_OFFSET_SLOT | KIND_ADD << KIND_SHIFT | ADD_SET;
        let epoch = self.heap.proto_epoch();
        let p = proto.map_or(0, |p| p.offset());
        self.ic_write(ic, [w0, old, new, p, epoch]);
    }

    /// `fast_step` for `define_field` (object literals) when the site's addition cache
    /// applies (0083).
    #[cfg(test)]
    #[inline(never)]
    pub(super) fn fast_define(&mut self, sp0: u32) -> bool {
        let ic = self.imm_u16(5);
        let (o, v) = (self.top(1), self.top(0));
        let Some(obj) = o.as_object() else {
            return false;
        };
        if !self.ic_define(obj, ic, v) {
            return false;
        }
        // `obj v -> obj`
        self.sp -= 1;
        self.advance(Op::DefineField, sp0);
        true
    }

    /// `fast_step` for `get_field`, `get_method` and `put_field` when the site's cache
    /// applies; false, with nothing changed, otherwise.
    #[cfg(test)]
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
            if !self.ic_put(obj, key, ic, v) {
                return false;
            }
            self.sp -= 2;
            self.advance(Op::PutField, sp0);
            return true;
        }
        let o = self.top(0);
        if key == crate::heap::atoms::LENGTH && op == fast::GET_FIELD {
            // `a.length` of an Array (a non-configurable own data property, its slot 0)
            // and of a string primitive have no table entry for the IC (0086).
            let len = match (o.as_object(), o.as_string()) {
                (Some(a), _) if self.heap.object_class(a) == ObjectClass::Array => {
                    Some(self.heap.object_slot(a, 0))
                }
                (_, Some(s)) => i32::try_from(self.heap.str_len(s)).ok().map(Value::int),
                _ => None,
            };
            if let Some(v) = len {
                self.set(self.sp - 1, v);
                self.advance(Op::GetField, sp0);
                return true;
            }
        }
        let Some(Hit::Value(v)) = self.ic_get(o, key, ic) else {
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
