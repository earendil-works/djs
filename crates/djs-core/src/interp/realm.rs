//! The realm: the global object and the intrinsics, and Error objects.

use alloc::string::String;

use crate::bytecode::intrinsics::INTRINSICS;
use crate::heap::cells::{Realm, Roots};
use crate::heap::{
    Atom, CellKind, Heap, HeapError, ObjectClass, PropFlags, PropertyDescriptor, atoms,
};
use crate::value::{Obj, Value};

/// Intrinsics, by index into `Realm::INTRINSICS`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Intrinsic {
    Global,
    ObjectProto,
    FunctionProto,
    ArrayProto,
    ErrorProto,
    TypeErrorProto,
    RangeErrorProto,
    ReferenceErrorProto,
    SyntaxErrorProto,
    InternalErrorProto,
    /// A preallocated `InternalError: out of memory`, thrown when even the error object
    /// cannot be allocated.
    OutOfMemoryError,
    /// The prelude's `Script` cell (an internal reference), or `undefined`: its frames
    /// are builtins in stack traces.
    PreludeScript,
    EvalErrorProto,
    UriErrorProto,
    AggregateErrorProto,
    BooleanProto,
    NumberProto,
    StringProto,
    SymbolProto,
    /// `Symbol.for`'s registry: a prototype-less object from keys to symbols.
    SymbolRegistry,
    /// `Math.random`'s state: an internal ValueArray of two int32 halves.
    RandomState,
    /// %IteratorPrototype%.
    IteratorProto,
    /// %ArrayIteratorPrototype% ([[Prototype]] = %IteratorPrototype%).
    ArrayIteratorProto,
    /// %StringIteratorPrototype% ([[Prototype]] = %IteratorPrototype%). String Iterators
    /// are Array Iterator objects of kind 3 (D1346).
    StringIteratorProto,
    MapProto,
    SetProto,
    WeakMapProto,
    WeakSetProto,
    /// %MapIteratorPrototype% and %SetIteratorPrototype% ([[Prototype]]
    /// %IteratorPrototype%).
    MapIteratorProto,
    SetIteratorProto,
    /// %Promise.prototype% (an ordinary object).
    PromiseProto,
}

const INTRINSIC_COUNT: u32 = Intrinsic::PromiseProto as u32 + 1;

impl Intrinsic {
    /// The intrinsic prototype of the constructor `name`, if the realm has one: the
    /// constructors installed by the prelude (`%InstallConstructor`) reuse it.
    pub(crate) fn prototype_of(name: &str) -> Option<Intrinsic> {
        Some(match name {
            "Object" => Intrinsic::ObjectProto,
            "Function" => Intrinsic::FunctionProto,
            "Array" => Intrinsic::ArrayProto,
            "Error" => Intrinsic::ErrorProto,
            "TypeError" => Intrinsic::TypeErrorProto,
            "RangeError" => Intrinsic::RangeErrorProto,
            "ReferenceError" => Intrinsic::ReferenceErrorProto,
            "SyntaxError" => Intrinsic::SyntaxErrorProto,
            "EvalError" => Intrinsic::EvalErrorProto,
            "URIError" => Intrinsic::UriErrorProto,
            "AggregateError" => Intrinsic::AggregateErrorProto,
            "InternalError" => Intrinsic::InternalErrorProto,
            "Boolean" => Intrinsic::BooleanProto,
            "Number" => Intrinsic::NumberProto,
            "String" => Intrinsic::StringProto,
            "Symbol" => Intrinsic::SymbolProto,
            "Map" => Intrinsic::MapProto,
            "Set" => Intrinsic::SetProto,
            "WeakMap" => Intrinsic::WeakMapProto,
            "WeakSet" => Intrinsic::WeakSetProto,
            "Promise" => Intrinsic::PromiseProto,
            _ => return None,
        })
    }

    /// `%RealmIntrinsic(name)` of the builtin language: `<Constructor>Prototype`.
    pub(crate) fn by_name(name: &str) -> Option<Intrinsic> {
        Intrinsic::prototype_of(name.strip_suffix("Prototype")?)
    }
}

/// The native error types the VM throws itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorKind {
    Error,
    TypeError,
    RangeError,
    ReferenceError,
    SyntaxError,
    InternalError,
}

