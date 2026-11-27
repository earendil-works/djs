//! What a snapshot knows about the heap's records by name: the object classes and their
//! internal slots, the header flags, and how the generic cell kinds of `heap/cells.rs`
//! are read. Everything here is the snapshot's *contract*: a name in these tables is what
//! must keep its meaning for a snapshot to load into another build.

use crate::heap::cells::flags;
use crate::heap::{CellKind, ObjectClass};

/// How a slot's value is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SlotTy {
    /// A value (immediates, references to records, builtins by path).
    Value,
    /// An int32 intrinsic id: written as the intrinsic's name.
    Intrinsic,
    /// A Map/Set hash index: written as its bucket count, rebuilt on import.
    Buckets,
    /// A RegExp's compiled program: not written, compiled again from source and flags.
    Program,
}

impl SlotTy {
    pub fn name(self) -> &'static str {
        match self {
            SlotTy::Value => "value",
            SlotTy::Intrinsic => "intrinsic",
            SlotTy::Buckets => "buckets",
            SlotTy::Program => "program",
        }
    }
}

pub(super) struct ClassSpec {
    pub class: ObjectClass,
    pub name: &'static str,
    /// The fixed slots, by name.
    pub slots: &'static [(&'static str, SlotTy)],
    /// The name of the variable-length rest (closures' captures, records' fields).
    pub rest: Option<&'static str>,
}

use SlotTy::*;

macro_rules! classes {
    ($($class:ident $name:literal [$($slot:literal : $ty:ident),*] $(rest $rest:literal)?;)*) => {
        pub(super) const CLASSES: &[ClassSpec] = &[$(ClassSpec {
            class: ObjectClass::$class,
            name: $name,
            slots: &[$(($slot, $ty)),*],
            rest: classes!(@rest $($rest)?),
        }),*];
    };
    (@rest) => { None };
    (@rest $r:literal) => { Some($r) };
}

// The classes a snapshot can hold, with their slots named as `heap/object.rs` documents
// them. A class not listed refuses the snapshot.
classes! {
    Ordinary "Ordinary" [];
    Array "Array" ["length": Value];
    Function "Function" [] rest "captures";
    HostFunction "HostFunction" ["id": Value];
    Error "Error" ["stack": Value] rest "more";
    Arguments "Arguments" [];
    ArrayIterator "ArrayIterator" ["target": Value, "index": Value, "kind": Value];
    BoundFunction "BoundFunction" ["target": Value, "this": Value, "args": Value];
    Boolean "Boolean" ["primitive": Value];
    Number "Number" ["primitive": Value];
    String "String" ["primitive": Value];
    Symbol "Symbol" ["primitive": Value];
    BigInt "BigInt" ["primitive": Value];
    Map "Map" ["entries": Value, "index": Buckets, "live": Value];
    Set "Set" ["entries": Value, "index": Buckets, "live": Value];
    WeakMap "WeakMap" ["entries": Value, "index": Buckets, "live": Value];
    WeakSet "WeakSet" ["entries": Value, "index": Buckets, "live": Value];
    CollectionIterator "CollectionIterator" ["entries": Value, "next": Value, "kind": Value, "isSet": Value, "key": Value, "value": Value];
    Promise "Promise" ["state": Value, "result": Value, "reactions": Value, "flags": Value];
    BuiltinClosure "BuiltinClosure" ["builtin": Intrinsic, "data": Value, "length": Value];
    Record "Record" [] rest "fields";
    Date "Date" ["time": Value];
    Generator "Generator" ["state": Value, "frame": Value, "pc": Value, "hp": Value, "promise": Value, "kind": Value, "queue": Value];
    RegExp "RegExp" ["program": Program, "source": Value, "flags": Value];
    RegExpStringIterator "RegExpStringIterator" ["regexp": Value, "string": Value, "flags": Value];
    Proxy "Proxy" ["target": Value, "handler": Value, "flags": Value];
    IteratorHelper "IteratorHelper" [] rest "fields";
    RawJSON "RawJSON" [];
    CallSite "CallSite" ["function": Value, "name": Value, "file": Value, "line": Value, "column": Value, "native": Value, "construct": Value];
    DisposableStack "DisposableStack" ["async": Value, "state": Value, "resources": Value];
    ArrayBuffer "ArrayBuffer" [] rest "slots";
    TypedArray "TypedArray" [] rest "slots";
    DataView "DataView" [] rest "slots";
}

pub(super) fn class_spec(c: ObjectClass) -> Option<&'static ClassSpec> {
    CLASSES.iter().find(|s| s.class == c)
}

pub(super) fn class_by_name(n: &str) -> Option<&'static ClassSpec> {
    CLASSES.iter().find(|s| s.name == n)
}

/// The object header flags a snapshot keeps, by name. The others are caches or follow
/// from the storage (`OBJ_INDEXED_PROPS`, `OBJ_IS_PROTO`, `OBJ_DICTIONARY`).
pub(super) const OBJECT_FLAGS: &[(&str, u16)] = &[
    ("extensible", flags::OBJ_EXTENSIBLE),
    ("lengthReadonly", flags::OBJ_LENGTH_READONLY),
    ("builtin", flags::OBJ_BUILTIN),
    ("builtinCtor", flags::OBJ_BUILTIN_CTOR),
    ("hostAsync", flags::OBJ_HOST_ASYNC),
    ("lazyLength", flags::OBJ_LAZY_LENGTH),
    ("lazyName", flags::OBJ_LAZY_NAME),
];

/// A generic cell kind: written field by field from its layout (`heap/cells.rs`), by the
/// fields' names.
pub(super) struct KindSpec {
    pub kind: CellKind,
    /// The field that counts the tail elements in use (the rest is spare capacity).
    pub count: Option<&'static str>,
    /// Fields that are not state (an atom id the import assigns afresh).
    pub skip: &'static [&'static str],
    /// The header flags, by name.
    pub flags: &'static [(&'static str, u16)],
}

pub(super) const KINDS: &[KindSpec] = &[
    KindSpec {
        kind: CellKind::ValueArray,
        count: Some("LEN"),
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::EphemeronArray,
        count: Some("LEN"),
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::U32Array,
        count: Some("LEN"),
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::Bytes,
        count: Some("LEN"),
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::BoxCell,
        count: None,
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::HostCall,
        count: None,
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::HandleTable,
        count: Some("LEN"),
        skip: &[],
        flags: &[],
    },
    KindSpec {
        kind: CellKind::BigInt,
        count: Some("LEN"),
        skip: &[],
        flags: &[("negative", flags::BIGINT_NEG)],
    },
    KindSpec {
        kind: CellKind::Symbol,
        count: None,
        skip: &["ATOM"],
        flags: &[
            ("private", flags::SYM_PRIVATE),
            ("privateMethod", flags::SYM_PRIVATE_METHOD),
            ("privateReadonly", flags::SYM_PRIVATE_READONLY),
        ],
    },
];

pub(super) fn kind_spec(k: CellKind) -> Option<&'static KindSpec> {
    KINDS.iter().find(|s| s.kind == k)
}
