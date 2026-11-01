//! Objects: properties, dense elements, prototypes. All operations here are leaf: they never
//! run user code. Getters and setters are returned to the caller, which decides how to call
//! them.
//!
//! # Storage
//!
//! An [`Object`] cell has a class, a prototype, a property table ref and an elements ref.
//!
//! - The [`PropTable`] keeps entries in insertion order. Up to [`LINEAR_MAX`] entries it is
//!   searched linearly; above that a hash index over the entries is built (like a
//!   compact dict). Entry keys are atoms, so lookups compare `u32`s.
//! - Dense elements (arrays only, for now) are a [`ValueArray`] whose `LEN` is the dense
//!   length. A dense slot holds a data property with all of writable, enumerable and
//!   configurable set, or a hole.
//!
//! Invariant: an array-index key lives in exactly one place. If its dense slot is not a
//! hole, the property table has no entry for it. Index keys in the table (sparse elements,
//! elements with non-default attributes, index keys of non-arrays) set the
//! [`OBJ_INDEXED_PROPS`] flag, so objects without it never search the table for an index.
//!
//! Classes with exotic integer-keyed behavior (String wrappers, typed arrays, proxies)
//! must also set [`OBJ_INDEXED_PROPS`]: the element fast paths rely on it to know
//! that a prototype chain cannot intercept an index.
//!
//! An array's `length` is internal slot 0 (a Number), not a table entry. It is reported as
//! an own data property `{writable: !OBJ_LENGTH_READONLY, enumerable: false,
//! configurable: false}`.

use alloc::vec::Vec;

use super::atom::{Atom, atoms};
use super::cells::{
    AccessorPair, Object, PropEntry, PropTable, Roots, ValueArray,
    flags::{OBJ_EXTENSIBLE, OBJ_INDEXED_PROPS, OBJ_IS_PROTO, OBJ_LENGTH_READONLY},
};
use super::layout::Ref;
use super::{CellKind, Heap, HeapError};
use crate::value::{Obj, Value};

/// Tables with at most this many entries are searched linearly.
pub const LINEAR_MAX: u32 = 8;

/// A write may extend the dense elements by at most this many holes; writes further out
/// go to the property table (sparse).
pub const DENSE_GAP: u32 = 64;

/// Bucket value for a deleted entry in a property hash index.
const TOMBSTONE: u32 = u32::MAX;

/// Storage-only flag bit: the entry is an accessor.
pub(crate) const STORE_ACCESSOR: u32 = 8;

/// The object class, stored in [`Object::CLASS`]. Issue 0006 adds more.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum ObjectClass {
    Ordinary = 0,
    /// An Array exotic object: dense elements and a `length` in slot 0.
    Array = 1,
    /// A JS closure: slot 0 is an internal reference to its `FuncTemplate`, slots 1.. the
    /// internal references to its captured Boxes.
    Function = 2,
    /// A function implemented by the embedder: slot 0 is its host function id (int32).
    HostFunction = 3,
    /// An Error instance (`[[ErrorData]]`).
    Error = 4,
    /// An unmapped (strict) arguments object: ordinary storage, its own class for
    /// `Object.prototype.toString`.
    Arguments = 5,
    /// An Array Iterator: slot 0 the target (undefined once exhausted), slot 1 the next
    /// index, slot 2 the kind (0 keys, 1 values, 2 entries).
    ArrayIterator = 6,
    /// A bound function exotic object: slot 0 the target function, slot 1 the bound
    /// `this`, slot 2 an Array of the bound arguments.
    BoundFunction = 7,
    /// Primitive wrapper objects (`[[BooleanData]]`, `[[NumberData]]`,
    /// `[[StringData]]`, `[[SymbolData]]`): slot 0 is the primitive. A String object's
    /// `length` and index properties are ordinary own properties defined at creation.
    Boolean = 8,
    Number = 9,
    String = 10,
    Symbol = 11,
    /// Map, Set, WeakMap, WeakSet (0038, `heap/collection.rs`): slot 0 the entries (a
    /// ValueArray, or an EphemeronArray for the weak ones), slot 1 the hash index (a
    /// U32Array), slot 2 the number of live entries.
    Map = 12,
    Set = 13,
    WeakMap = 14,
    WeakSet = 15,
    /// A Map or Set Iterator: slot 0 the entries it walks (undefined once done), slot 1
    /// the next entry number, slot 2 the kind (0 keys, 1 values, 2 entries), slot 3 0
    /// for a Map, 1 for a Set, slots 4 and 5 the key and value of the last step.
    CollectionIterator = 16,
    /// A Promise (`[[PromiseState]]` ...): slot 0 the state (int32: 0 pending, 1
    /// fulfilled, 2 rejected), slot 1 the result, slot 2 the pending reactions (an
    /// internal `ValueArray` of `[capability, onFulfilled, onRejected]` triples, or
    /// undefined), slot 3 flags (int32, `interp::run::promise`). D1371.
    Promise = 17,
    /// A builtin closure (D1372): calling it calls the installed intrinsic builtin whose
    /// id is slot 0 (int32) with `this` = slot 1 (its data) and the call's arguments.
    /// Promise resolving functions, `Promise.all` element functions, ...
    BuiltinClosure = 18,
    /// An internal record (D1372): a prototype-less object with only internal slots,
    /// never handed to user code; the builtins read it with `%RecordGet`/`%RecordSet`.
    Record = 19,
    /// A Date: slot 0 the time value (a Number, NaN for an invalid date).
    Date = 20,
    /// A suspendable function activation (D1375): a generator object, or the internal
    /// state of an async function call. Slot 0 the state (`interp::run::suspend`), slot
    /// 1 the saved frame (an internal ValueArray) while suspended, slot 2 the pc to
    /// resume at, slot 3 the frame's `hp - bp`, slot 4 the result promise (async), slot
    /// 5 the kind (0 generator, 1 async function, 2 async generator), slot 6 an async
    /// generator's request queue (D1421).
    Generator = 21,
    /// An ArrayBuffer, a typed array, a DataView (0039, `heap/typed.rs`).
    ArrayBuffer = 22,
    TypedArray = 23,
    DataView = 24,
    /// A RegExp (0033, `builtins/regexp.rs`): slot 0 the compiled program (an internal
    /// U32Array), slot 1 the source string, slot 2 the flag bits (int32).
    RegExp = 25,
    /// A RegExp String Iterator (matchAll): slot 0 the regexp, slot 1 the string, slot
    /// 2 flags (int32: 1 global, 2 unicode, 4 done).
    RegExpStringIterator = 26,
    /// A Proxy exotic object (0045, `heap/proxy.rs`): slot 0 the target, slot 1 the
    /// handler (both null once revoked), slot 2 flags (int32: 1 callable, 2
    /// constructor, fixed at creation from the target). No prototype, no own
    /// properties, not extensible as far as the heap is concerned: the interpreter and
    /// the builtins send every internal method to the traps (builtins/proxy.ts).
    Proxy = 27,
    /// An iterator helper object (0051, builtins/iterator.ts): the objects of
    /// `Iterator.prototype.map` and friends, `Iterator.concat` and `Iterator.from`'s
    /// wrapper, told apart by slot 0 (the kind). The builtins keep their state in slots
    /// 0..8 with `%RecordGet`/`%RecordSet`.
    IteratorHelper = 28,
    /// A WeakRef and a FinalizationRegistry (0051, `heap/weakref.rs`, D1513).
    WeakRef = 31,
    FinalizationRegistry = 32,
    /// A DisposableStack or AsyncDisposableStack (0051, builtins/disposable.ts): slot 0
    /// true for an AsyncDisposableStack, slot 1 the state (0 pending, 1 disposed), slot 2
    /// the resources (an internal Array of [value, method, kind] triples).
    DisposableStack = 33,
    /// A BigInt wrapper (`[[BigIntData]]`, 0046): slot 0 is the BigInt, like the other
    /// primitive wrappers.
    BigInt = 30,
    /// A raw JSON object (`[[IsRawJSON]]`, JSON.rawJSON, D1611): frozen, null prototype,
    /// one own `rawJSON` string property, no slots.
    RawJSON = 29,
}