impl ErrorKind {
    pub const fn name(self) -> &'static str {
        match self {
            ErrorKind::Error => "Error",
            ErrorKind::TypeError => "TypeError",
            ErrorKind::RangeError => "RangeError",
            ErrorKind::ReferenceError => "ReferenceError",
            ErrorKind::SyntaxError => "SyntaxError",
            ErrorKind::InternalError => "InternalError",
        }
    }

    const fn proto(self) -> Intrinsic {
        match self {
            ErrorKind::Error => Intrinsic::ErrorProto,
            ErrorKind::TypeError => Intrinsic::TypeErrorProto,
            ErrorKind::RangeError => Intrinsic::RangeErrorProto,
            ErrorKind::ReferenceError => Intrinsic::ReferenceErrorProto,
            ErrorKind::SyntaxError => Intrinsic::SyntaxErrorProto,
            ErrorKind::InternalError => Intrinsic::InternalErrorProto,
        }
    }
}

/// The realm cell.
#[inline]
pub(crate) fn realm(heap: &Heap) -> u32 {
    heap.realm().as_internal().expect("no realm")
}

/// An intrinsic of the realm.
#[inline]
pub fn intrinsic(heap: &Heap, i: Intrinsic) -> Value {
    heap.arena()
        .read(Realm::INTRINSICS.at(realm(heap), i as u32))
}

/// An intrinsic that is an object.
pub fn intrinsic_obj(heap: &Heap, i: Intrinsic) -> Obj {
    intrinsic(heap, i).as_object().expect("object intrinsic")
}

/// A non-enumerable data property (how builtins define their properties).
pub(crate) fn define_hidden(heap: &mut Heap, o: Obj, key: Atom, v: Value) -> Result<(), HeapError> {
    let d = PropertyDescriptor::data(v, PropFlags::WRITABLE | PropFlags::CONFIGURABLE);
    heap.define_own_property(o, key, &d)?;
    Ok(())
}

