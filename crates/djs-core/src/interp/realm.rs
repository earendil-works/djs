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
}

const INTRINSIC_COUNT: u32 = Intrinsic::PreludeScript as u32 + 1;

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