impl ObjectClass {
    pub const fn from_u32(c: u32) -> Option<ObjectClass> {
        match c {
            0 => Some(ObjectClass::Ordinary),
            1 => Some(ObjectClass::Array),
            2 => Some(ObjectClass::Function),
            3 => Some(ObjectClass::HostFunction),
            4 => Some(ObjectClass::Error),
            5 => Some(ObjectClass::Arguments),
            6 => Some(ObjectClass::ArrayIterator),
            7 => Some(ObjectClass::BoundFunction),
            8 => Some(ObjectClass::Boolean),
            9 => Some(ObjectClass::Number),
            10 => Some(ObjectClass::String),
            11 => Some(ObjectClass::Symbol),
            12 => Some(ObjectClass::Map),
            13 => Some(ObjectClass::Set),
            14 => Some(ObjectClass::WeakMap),
            15 => Some(ObjectClass::WeakSet),
            16 => Some(ObjectClass::CollectionIterator),
            17 => Some(ObjectClass::Promise),
            18 => Some(ObjectClass::BuiltinClosure),
            19 => Some(ObjectClass::Record),
            20 => Some(ObjectClass::Date),
            21 => Some(ObjectClass::Generator),
            22 => Some(ObjectClass::ArrayBuffer),
            23 => Some(ObjectClass::TypedArray),
            24 => Some(ObjectClass::DataView),
            25 => Some(ObjectClass::RegExp),
            26 => Some(ObjectClass::RegExpStringIterator),
            27 => Some(ObjectClass::Proxy),
            28 => Some(ObjectClass::IteratorHelper),
            31 => Some(ObjectClass::WeakRef),
            32 => Some(ObjectClass::FinalizationRegistry),
            33 => Some(ObjectClass::DisposableStack),
            30 => Some(ObjectClass::BigInt),
            29 => Some(ObjectClass::RawJSON),
            _ => None,
        }
    }

    /// Number of internal slots the class needs.
    pub const fn slots(self) -> u32 {
        match self {
            ObjectClass::Ordinary
            | ObjectClass::Error
            | ObjectClass::Arguments
            | ObjectClass::Record
            | ObjectClass::RawJSON => 0,
            ObjectClass::Promise => 4,
            ObjectClass::BuiltinClosure => 2,
            ObjectClass::Array | ObjectClass::Function | ObjectClass::HostFunction => 1,
            ObjectClass::ArrayIterator => 3,
            ObjectClass::BoundFunction => 3,
            ObjectClass::Boolean
            | ObjectClass::Number
            | ObjectClass::String
            | ObjectClass::Symbol
            | ObjectClass::BigInt => 1,
            ObjectClass::Map | ObjectClass::Set | ObjectClass::WeakMap | ObjectClass::WeakSet => 3,
            ObjectClass::CollectionIterator => 6,
            ObjectClass::Date => 1,
            ObjectClass::Generator => 7,
            ObjectClass::ArrayBuffer => 2,
            ObjectClass::TypedArray => 4,
            ObjectClass::DataView => 3,
            ObjectClass::RegExp | ObjectClass::RegExpStringIterator => 3,
            ObjectClass::Proxy => 3,
            ObjectClass::IteratorHelper => 8,
            ObjectClass::DisposableStack => 3,
            ObjectClass::WeakRef => 1,
            ObjectClass::FinalizationRegistry => 5,
        }
    }

    /// Callable objects of this class. A Proxy is callable when its target is: see
    /// [`Heap::is_callable`].
    pub const fn is_callable(self) -> bool {
        matches!(
            self,
            ObjectClass::Function
                | ObjectClass::HostFunction
                | ObjectClass::BoundFunction
                | ObjectClass::BuiltinClosure
        )
    }
}

/// Property attributes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PropFlags(u8);

impl PropFlags {
    pub const NONE: PropFlags = PropFlags(0);
    pub const WRITABLE: PropFlags = PropFlags(1);
    pub const ENUMERABLE: PropFlags = PropFlags(2);
    pub const CONFIGURABLE: PropFlags = PropFlags(4);
    /// Writable, enumerable and configurable: what assignment and dense elements use.
    pub const ALL: PropFlags = PropFlags(7);

    pub const fn new(writable: bool, enumerable: bool, configurable: bool) -> PropFlags {
        PropFlags(writable as u8 | (enumerable as u8) << 1 | (configurable as u8) << 2)
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn from_bits(bits: u8) -> PropFlags {
        PropFlags(bits & 7)
    }

    /// Meaningless for accessors (always false there).
    #[inline]
    pub const fn writable(self) -> bool {
        self.0 & 1 != 0
    }

    #[inline]
    pub const fn enumerable(self) -> bool {
        self.0 & 2 != 0
    }

    #[inline]
    pub const fn configurable(self) -> bool {
        self.0 & 4 != 0
    }
}

impl core::ops::BitOr for PropFlags {
    type Output = PropFlags;
    fn bitor(self, o: PropFlags) -> PropFlags {
        PropFlags(self.0 | o.0)
    }
}

/// A complete own property, as stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Property {
    Data {
        value: Value,
        flags: PropFlags,
    },
    /// `getter`/`setter` are function objects or `undefined`. `flags` has no writable bit.
    Accessor {
        getter: Value,
        setter: Value,
        flags: PropFlags,
    },
}

impl Property {
    pub const fn data(value: Value, flags: PropFlags) -> Property {
        Property::Data { value, flags }
    }

    pub const fn flags(&self) -> PropFlags {
        match self {
            Property::Data { flags, .. } | Property::Accessor { flags, .. } => *flags,
        }
    }

    pub const fn is_accessor(&self) -> bool {
        matches!(self, Property::Accessor { .. })
    }

    /// The value of a data property.
    pub const fn value(&self) -> Option<Value> {
        match self {
            Property::Data { value, .. } => Some(*value),
            Property::Accessor { .. } => None,
        }
    }
}

/// A property descriptor with optional fields (the spec's Property Descriptor). A
/// descriptor must not have both data fields (`value`, `writable`) and accessor fields
/// (`get`, `set`); `ToPropertyDescriptor` (caller side) rejects that.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PropertyDescriptor {
    pub value: Option<Value>,
    pub get: Option<Value>,
    pub set: Option<Value>,
    pub writable: Option<bool>,
    pub enumerable: Option<bool>,
    pub configurable: Option<bool>,
}

impl PropertyDescriptor {
    /// A complete data descriptor.
    pub const fn data(value: Value, flags: PropFlags) -> PropertyDescriptor {
        PropertyDescriptor {
            value: Some(value),
            get: None,
            set: None,
            writable: Some(flags.writable()),
            enumerable: Some(flags.enumerable()),
            configurable: Some(flags.configurable()),
        }
    }

    /// A complete accessor descriptor.
    pub const fn accessor(getter: Value, setter: Value, flags: PropFlags) -> PropertyDescriptor {
        PropertyDescriptor {
            value: None,
            get: Some(getter),
            set: Some(setter),
            writable: None,
            enumerable: Some(flags.enumerable()),
            configurable: Some(flags.configurable()),
        }
    }

    pub const fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    pub const fn is_data(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
    }

    pub const fn is_generic(&self) -> bool {
        !self.is_accessor() && !self.is_data()
    }

    const fn is_empty(&self) -> bool {
        self.is_generic() && self.enumerable.is_none() && self.configurable.is_none()
    }
}

/// The result of [`Heap::lookup_property`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lookup {
    /// The object on the chain that has the key, and the property.
    Found(Obj, Property),
    Absent,
    /// A Proxy reached before the key was found.
    Proxy(Obj),
}

/// Where an own property is stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Loc {
    None,
    Dense(u32),
    Table(u32),
    ArrayLength,
}

/// The result of ValidateAndApplyPropertyDescriptor.
enum Outcome {
    Reject,
    Keep,
    Write(Property),
}

#[inline]
fn key_hash(a: Atom) -> u32 {
    let h = a.id().wrapping_mul(0x9E37_79B9);
    h ^ (h >> 15)
}

impl Heap {
    // ---------------------------------------------------------------- objects