/// Creates the realm: prototypes, error prototypes, the global object.
pub(crate) fn init(heap: &mut Heap) -> Result<(), HeapError> {
    let size = Realm::INTRINSICS
        .cell_size(INTRINSIC_COUNT)
        .ok_or(HeapError::OutOfMemory)?;
    let r = heap.alloc_cell(CellKind::Realm, size)?;
    heap.set_realm(Value::internal(r));
    let set = |heap: &mut Heap, i: Intrinsic, v: Value| {
        heap.arena_mut().write(Realm::INTRINSICS.at(r, i as u32), v)
    };
    let object_proto = heap.new_object(None)?;
    set(heap, Intrinsic::ObjectProto, Value::object(object_proto));
    let function_proto = heap.new_object(Some(object_proto))?;
    set(
        heap,
        Intrinsic::FunctionProto,
        Value::object(function_proto),
    );
    let array_proto = heap.new_array(Some(object_proto), 0)?;
    set(heap, Intrinsic::ArrayProto, Value::object(array_proto));
    let iterator_proto = heap.new_object(Some(object_proto))?;
    set(
        heap,
        Intrinsic::IteratorProto,
        Value::object(iterator_proto),
    );
    let array_iterator_proto = heap.new_object(Some(iterator_proto))?;
    set(
        heap,
        Intrinsic::ArrayIteratorProto,
        Value::object(array_iterator_proto),
    );
    let string_iterator_proto = heap.new_object(Some(iterator_proto))?;
    set(
        heap,
        Intrinsic::StringIteratorProto,
        Value::object(string_iterator_proto),
    );
    for i in [Intrinsic::MapIteratorProto, Intrinsic::SetIteratorProto] {
        let p = heap.new_object(Some(iterator_proto))?;
        set(heap, i, Value::object(p));
    }
    for i in [
        Intrinsic::MapProto,
        Intrinsic::SetProto,
        Intrinsic::WeakMapProto,
        Intrinsic::WeakSetProto,
    ] {
        let p = heap.new_object(Some(object_proto))?;
        set(heap, i, Value::object(p));
    }
    let error_proto = heap.new_object(Some(object_proto))?;
    set(heap, Intrinsic::ErrorProto, Value::object(error_proto));
    let empty = Value::string(heap.atom_string(atoms::EMPTY)?);
    define_hidden(heap, error_proto, atoms::MESSAGE, empty)?;
    let name = heap.alloc_str("Error")?;
    define_hidden(heap, error_proto, atoms::NAME, Value::string(name))?;
    for kind in [
        ErrorKind::TypeError,
        ErrorKind::RangeError,
        ErrorKind::ReferenceError,
        ErrorKind::SyntaxError,
        ErrorKind::InternalError,
    ] {
        let p = heap.new_object(Some(error_proto))?;
        let name = heap.alloc_str(kind.name())?;
        define_hidden(heap, p, atoms::NAME, Value::string(name))?;
        define_hidden(heap, p, atoms::MESSAGE, empty)?;
        set(heap, kind.proto(), Value::object(p));
    }
    for (i, name) in [
        (Intrinsic::EvalErrorProto, "EvalError"),
        (Intrinsic::UriErrorProto, "URIError"),
        (Intrinsic::AggregateErrorProto, "AggregateError"),
    ] {
        let p = heap.new_object(Some(error_proto))?;
        let name = heap.alloc_str(name)?;
        define_hidden(heap, p, atoms::NAME, Value::string(name))?;
        define_hidden(heap, p, atoms::MESSAGE, empty)?;
        set(heap, i, Value::object(p));
    }
    // Boolean.prototype, Number.prototype and String.prototype are wrapper objects
    // themselves (false, +0, "").
    for (i, class, v) in [
        (Intrinsic::BooleanProto, ObjectClass::Boolean, Value::FALSE),
        (Intrinsic::NumberProto, ObjectClass::Number, Value::int(0)),
        (Intrinsic::StringProto, ObjectClass::String, empty),
    ] {
        let p = heap.alloc_object(class, Some(object_proto), 1)?;
        heap.set_object_slot(p, 0, v);
        set(heap, i, Value::object(p));
    }
    let string_proto = intrinsic_obj(heap, Intrinsic::StringProto);
    let d = PropertyDescriptor::data(Value::int(0), PropFlags::NONE);
    heap.define_own_property(string_proto, atoms::LENGTH, &d)?;
    let symbol_proto = heap.new_object(Some(object_proto))?;
    set(heap, Intrinsic::SymbolProto, Value::object(symbol_proto));
    let promise_proto = heap.new_object(Some(object_proto))?;
    set(heap, Intrinsic::PromiseProto, Value::object(promise_proto));
    let registry = heap.new_object(None)?;
    set(heap, Intrinsic::SymbolRegistry, Value::object(registry));
    let random = heap.alloc_value_array(2)?;
    heap.arena_mut()
        .set(random, crate::heap::cells::ValueArray::LEN, 2);
    set(heap, Intrinsic::RandomState, Value::internal(random));
    crate::builtins::number::set_seed(heap, crate::builtins::number::DEFAULT_RANDOM_SEED);
    let global = heap.new_object(Some(object_proto))?;
    set(heap, Intrinsic::Global, Value::object(global));
    define_hidden(heap, global, atoms::GLOBAL_THIS, Value::object(global))?;
    // The value properties of the global object: non-writable, non-configurable.
    for (k, v) in [
        (atoms::NAN, Value::NAN),
        (atoms::INFINITY, Value::number(f64::INFINITY)),
        (atoms::UNDEFINED, Value::UNDEFINED),
    ] {
        heap.define_own_property(global, k, &PropertyDescriptor::data(v, PropFlags::NONE))?;
    }
    // The global lexical record: a hidden object, never visible to JS. Bindings are
    // configurable data properties (so `init_global_lex` can always overwrite them);
    // `writable` is false for `const` (D1220).
    let installed = heap.alloc_value_array(INTRINSICS.len() as u32)?;
    for i in 0..INTRINSICS.len() as u32 {
        heap.value_array_set(installed, i, Value::UNDEFINED);
    }
    heap.arena_mut().set(
        installed,
        crate::heap::cells::ValueArray::LEN,
        INTRINSICS.len() as u32,
    );
    heap.arena_mut()
        .set(r, Realm::INSTALLED, crate::heap::Ref(installed));
    let lex = heap.new_object(None)?;
    heap.arena_mut()
        .set(r, Realm::GLOBAL_LEX, crate::heap::Ref(lex.offset()));
    let oom = new_error(heap, ErrorKind::InternalError, "out of memory")?;
    set(heap, Intrinsic::OutOfMemoryError, oom);
    debug_assert_eq!(heap.arena().get(0, Roots::REALM).as_internal(), Some(r));
    Ok(())
}

/// The JS function installed for intrinsic `id`, or `undefined`.
pub(crate) fn installed_intrinsic(heap: &Heap, id: u16) -> Value {
    let t: crate::heap::Ref = heap.arena().get(realm(heap), Realm::INSTALLED);
    if (id as u32) < heap.value_array_len(t.0) {
        heap.value_array_get(t.0, id as u32)
    } else {
        Value::UNDEFINED
    }
}

pub(crate) fn install_intrinsic(heap: &mut Heap, id: u16, f: Value) {
    let t: crate::heap::Ref = heap.arena().get(realm(heap), Realm::INSTALLED);
    heap.value_array_set(t.0, id as u32, f);
}