    /// Allocates an object with `slots` internal slots (set to `undefined`).
    pub fn alloc_object(
        &mut self,
        class: ObjectClass,
        proto: Option<Obj>,
        slots: u32,
    ) -> Result<Obj, HeapError> {
        let size = Object::SLOTS
            .cell_size(slots)
            .ok_or(HeapError::OutOfMemory)?;
        let o = self.alloc_cell(CellKind::Object, size)?;
        let a = self.arena_mut();
        a.set(o, Object::CLASS, class as u32);
        a.set(o, Object::PROTO, Ref(proto.map_or(0, |p| p.0)));
        for i in 0..slots {
            a.write(Object::SLOTS.at(o, i), Value::UNDEFINED);
        }
        self.set_cell_flags(o, OBJ_EXTENSIBLE);
        Ok(Obj(o))
    }

    /// A new ordinary object.
    pub fn new_object(&mut self, proto: Option<Obj>) -> Result<Obj, HeapError> {
        self.alloc_object(ObjectClass::Ordinary, proto, 0)
    }

    /// A new empty array with room for `capacity` dense elements.
    pub fn new_array(&mut self, proto: Option<Obj>, capacity: u32) -> Result<Obj, HeapError> {
        let o = self.alloc_object(ObjectClass::Array, proto, ObjectClass::Array.slots())?;
        self.set_array_length_raw(o, 0);
        if capacity > 0 {
            let el = self.alloc_value_array(capacity)?;
            self.arena_mut().set(o.0, Object::ELEMENTS, Ref(el));
        }
        Ok(o)
    }

    /// A new ordinary object with the data properties `keys[i]: values[i]` (writable,
    /// enumerable, configurable), defined in order: a repeated key keeps its first
    /// position and its last value, as repeated CreateDataProperty calls would. The table is
    /// allocated once at its final size. Used by JSON.parse and object literals.
    pub fn new_object_from_entries(
        &mut self,
        proto: Option<Obj>,
        keys: &[Atom],
        values: &[Value],
    ) -> Result<Obj, HeapError> {
        assert_eq!(keys.len(), values.len());
        let o = self.new_object(proto)?;
        if keys.is_empty() {
            return Ok(o);
        }
        let n = u32::try_from(keys.len()).map_err(|_| HeapError::OutOfMemory)?;
        let t = self.alloc_table(n.max(4))?;
        self.arena_mut().set(o.0, Object::PROPS, Ref(t));
        let mut indexed = false;
        for (&key, &value) in keys.iter().zip(values) {
            let p = Property::data(value, PropFlags::ALL);
            let t = self.props(o);
            match self.table_find(t, key) {
                Some(i) => self.write_entry(t, i, key, &p)?,
                None => {
                    self.table_insert(o, key, &p)?;
                    indexed |= self.atom_array_index(key).is_some();
                }
            }
        }
        if indexed {
            self.set_obj_flag(o, OBJ_INDEXED_PROPS, true);
        }
        Ok(o)
    }

    /// A new array with `values` as its dense elements (`length` = `values.len()`).
    /// Values may be holes.
    pub fn new_array_from_values(
        &mut self,
        proto: Option<Obj>,
        values: &[Value],
    ) -> Result<Obj, HeapError> {
        let n = u32::try_from(values.len())
            .ok()
            .filter(|&n| n <= atom_int_max())
            .ok_or(HeapError::OutOfMemory)?;
        let o = self.new_array(proto, n)?;
        if n > 0 {
            let el = self.elements(o);
            let a = self.arena_mut();
            let base = ValueArray::VALUES.at(el, 0);
            let dst = a.slice_mut(base, n * 8);
            for (d, v) in dst.as_chunks_mut::<8>().0.iter_mut().zip(values) {
                *d = v.to_bits().to_le_bytes();
            }
            a.set(el, ValueArray::LEN, n);
            self.set_array_length_raw(o, n);
        }
        Ok(o)
    }

    pub fn object_class(&self, o: Obj) -> ObjectClass {
        let c = self.arena().get(o.0, Object::CLASS);
        ObjectClass::from_u32(c).unwrap_or_else(|| panic!("invalid object class {c}"))
    }

    /// Number of internal slots.
    pub fn object_slot_count(&self, o: Obj) -> u32 {
        Object::LAYOUT.tail_capacity(self.cell_size(o.0))
    }

    pub fn object_slot(&self, o: Obj, i: u32) -> Value {
        assert!(i < self.object_slot_count(o), "slot {i} out of range");
        self.arena().read(Object::SLOTS.at(o.0, i))
    }

    pub fn set_object_slot(&mut self, o: Obj, i: u32, v: Value) {
        assert!(i < self.object_slot_count(o), "slot {i} out of range");
        self.arena_mut().write(Object::SLOTS.at(o.0, i), v)
    }

    #[inline]
    fn obj_flags(&self, o: Obj) -> u16 {
        self.cell_flags(o.0)
    }

    fn set_obj_flag(&mut self, o: Obj, flag: u16, on: bool) {
        let f = self.cell_flags(o.0);
        self.set_cell_flags(o.0, if on { f | flag } else { f & !flag });
    }

    pub fn get_prototype_of(&self, o: Obj) -> Option<Obj> {
        self.arena().get(o.0, Object::PROTO).get().map(Obj)
    }

    /// OrdinarySetPrototypeOf: fails on a non-extensible object or a cycle.
    pub fn set_prototype_of(&mut self, o: Obj, proto: Option<Obj>) -> bool {
        if self.get_prototype_of(o) == proto {
            return true;
        }
        if !self.is_extensible(o) {
            return false;
        }
        let mut p = proto;
        while let Some(q) = p {
            if q == o {
                return false;
            }
            p = self.get_prototype_of(q);
        }
        self.arena_mut()
            .set(o.0, Object::PROTO, Ref(proto.map_or(0, |p| p.0)));
        self.proto_changed(o);
        true
    }

    /// The prototype epoch of the inline caches (D1621).
    #[inline(always)]
    pub fn proto_epoch(&self) -> u32 {
        self.arena().get(0, Roots::PROTO_EPOCH)
    }

    pub(crate) fn bump_proto_epoch(&mut self) {
        let e = self.proto_epoch();
        self.arena_mut()
            .set(0, Roots::PROTO_EPOCH, e.wrapping_add(1));
    }

    /// Marks `o` as on the chain of a cached prototype property (D1621).
    pub fn mark_proto(&mut self, o: Obj) {
        self.set_obj_flag(o, OBJ_IS_PROTO, true);
    }

    /// A key was added to `o` or its prototype changed: caches through it are stale.
    #[inline]
    fn proto_changed(&mut self, o: Obj) {
        if self.obj_flags(o) & OBJ_IS_PROTO != 0 {
            self.bump_proto_epoch();
        }
    }

    pub fn is_extensible(&self, o: Obj) -> bool {
        self.obj_flags(o) & OBJ_EXTENSIBLE != 0
    }

    pub fn prevent_extensions(&mut self, o: Obj) -> bool {
        self.set_obj_flag(o, OBJ_EXTENSIBLE, false);
        true
    }

    // ---------------------------------------------------------------- arrays

    /// The `length` of an array.
    pub fn array_length(&self, o: Obj) -> u32 {
        debug_assert_eq!(self.object_class(o), ObjectClass::Array);
        let n = self.object_slot(o, 0).as_number().unwrap_or(0.0);
        n as u32
    }

    fn set_array_length_raw(&mut self, o: Obj, len: u32) {
        let v = if len <= i32::MAX as u32 {
            Value::int(len as i32)
        } else {
            Value::number(len as f64)
        };
        self.set_object_slot(o, 0, v);
    }

    #[inline]
    fn elements(&self, o: Obj) -> u32 {
        self.arena().get(o.0, Object::ELEMENTS).0
    }

    /// The dense length (0 without elements).
    pub fn dense_len(&self, o: Obj) -> u32 {
        match self.elements(o) {
            0 => 0,
            el => self.value_array_len(el),
        }
    }

    /// The first `length` slots of an Array's dense store, if they cover its length
    /// (holes included: callers check `is_hole`), read straight from one slice. For bulk
    /// readers (`String.fromCodePoint`, `apply`): no per-element class or bounds checks.
    /// `None` for anything else.
    pub(crate) fn dense_array_values(&self, o: Obj) -> Option<impl Iterator<Item = Value> + '_> {
        if self.object_class(o) != ObjectClass::Array {
            return None;
        }
        let n = self.array_length(o);
        let el = self.elements(o);
        let bytes: &[u8] = if n == 0 {
            &[]
        } else if el != 0 && self.value_array_len(el) >= n {
            self.arena()
                .slice(ValueArray::VALUES.at(el, 0), n.checked_mul(8)?)
        } else {
            return None;
        };
        Some(
            bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|b| Value::from_raw(u64::from_le_bytes(*b))),
        )
    }

    /// Fast path for reading element `i`: `Some` if it is a dense, non-hole element.
    /// `None` means "take the generic path" (property table, prototype chain).
    #[inline]
    pub fn get_element_fast(&self, o: Obj, i: u32) -> Option<Value> {
        let el = self.elements(o);
        if el == 0 {
            // Typed arrays have no elements store: their elements live in the buffer.
            // A BigInt element (a placeholder here) takes the generic path.
            return if self.object_class(o) == ObjectClass::TypedArray {
                self.ta_get(o, i).filter(|v| !v.is_hole())
            } else {
                None
            };
        }
        if i >= self.value_array_len(el) {
            return None;
        }
        let v = self.value_array_get(el, i);
        if v.is_hole() { None } else { Some(v) }
    }

    /// Fast path for `o[i] = v` with `o` as receiver. Returns `Ok(true)` when done:
    /// overwriting a dense element, or adding one (filling a hole or appending) to an
    /// extensible array whose prototype chain has no index properties. `Ok(false)` means
    /// "take the generic `[[Set]]` path".
    /// splice's element moves on a plain dense array in one step: `o[start .. start +
    /// delete)` is replaced by `insert` undefined slots (the caller writes them), the
    /// tail moves, and `length` follows. Only for an extensible Array with a writable
    /// length, no holes, every element dense (default attributes) and no index
    /// properties on its prototype chain, where the generic moves (HasProperty, Get,
    /// Set, DeletePropertyOrThrow) could not observe anything else. `Ok(false)`: not
    /// such an array; nothing changed.
    pub(crate) fn array_splice_dense(
        &mut self,
        o: Obj,
        start: u32,
        delete: u32,
        insert: u32,
    ) -> Result<bool, HeapError> {
        if self.object_class(o) != ObjectClass::Array {
            return Ok(false);
        }
        let f = self.obj_flags(o);
        if f & OBJ_EXTENSIBLE == 0 || f & (OBJ_INDEXED_PROPS | OBJ_LENGTH_READONLY) != 0 {
            return Ok(false);
        }
        let len = self.array_length(o);
        if self.dense_len(o) != len || start > len || delete > len - start {
            return Ok(false);
        }
        let mut p = self.get_prototype_of(o);
        while let Some(q) = p {
            if self.dense_len(q) != 0 || self.obj_flags(q) & OBJ_INDEXED_PROPS != 0 {
                return Ok(false);
            }
            p = self.get_prototype_of(q);
        }
        let el = self.elements(o);
        let mut values: Vec<Value> = Vec::with_capacity((len - delete + insert) as usize);
        for i in 0..len {
            let v = self.value_array_get(el, i);
            if v.is_hole() {
                return Ok(false);
            }
            if i == start {
                values.extend(core::iter::repeat_n(Value::UNDEFINED, insert as usize));
            }
            if i < start || i >= start + delete {
                values.push(v);
            }
        }
        if start == len {
            values.extend(core::iter::repeat_n(Value::UNDEFINED, insert as usize));
        }
        let new_len = u32::try_from(values.len()).map_err(|_| HeapError::OutOfMemory)?;
        if new_len > atom_int_max() {
            return Ok(false);
        }
        for (i, v) in values.into_iter().enumerate() {
            self.dense_put(o, i as u32, v)?;
        }
        if new_len < len {
            self.truncate_elements(o, new_len);
        }
        self.set_array_length_raw(o, new_len);
        Ok(true)
    }

    pub fn set_element_fast(&mut self, o: Obj, i: u32, v: Value) -> Result<bool, HeapError> {
        let el = self.elements(o);
        let len = if el == 0 { 0 } else { self.value_array_len(el) };
        if i < len && !self.value_array_get(el, i).is_hole() {
            self.value_array_set(el, i, v);
            return Ok(true);
        }
        if i > len || i > atom_int_max() || self.object_class(o) != ObjectClass::Array {
            // A typed array element: [[Set]] with the array as receiver always succeeds
            // (an invalid index writes nothing), once the value is a primitive.
            if el == 0 && self.object_class(o) == ObjectClass::TypedArray {
                return Ok(match self.element_of(self.ta_kind(o), v) {
                    Some(x) => {
                        self.ta_set(o, i, x);
                        true
                    }
                    None => false,
                });
            }
            return Ok(false);
        }
        let f = self.obj_flags(o);
        if f & OBJ_EXTENSIBLE == 0 || f & OBJ_INDEXED_PROPS != 0 {
            return Ok(false);
        }
        let length = self.array_length(o);
        if i >= length && f & OBJ_LENGTH_READONLY != 0 {
            return Ok(false);
        }
        let mut p = self.get_prototype_of(o);
        while let Some(q) = p {
            if self.dense_len(q) != 0 || self.obj_flags(q) & OBJ_INDEXED_PROPS != 0 {
                return Ok(false);
            }
            p = self.get_prototype_of(q);
        }
        self.dense_put(o, i, v)?;
        if i >= length {
            self.set_array_length_raw(o, i + 1);
        }
        Ok(true)
    }

    /// Stores `v` in dense slot `i`, growing the elements (with holes) as needed.
    fn dense_put(&mut self, o: Obj, i: u32, v: Value) -> Result<(), HeapError> {
        let mut el = self.elements(o);
        let len = if el == 0 { 0 } else { self.value_array_len(el) };
        if i >= len {
            let grown = self.value_array_reserve(el, i + 1)?;
            if grown != el {
                el = grown;
                self.arena_mut().set(o.0, Object::ELEMENTS, Ref(el));
            }
            for j in len..i {
                self.value_array_set(el, j, Value::HOLE);
            }
            self.arena_mut().set(el, ValueArray::LEN, i + 1);
        }
        self.value_array_set(el, i, v);
        Ok(())
    }

    /// Whether a new element at `i` should go into dense storage.
    fn dense_accepts(&self, o: Obj, i: u32) -> bool {
        self.object_class(o) == ObjectClass::Array
            && i <= atom_int_max()
            && i <= self.dense_len(o).saturating_add(DENSE_GAP)
    }

    // ---------------------------------------------------------------- property table

    #[inline]
    fn props(&self, o: Obj) -> u32 {
        self.arena().get(o.0, Object::PROPS).0
    }

    #[inline]
    fn entry_key(&self, t: u32, i: u32) -> Atom {
        Atom::from_id(
            self.arena()
                .get(PropTable::ENTRIES.at(t, i), PropEntry::KEY),
        )
    }

    fn table_capacity(&self, t: u32) -> u32 {
        PropTable::LAYOUT.tail_capacity(self.cell_size(t))
    }

    fn table_find(&self, t: u32, key: Atom) -> Option<u32> {
        let a = self.arena();
        let count = a.get(t, PropTable::COUNT);
        let index = a.get(t, PropTable::INDEX).0;
        if index == 0 {
            return (0..count).find(|&i| self.entry_key(t, i) == key);
        }
        self.probe(index, key_hash(key), |_, e| match e {
            0 => Some(None),
            TOMBSTONE => None,
            e => (self.entry_key(t, e - 1) == key).then_some(Some(e - 1)),
        })
    }

    // ---------------------------------------------------------------- inline caches

    /// The table entry of own property `key` of `o`, if it is stored in the property
    /// table (not a dense element, not an array's `length`): the index an inline cache
    /// remembers (D1620). Only ordinary storage: callers check the class.
    #[inline]
    pub fn own_entry_index(&self, o: Obj, key: Atom) -> Option<u32> {
        match self.props(o) {
            0 => None,
            t => self.table_find(t, key),
        }
    }

    /// The value of entry `i` of `o`'s property table if that entry is the data property
    /// `key` (an inline cache hit, D1620); `None` for anything else, including a stale
    /// index. Entry keys are never array indexes of arrays here, since those callers
    /// only cache non-index atoms.
    #[inline(always)]
    pub fn data_at(&self, o: Obj, i: u32, key: Atom) -> Option<Value> {
        let t = self.props(o);
        if t == 0 || i >= self.arena().get(t, PropTable::COUNT) {
            return None;
        }
        let e = PropTable::ENTRIES.at(t, i);
        let a = self.arena();
        if a.get(e, PropEntry::KEY) != key.id() || a.get(e, PropEntry::FLAGS) & STORE_ACCESSOR != 0
        {
            return None;
        }
        Some(a.get(e, PropEntry::VALUE))
    }

    /// Writes `v` to entry `i` of `o`'s table if it is the writable data property `key`
    /// (an inline cache hit of a store, D1620); false with nothing changed otherwise.
    #[inline(always)]
    pub fn set_data_at(&mut self, o: Obj, i: u32, key: Atom, v: Value) -> bool {
        let t = self.props(o);
        if t == 0 || i >= self.arena().get(t, PropTable::COUNT) {
            return false;
        }
        let e = PropTable::ENTRIES.at(t, i);
        let fl = self.arena().get(e, PropEntry::FLAGS);
        if self.arena().get(e, PropEntry::KEY) != key.id()
            || fl & STORE_ACCESSOR != 0
            || !PropFlags::from_bits(fl as u8).writable()
        {
            return false;
        }
        self.arena_mut().set(e, PropEntry::VALUE, v);
        true
    }

    fn read_entry(&self, t: u32, i: u32) -> Property {
        let e = PropTable::ENTRIES.at(t, i);
        let a = self.arena();
        let fl = a.get(e, PropEntry::FLAGS);
        let flags = PropFlags::from_bits(fl as u8);
        if fl & STORE_ACCESSOR != 0 {
            let pair = a
                .get::<Value>(e, PropEntry::VALUE)
                .as_internal()
                .expect("accessor pair");
            Property::Accessor {
                getter: a.get(pair, AccessorPair::GETTER),
                setter: a.get(pair, AccessorPair::SETTER),
                flags,
            }
        } else {
            Property::Data {
                value: a.get(e, PropEntry::VALUE),
                flags,
            }
        }
    }

    /// Writes entry i. An accessor reuses the entry's pair if it has one, else allocates
    /// one first, so a failed allocation changes nothing (D1700).
    fn write_entry(&mut self, t: u32, i: u32, key: Atom, p: &Property) -> Result<(), HeapError> {
        let e = PropTable::ENTRIES.at(t, i);
        match *p {
            Property::Data { value, flags } => {
                let a = self.arena_mut();
                a.set(e, PropEntry::KEY, key.id());
                a.set(e, PropEntry::FLAGS, flags.bits() as u32);
                a.set(e, PropEntry::VALUE, value);
            }
            Property::Accessor {
                getter,
                setter,
                flags,
            } => {
                let a = self.arena();
                let old = (a.get(e, PropEntry::KEY) == key.id()
                    && a.get(e, PropEntry::FLAGS) & STORE_ACCESSOR != 0)
                    .then(|| a.get::<Value>(e, PropEntry::VALUE).as_internal())
                    .flatten();
                let pair = match old {
                    Some(pair) => pair,
                    None => {
                        self.alloc_cell(CellKind::AccessorPair, AccessorPair::LAYOUT.min_size)?
                    }
                };
                // Accessors have no writable attribute.
                let bits = (flags.bits() & !PropFlags::WRITABLE.bits()) as u32;
                let a = self.arena_mut();
                a.set(pair, AccessorPair::GETTER, getter);
                a.set(pair, AccessorPair::SETTER, setter);
                a.set(e, PropEntry::KEY, key.id());
                a.set(e, PropEntry::FLAGS, bits | STORE_ACCESSOR);
                a.set(e, PropEntry::VALUE, Value::internal(pair));
            }
        }
        Ok(())
    }

    fn alloc_table(&mut self, cap: u32) -> Result<u32, HeapError> {
        let size = PropTable::ENTRIES
            .cell_size(cap)
            .ok_or(HeapError::OutOfMemory)?;
        self.alloc_cell(CellKind::PropTable, size)
    }

    /// Builds a hash index for table `t`, sized for its full capacity.
    pub(super) fn build_table_index(&mut self, t: u32) -> Result<(), HeapError> {
        self.build_index(t)
    }

    fn build_index(&mut self, t: u32) -> Result<(), HeapError> {
        let buckets = (self.table_capacity(t) * 2).next_power_of_two();
        let index = self.alloc_u32_array(buckets)?;
        for i in 0..self.arena().get(t, PropTable::COUNT) {
            let key = self.entry_key(t, i);
            if key.is_null() {
                continue;
            }
            let b = self.probe_empty(index, key_hash(key));
            self.u32_array_set(index, b, i + 1);
        }
        self.arena_mut().set(t, PropTable::INDEX, Ref(index));
        Ok(())
    }

    /// Copies the live entries of `t` (or nothing) into a new, larger table for `o`.
    fn grow_table(&mut self, o: Obj, t: u32) -> Result<u32, HeapError> {
        let (count, deleted) = if t == 0 {
            (0, 0)
        } else {
            let a = self.arena();
            (a.get(t, PropTable::COUNT), a.get(t, PropTable::DELETED))
        };
        let live = count - deleted;
        let cap = (live * 2).max(4);
        let new = self.alloc_table(cap)?;
        let mut n = 0;
        for i in 0..count {
            if self.entry_key(t, i).is_null() {
                continue;
            }
            let src = PropTable::ENTRIES.at(t, i);
            let dst = PropTable::ENTRIES.at(new, n);
            self.arena_mut()
                .copy_within(src, PropTable::ENTRIES.elem_size(), dst);
            n += 1;
        }
        self.arena_mut().set(new, PropTable::COUNT, n);
        if n > LINEAR_MAX {
            self.build_index(new)?;
        }
        self.arena_mut().set(o.0, Object::PROPS, Ref(new));
        Ok(new)
    }

    /// Appends a property (the key must not be present).
    fn table_insert(&mut self, o: Obj, key: Atom, p: &Property) -> Result<(), HeapError> {
        self.proto_changed(o);
        let mut t = self.props(o);
        if t == 0 || self.arena().get(t, PropTable::COUNT) == self.table_capacity(t) {
            t = self.grow_table(o, t)?;
        }
        let count = self.arena().get(t, PropTable::COUNT);
        let mut index = self.arena().get(t, PropTable::INDEX).0;
        if index == 0 && count + 1 > LINEAR_MAX {
            self.build_index(t)?;
            index = self.arena().get(t, PropTable::INDEX).0;
        }
        self.write_entry(t, count, key, p)?;
        self.arena_mut().set(t, PropTable::COUNT, count + 1);
        if index != 0 {
            let b = self.probe(index, key_hash(key), |b, v| {
                matches!(v, 0 | TOMBSTONE).then_some(b)
            });
            self.u32_array_set(index, b, count + 1);
        }
        Ok(())
    }

    /// Removes entry `i` of `o`'s table.
    fn table_remove(&mut self, o: Obj, i: u32) {
        let t = self.props(o);
        let count = self.arena().get(t, PropTable::COUNT);
        let index = self.arena().get(t, PropTable::INDEX).0;
        let esize = PropTable::ENTRIES.elem_size();
        if index == 0 {
            // Linear mode stays compact, preserving insertion order.
            let src = PropTable::ENTRIES.at(t, i + 1);
            let dst = PropTable::ENTRIES.at(t, i);
            self.arena_mut()
                .copy_within(src, (count - i - 1) * esize, dst);
            self.arena_mut()
                .zero(PropTable::ENTRIES.at(t, count - 1), esize);
            self.arena_mut().set(t, PropTable::COUNT, count - 1);
            return;
        }
        let key = self.entry_key(t, i);
        let b = self.probe(index, key_hash(key), |b, v| (v == i + 1).then_some(b));
        self.u32_array_set(index, b, TOMBSTONE);
        self.arena_mut().zero(PropTable::ENTRIES.at(t, i), esize);
        let deleted = self.arena().get(t, PropTable::DELETED);
        self.arena_mut().set(t, PropTable::DELETED, deleted + 1);
    }

    // ---------------------------------------------------------------- own properties

    fn find_own(&self, o: Obj, key: Atom) -> Loc {
        if let Some(i) = key.as_int() {
            if self.get_element_fast(o, i).is_some() {
                return Loc::Dense(i);
            }
            if self.obj_flags(o) & OBJ_INDEXED_PROPS == 0 {
                return Loc::None;
            }
        } else if key == atoms::LENGTH && self.object_class(o) == ObjectClass::Array {
            return Loc::ArrayLength;
        }
        match self.props(o) {
            0 => Loc::None,
            t => self.table_find(t, key).map_or(Loc::None, Loc::Table),
        }
    }

    fn read_loc(&self, o: Obj, loc: Loc) -> Option<Property> {
        match loc {
            Loc::None => None,
            Loc::Dense(i) => Some(Property::data(
                self.value_array_get(self.elements(o), i),
                PropFlags::ALL,
            )),
            Loc::Table(i) => Some(self.read_entry(self.props(o), i)),
            Loc::ArrayLength => Some(self.array_length_property(o)),
        }
    }

    fn array_length_property(&self, o: Obj) -> Property {
        let writable = self.obj_flags(o) & OBJ_LENGTH_READONLY == 0;
        Property::data(
            Value::number(self.array_length(o) as f64),
            PropFlags::new(writable, false, false),
        )
    }

    /// [[Set]] on an own writable data property, which only replaces its value
    /// (OrdinarySet's DefineOwnProperty with `{ [[Value]]: v }`). Returns false, with
    /// nothing changed, for anything else (absent, an accessor, read-only, an array's
    /// `length`), which needs the full algorithm. The interpreter's hot path (0035).
    pub fn set_own_data_value(&mut self, o: Obj, key: Atom, v: Value) -> bool {
        if self.object_class(o) == ObjectClass::TypedArray
            && self.canonical_numeric_index(key).is_some()
        {
            // [[Set]] on a typed array: true even for an invalid index (nothing written),
            // once the value is a primitive.
            if self.element_of(self.ta_kind(o), v).is_some() {
                self.ta_numeric_set(o, key, v);
                return true;
            }
            return false;
        }
        match self.find_own(o, key) {
            Loc::Dense(i) => {
                let el = self.elements(o);
                self.value_array_set(el, i, v);
                true
            }
            Loc::Table(i) => {
                let e = PropTable::ENTRIES.at(self.props(o), i);
                let fl = self.arena().get(e, PropEntry::FLAGS);
                if fl & STORE_ACCESSOR != 0 || !PropFlags::from_bits(fl as u8).writable() {
                    return false;
                }
                self.arena_mut().set(e, PropEntry::VALUE, v);
                true
            }
            Loc::None | Loc::ArrayLength => false,
        }
    }

    /// `[[GetOwnProperty]]`.
    pub fn get_own_property(&self, o: Obj, key: Atom) -> Option<Property> {
        if self.object_class(o) == ObjectClass::TypedArray
            && let Some(r) = self.ta_numeric_get(o, key)
        {
            return r.map(|v| Property::data(v, PropFlags::ALL));
        }
        self.read_loc(o, self.find_own(o, key))
    }

    pub fn has_own_property(&self, o: Obj, key: Atom) -> bool {
        if self.object_class(o) == ObjectClass::TypedArray
            && let Some(r) = self.ta_numeric_get(o, key)
        {
            return r.is_some();
        }
        self.find_own(o, key) != Loc::None
    }

    /// Looks `key` up along the prototype chain. Returns the object that has it and the
    /// property; for an accessor the caller calls the getter with the original receiver.
    /// A Proxy on the chain ends the walk as if it were an empty object without a
    /// prototype: callers that can meet one use [`Heap::lookup_property`].
    pub fn find_property(&self, o: Obj, key: Atom) -> Option<(Obj, Property)> {
        match self.lookup_property(o, key) {
            Lookup::Found(h, p) => Some((h, p)),
            Lookup::Absent | Lookup::Proxy(_) => None,
        }
    }

    /// [`Heap::find_property`] that reports a Proxy on the chain (`o` itself or a
    /// prototype) before the key was found: the rest of the lookup is its traps'.
    #[inline]
    pub fn lookup_property(&self, o: Obj, key: Atom) -> Lookup {
        let mut c = o;
        loop {
            if let Some(p) = self.get_own_property(c, key) {
                return Lookup::Found(c, p);
            }
            match self.object_class(c) {
                // A typed array answers numeric keys itself: no prototype lookup.
                ObjectClass::TypedArray if self.canonical_numeric_index(key).is_some() => {
                    return Lookup::Absent;
                }
                ObjectClass::Proxy => return Lookup::Proxy(c),
                _ => {}
            }
            match self.get_prototype_of(c) {
                Some(p) => c = p,
                None => return Lookup::Absent,
            }
        }
    }

    /// `[[HasProperty]]`, with a Proxy on the chain counting as absent: callers that can
    /// meet one use [`Heap::has_property_or_proxy`].
    pub fn has_property(&self, o: Obj, key: Atom) -> bool {
        self.has_property_or_proxy(o, key) == Ok(true)
    }

    /// `[[HasProperty]]` up to a Proxy on the chain: `Err(proxy)` when the answer is its
    /// `has` trap's.
    #[inline]
    pub fn has_property_or_proxy(&self, o: Obj, key: Atom) -> Result<bool, Obj> {
        let mut c = o;
        loop {
            if self.has_own_property(c, key) {
                return Ok(true);
            }
            match self.object_class(c) {
                ObjectClass::TypedArray if self.canonical_numeric_index(key).is_some() => {
                    return Ok(false);
                }
                ObjectClass::Proxy => return Err(c),
                _ => {}
            }
            match self.get_prototype_of(c) {
                Some(p) => c = p,
                None => return Ok(false),
            }
        }
    }

    /// Stores a complete property at `loc` (where `key` currently is, or `Loc::None`).
    fn write_property(
        &mut self,
        o: Obj,
        key: Atom,
        loc: Loc,
        p: Property,
    ) -> Result<(), HeapError> {
        let dense_ok = matches!(p, Property::Data { flags, .. } if flags == PropFlags::ALL);
        match loc {
            Loc::Dense(i) => {
                if let (true, Property::Data { value, .. }) = (dense_ok, p) {
                    self.value_array_set(self.elements(o), i, value);
                } else {
                    // Non-default attributes: move the element to the table.
                    self.table_insert(o, key, &p)?;
                    self.set_obj_flag(o, OBJ_INDEXED_PROPS, true);
                    self.value_array_set(self.elements(o), i, Value::HOLE);
                }
            }
            Loc::Table(i) => {
                let t = self.props(o);
                self.write_entry(t, i, key, &p)?;
            }
            Loc::ArrayLength => unreachable!("array length is written by array_set_length"),
            Loc::None => {
                if let (true, Some(i), Property::Data { value, .. }) = (dense_ok, key.as_int(), p)
                    && self.dense_accepts(o, i)
                {
                    self.dense_put(o, i, value)?;
                } else {
                    self.table_insert(o, key, &p)?;
                    if self.atom_array_index(key).is_some() {
                        self.set_obj_flag(o, OBJ_INDEXED_PROPS, true);
                    }
                }
            }
        }
        Ok(())
    }

    /// SameValue.
    pub fn same_value(&self, a: Value, b: Value) -> bool {
        if let (Some(x), Some(y)) = (a.as_number(), b.as_number()) {
            return (x.is_nan() && y.is_nan())
                || (x == y && x.is_sign_negative() == y.is_sign_negative());
        }
        if let (Some(x), Some(y)) = (a.as_string(), b.as_string()) {
            return self.str_eq(x, y);
        }
        if a.as_bigint().is_some() && b.as_bigint().is_some() {
            return self.bigint_eq(a, b);
        }
        a == b
    }

    /// ValidateAndApplyPropertyDescriptor, without the write.
    fn validate_descriptor(
        &self,
        current: Option<Property>,
        extensible: bool,
        d: &PropertyDescriptor,
    ) -> Outcome {
        debug_assert!(!(d.is_accessor() && d.is_data()));
        let undef = Value::UNDEFINED;
        let Some(cur) = current else {
            if !extensible {
                return Outcome::Reject;
            }
            let e = d.enumerable.unwrap_or(false);
            let c = d.configurable.unwrap_or(false);
            return Outcome::Write(if d.is_accessor() {
                Property::Accessor {
                    getter: d.get.unwrap_or(undef),
                    setter: d.set.unwrap_or(undef),
                    flags: PropFlags::new(false, e, c),
                }
            } else {
                Property::Data {
                    value: d.value.unwrap_or(undef),
                    flags: PropFlags::new(d.writable.unwrap_or(false), e, c),
                }
            });
        };
        if d.is_empty() {
            return Outcome::Keep;
        }
        let cf = cur.flags();
        if !cf.configurable() {
            if d.configurable == Some(true) {
                return Outcome::Reject;
            }
            if d.enumerable.is_some_and(|e| e != cf.enumerable()) {
                return Outcome::Reject;
            }
            if !d.is_generic() && d.is_accessor() != cur.is_accessor() {
                return Outcome::Reject;
            }
            match cur {
                Property::Accessor { getter, setter, .. } => {
                    if d.get.is_some_and(|g| !self.same_value(g, getter))
                        || d.set.is_some_and(|s| !self.same_value(s, setter))
                    {
                        return Outcome::Reject;
                    }
                }
                Property::Data { value, flags } if !flags.writable() => {
                    if d.writable == Some(true)
                        || d.value.is_some_and(|v| !self.same_value(v, value))
                    {
                        return Outcome::Reject;
                    }
                }
                Property::Data { .. } => {}
            }
        }
        let e = d.enumerable.unwrap_or(cf.enumerable());
        let c = d.configurable.unwrap_or(cf.configurable());
        Outcome::Write(match cur {
            Property::Data { .. } if d.is_accessor() => Property::Accessor {
                getter: d.get.unwrap_or(undef),
                setter: d.set.unwrap_or(undef),
                flags: PropFlags::new(false, e, c),
            },
            Property::Accessor { .. } if d.is_data() => Property::Data {
                value: d.value.unwrap_or(undef),
                flags: PropFlags::new(d.writable.unwrap_or(false), e, c),
            },
            Property::Data { value, flags } => Property::Data {
                value: d.value.unwrap_or(value),
                flags: PropFlags::new(d.writable.unwrap_or(flags.writable()), e, c),
            },
            Property::Accessor { getter, setter, .. } => Property::Accessor {
                getter: d.get.unwrap_or(getter),
                setter: d.set.unwrap_or(setter),
                flags: PropFlags::new(false, e, c),
            },
        })
    }

    /// OrdinaryDefineOwnProperty.
    fn ordinary_define(
        &mut self,
        o: Obj,
        key: Atom,
        d: &PropertyDescriptor,
    ) -> Result<bool, HeapError> {
        let loc = self.find_own(o, key);
        let current = self.read_loc(o, loc);
        match self.validate_descriptor(current, self.is_extensible(o), d) {
            Outcome::Reject => Ok(false),
            Outcome::Keep => Ok(true),
            Outcome::Write(p) => {
                self.write_property(o, key, loc, p)?;
                Ok(true)
            }
        }
    }

    /// `[[DefineOwnProperty]]` (ordinary, plus the Array exotic behavior). `Ok(false)` means
    /// the spec returns false (the caller throws a TypeError in strict code). For an
    /// array's `length`, `d.value` must already be a Number (the caller does the
    /// ToNumber/ToUint32 conversions); a value that is not a `u32` gives
    /// [`HeapError::InvalidArrayLength`].
    pub fn define_own_property(
        &mut self,
        o: Obj,
        key: Atom,
        d: &PropertyDescriptor,
    ) -> Result<bool, HeapError> {
        if self.object_class(o) == ObjectClass::TypedArray
            && let Some(n) = self.canonical_numeric_index(key)
        {
            // IntegerIndexedDefineOwnProperty (10.4.5.3).
            let valid = self.ta_get_valid(o, n);
            if !valid
                || d.configurable == Some(false)
                || d.enumerable == Some(false)
                || d.is_accessor()
                || d.writable == Some(false)
            {
                return Ok(false);
            }
            if let Some(v) = d.value {
                return Ok(self.ta_numeric_set(o, key, v) == Some(true));
            }
            return Ok(true);
        }
        if self.object_class(o) == ObjectClass::Array {
            if key == atoms::LENGTH {
                return self.array_set_length(o, d);
            }
            if let Some(i) = self.atom_array_index(key) {
                let len = self.array_length(o);
                if i >= len && self.obj_flags(o) & OBJ_LENGTH_READONLY != 0 {
                    return Ok(false);
                }
                if !self.ordinary_define(o, key, d)? {
                    return Ok(false);
                }
                if i >= len {
                    self.set_array_length_raw(o, i + 1);
                }
                return Ok(true);
            }
        }
        self.ordinary_define(o, key, d)
    }

    /// CreateDataProperty: define `key` as a writable, enumerable, configurable data
    /// property.
    pub fn create_data_property(
        &mut self,
        o: Obj,
        key: Atom,
        value: Value,
    ) -> Result<bool, HeapError> {
        // A new key on an extensible ordinary object (object literals, the common case):
        // what ordinary_define would do, without building and validating a descriptor.
        if !matches!(
            self.object_class(o),
            ObjectClass::Array | ObjectClass::TypedArray
        ) && key.as_int().is_none()
            && self.find_own(o, key) == Loc::None
            && self.is_extensible(o)
        {
            self.table_insert(o, key, &Property::data(value, PropFlags::ALL))?;
            if self.atom_array_index(key).is_some() {
                self.set_obj_flag(o, OBJ_INDEXED_PROPS, true);
            }
            return Ok(true);
        }
        self.define_own_property(o, key, &PropertyDescriptor::data(value, PropFlags::ALL))
    }

    /// ArraySetLength.
    fn array_set_length(&mut self, o: Obj, d: &PropertyDescriptor) -> Result<bool, HeapError> {
        let current = self.array_length_property(o);
        let old_len = self.array_length(o);
        let Some(v) = d.value else {
            return Ok(match self.validate_descriptor(Some(current), true, d) {
                Outcome::Reject => false,
                Outcome::Keep => true,
                Outcome::Write(p) => {
                    if !p.flags().writable() {
                        self.set_obj_flag(o, OBJ_LENGTH_READONLY, true);
                    }
                    true
                }
            });
        };
        let new_len = match v.as_number() {
            // `as` saturates and maps NaN to 0, so the round trip rejects everything else.
            Some(n) if (n as u32) as f64 == n => n as u32,
            _ => return Err(HeapError::InvalidArrayLength),
        };
        let mut nd = *d;
        nd.value = Some(Value::number(new_len as f64));
        if new_len >= old_len {
            return Ok(match self.validate_descriptor(Some(current), true, &nd) {
                Outcome::Reject => false,
                Outcome::Keep => true,
                Outcome::Write(p) => {
                    self.set_array_length_raw(o, new_len);
                    if !p.flags().writable() {
                        self.set_obj_flag(o, OBJ_LENGTH_READONLY, true);
                    }
                    true
                }
            });
        }
        if !current.flags().writable() {
            return Ok(false);
        }
        let new_writable = d.writable != Some(false);
        nd.writable = Some(true);
        if let Outcome::Reject = self.validate_descriptor(Some(current), true, &nd) {
            return Ok(false);
        }
        let (ok, final_len) = self.truncate_elements(o, new_len);
        self.set_array_length_raw(o, final_len);
        if !new_writable {
            self.set_obj_flag(o, OBJ_LENGTH_READONLY, true);
        }
        Ok(ok)
    }

    /// Deletes every element at or above `new_len`, as ArraySetLength does from the top
    /// down: it stops above the highest non-configurable one. Returns whether everything
    /// was deleted, and the resulting length.
    fn truncate_elements(&mut self, o: Obj, new_len: u32) -> (bool, u32) {
        let mut stop = new_len;
        let mut ok = true;
        if self.obj_flags(o) & OBJ_INDEXED_PROPS != 0 && self.props(o) != 0 {
            let t = self.props(o);
            let mut doomed = Vec::new();
            for i in 0..self.arena().get(t, PropTable::COUNT) {
                let key = self.entry_key(t, i);
                if key.is_null() {
                    continue;
                }
                if let Some(idx) = self.atom_array_index(key)
                    && idx >= new_len
                {
                    if !self.read_entry(t, i).flags().configurable() && idx + 1 > stop {
                        stop = idx + 1;
                        ok = false;
                    }
                    doomed.push((idx, key));
                }
            }
            for (idx, key) in doomed {
                if idx >= stop
                    && let Some(i) = self.table_find(self.props(o), key)
                {
                    self.table_remove(o, i);
                }
            }
        }
        let el = self.elements(o);
        if el != 0 {
            let len = self.value_array_len(el);
            if len > stop {
                let start = ValueArray::VALUES.at(el, stop);
                self.arena_mut().zero(start, (len - stop) * 8);
                self.arena_mut().set(el, ValueArray::LEN, stop);
            }
        }
        (ok, stop)
    }

    /// Array.prototype.pop's moves in one step (0080): for an Array with only dense
    /// elements (no index properties in the table, so none is non-configurable, as in a
    /// frozen array), a writable `length`, its last element present in dense storage and
    /// no index properties on its prototype chain, removes that element, shrinks
    /// `length` by one and returns the element. There `Get`, `DeletePropertyOrThrow` and
    /// `Set(length)` observe nothing else. `None`, with nothing changed, otherwise.
    pub fn array_pop_dense(&mut self, o: Obj) -> Option<Value> {
        if !self.array_can_pop_dense(o) {
            return None;
        }
        let len = self.array_length(o);
        let el = self.elements(o);
        let v = self.value_array_get(el, len - 1);
        // As `truncate_elements`: the slots past the length are zero.
        let at = ValueArray::VALUES.at(el, len - 1);
        self.arena_mut().zero(at, 8);
        self.arena_mut().set(el, ValueArray::LEN, len - 1);
        self.set_array_length_raw(o, len - 1);
        Some(v)
    }

    /// Whether [`Heap::array_pop_dense`] applies to `o`.
    pub fn array_can_pop_dense(&self, o: Obj) -> bool {
        if self.object_class(o) != ObjectClass::Array
            || self.obj_flags(o) & (OBJ_INDEXED_PROPS | OBJ_LENGTH_READONLY) != 0
        {
            return false;
        }
        let len = self.array_length(o);
        if len == 0 || self.dense_len(o) != len {
            return false;
        }
        let mut p = self.get_prototype_of(o);
        while let Some(q) = p {
            if self.dense_len(q) != 0 || self.obj_flags(q) & OBJ_INDEXED_PROPS != 0 {
                return false;
            }
            p = self.get_prototype_of(q);
        }
        !self.value_array_get(self.elements(o), len - 1).is_hole()
    }

    /// `[[Delete]]`: `false` if the property is non-configurable.
    pub fn delete_own_property(&mut self, o: Obj, key: Atom) -> bool {
        if self.object_class(o) == ObjectClass::TypedArray
            && let Some(r) = self.ta_numeric_get(o, key)
        {
            return r.is_none();
        }
        match self.find_own(o, key) {
            Loc::None => true,
            Loc::Dense(i) => {
                let el = self.elements(o);
                self.value_array_set(el, i, Value::HOLE);
                true
            }
            Loc::ArrayLength => false,
            Loc::Table(i) => {
                if !self.read_entry(self.props(o), i).flags().configurable() {
                    return false;
                }
                self.table_remove(o, i);
                true
            }
        }
    }

    /// Removes an own table property even if it is non-configurable. Only for setting up
    /// builtins (a prelude function loses the `prototype` every ordinary function gets).
    pub(crate) fn remove_own_property_unchecked(&mut self, o: Obj, key: Atom) {
        if let Loc::Table(i) = self.find_own(o, key) {
            self.table_remove(o, i);
        }
    }

    /// For an ordinary object whose own properties are exactly its property table entries
    /// in `[[OwnPropertyKeys]]` order (no elements, no array-index keys), the number of
    /// entry slots to pass to [`Heap::plain_entry`]; `None` otherwise. Lets hot loops (JSON
    /// stringify) walk properties without collecting keys and looking each one up.
    pub fn plain_entry_slots(&self, o: Obj) -> Option<u32> {
        if self.object_class(o) != ObjectClass::Ordinary
            || self.obj_flags(o) & OBJ_INDEXED_PROPS != 0
            || self.dense_len(o) != 0
        {
            return None;
        }
        Some(match self.props(o) {
            0 => 0,
            t => self.arena().get(t, PropTable::COUNT),
        })
    }

    /// Entry slot `i` of a plain object (see [`Heap::plain_entry_slots`]): its key and
    /// property, or `None` for a deleted slot. Symbol keys are included; string keys come
    /// in creation order.
    pub fn plain_entry(&self, o: Obj, i: u32) -> Option<(Atom, Property)> {
        let t = self.props(o);
        let key = self.entry_key(t, i);
        if key.is_null() {
            None
        } else {
            Some((key, self.read_entry(t, i)))
        }
    }

    /// Adds an own property under a private name (D1400), whether or not the object is
    /// extensible (PrivateFieldAdd, PrivateMethodOrAccessorAdd). The caller checked that
    /// it is absent.
    pub fn add_private_property(
        &mut self,
        o: Obj,
        key: Atom,
        p: &Property,
    ) -> Result<(), HeapError> {
        debug_assert!(self.is_private_atom(key) && self.find_own(o, key) == Loc::None);
        self.table_insert(o, key, p)
    }

    /// `[[OwnPropertyKeys]]` in spec order: array indexes ascending, then string keys in
    /// creation order, then symbols in creation order.
    pub fn own_keys(&self, o: Obj) -> Vec<Atom> {
        if self.object_class(o) == ObjectClass::TypedArray {
            // The element indexes, then the ordinary keys (never numeric).
            let mut keys = self.ta_keys(o);
            keys.extend(self.ordinary_own_keys(o));
            return keys;
        }
        self.ordinary_own_keys(o)
    }

    fn ordinary_own_keys(&self, o: Obj) -> Vec<Atom> {
        let mut indexes: Vec<(u32, Atom)> = Vec::new();
        let mut strings = Vec::new();
        let mut symbols = Vec::new();
        let el = self.elements(o);
        if el != 0 {
            for i in 0..self.value_array_len(el) {
                if !self.value_array_get(el, i).is_hole() {
                    indexes.push((i, Atom::int(i).expect("dense index fits an int atom")));
                }
            }
        }
        let indexed = self.obj_flags(o) & OBJ_INDEXED_PROPS != 0;
        let mut sort = false;
        if let t @ 1.. = self.props(o) {
            for i in 0..self.arena().get(t, PropTable::COUNT) {
                let key = self.entry_key(t, i);
                if key.is_null() {
                    continue;
                }
                if key.is_symbol() {
                    // Private names are not property keys to the language (D1400).
                    if !self.is_private_atom(key) {
                        symbols.push(key);
                    }
                } else if let Some(idx) = indexed.then(|| self.atom_array_index(key)).flatten() {
                    indexes.push((idx, key));
                    sort = true;
                } else {
                    strings.push(key);
                }
            }
        }
        if sort {
            crate::sort::sort_by(&mut indexes, |a, b| a.0 < b.0);
        }
        let mut out = Vec::with_capacity(indexes.len() + strings.len() + symbols.len() + 1);
        out.extend(indexes.into_iter().map(|(_, k)| k));
        if self.object_class(o) == ObjectClass::Array {
            out.push(atoms::LENGTH);
        }
        out.extend(strings);
        out.extend(symbols);
        out
    }
}

/// Dense elements are addressed by integer atoms, so they stop at the int atom range.
#[inline]
const fn atom_int_max() -> u32 {
    super::atom::MAX_INT_ATOM
}