/// The global lexical record (Script-mode `let`/`const`, D1220).
pub(crate) fn global_lex(heap: &Heap) -> Obj {
    let r: crate::heap::Ref = heap.arena().get(realm(heap), Realm::GLOBAL_LEX);
    Obj(r.0)
}

/// Declares an uninitialized global lexical binding.
pub(crate) fn declare_global_lexical(
    heap: &mut Heap,
    key: Atom,
    writable: bool,
) -> Result<(), HeapError> {
    let lex = global_lex(heap);
    let d = PropertyDescriptor::data(Value::UNINITIALIZED, PropFlags::new(writable, true, true));
    heap.define_own_property(lex, key, &d)?;
    Ok(())
}

/// Sets the value of a declared global lexical binding.
pub(crate) fn set_global_lexical(heap: &mut Heap, key: Atom, v: Value) -> Result<(), HeapError> {
    let lex = global_lex(heap);
    let d = PropertyDescriptor {
        value: Some(v),
        ..Default::default()
    };
    heap.define_own_property(lex, key, &d)?;
    Ok(())
}

/// ToObject of a primitive (`proto` None: the realm's prototype for its type): a
/// Boolean, Number, String or Symbol wrapper. `None` for undefined and null.
pub(crate) fn wrap_primitive(
    heap: &mut Heap,
    v: Value,
    proto: Option<Obj>,
) -> Result<Option<Obj>, HeapError> {
    let (class, default) = match v.kind() {
        crate::value::Kind::Bool => (ObjectClass::Boolean, Intrinsic::BooleanProto),
        crate::value::Kind::Number => (ObjectClass::Number, Intrinsic::NumberProto),
        crate::value::Kind::String => (ObjectClass::String, Intrinsic::StringProto),
        crate::value::Kind::Symbol => (ObjectClass::Symbol, Intrinsic::SymbolProto),
        _ => return Ok(None),
    };
    let proto = proto.unwrap_or_else(|| intrinsic_obj(heap, default));
    let o = heap.alloc_object(class, Some(proto), 1)?;
    heap.set_object_slot(o, 0, v);
    if let Some(s) = v.as_string() {
        // The String exotic object's own properties: the code units (read-only,
        // enumerable), then `length`.
        let len = heap.str_len(s);
        for i in 0..len {
            let u = heap.char_code_at(s, i).expect("index in range");
            let c = heap.alloc_utf16(&[u])?;
            let key = heap.atom_from_index(i)?;
            let d = PropertyDescriptor::data(Value::string(c), PropFlags::ENUMERABLE);
            heap.define_own_property(o, key, &d)?;
        }
        let d = PropertyDescriptor::data(Value::int(len as i32), PropFlags::NONE);
        heap.define_own_property(o, atoms::LENGTH, &d)?;
    }
    Ok(Some(o))
}

/// The realm prototype that property lookups on a primitive go through.
pub(crate) fn primitive_proto(heap: &Heap, v: Value) -> Option<Obj> {
    let i = match v.kind() {
        crate::value::Kind::Bool => Intrinsic::BooleanProto,
        crate::value::Kind::Number => Intrinsic::NumberProto,
        crate::value::Kind::String => Intrinsic::StringProto,
        crate::value::Kind::Symbol => Intrinsic::SymbolProto,
        _ => return None,
    };
    Some(intrinsic_obj(heap, i))
}

/// A new Error object of `kind` with `message`.
pub fn new_error(heap: &mut Heap, kind: ErrorKind, message: &str) -> Result<Value, HeapError> {
    let proto = intrinsic_obj(heap, kind.proto());
    let e = heap.alloc_object(ObjectClass::Error, Some(proto), 0)?;
    let m = heap.alloc_str(message)?;
    define_hidden(heap, e, atoms::MESSAGE, Value::string(m))?;
    Ok(Value::object(e))
}

/// `name: message` of an Error object (via its prototype chain), for host output. `None`
/// if `v` is not an Error.
pub fn error_summary(heap: &Heap, v: Value) -> Option<String> {
    let o = v.as_object()?;
    if heap.object_class(o) != ObjectClass::Error {
        return None;
    }
    let get = |k: Atom| {
        heap.find_property(o, k)
            .and_then(|(_, p)| p.value())
            .and_then(|v| v.as_string())
            .map(|s| heap.str_to_string(s))
            .unwrap_or_default()
    };
    let name = get(atoms::NAME);
    let message = get(atoms::MESSAGE);
    Some(if message.is_empty() {
        name
    } else {
        alloc::format!("{name}: {message}")
    })
}
