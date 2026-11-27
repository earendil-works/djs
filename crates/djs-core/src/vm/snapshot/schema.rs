//! What a snapshot knows about the heap's records by name: the object classes and their
//! internal slots, the header flags, and how the generic cell kinds of `heap/cells.rs`
//! are read. Everything here is the snapshot's *contract*: a name in these tables is what
//! must keep its meaning for a snapshot to load into another build.

use alloc::string::String;

use crate::heap::cells::flags;
use crate::heap::{CellKind, ObjectClass, RecordSchema};

/// The object classes' slots: heap/object.rs's table (`CLASSES`, D3700).
pub(super) use crate::heap::{ClassSlots as ClassSpec, SlotKind as SlotTy};

/// A slot kind's name in a record type.
pub(super) fn slot_ty_name(t: SlotTy) -> String {
    String::from(match t {
        SlotTy::Value => "value",
        SlotTy::Intrinsic => "intrinsic",
        SlotTy::Buckets => "buckets",
        SlotTy::Program => "program",
        SlotTy::Enum(c) => return alloc::format!("enum:{}", c.name),
        SlotTy::Bits(c) => return alloc::format!("bits:{}", c.name),
    })
}

/// Classes whose objects carry a record kind (D3710): their slots are the kind's
/// schema's, not the class's.
pub(super) fn is_kinded(c: ObjectClass) -> bool {
    matches!(c, ObjectClass::Record | ObjectClass::IteratorHelper)
}

/// This build's record schema `name` (of class `class`).
pub(super) fn record_by_name(class: ObjectClass, name: &str) -> Option<&'static RecordSchema> {
    crate::builtins::r#gen::records::RECORDS
        .iter()
        .find(|r| r.name == name && r.class == class)
}

/// Classes a snapshot does not hold yet: weak references, whose targets the snapshot
/// would have to hold weakly too.
const REFUSED: &[ObjectClass] = &[ObjectClass::WeakRef, ObjectClass::FinalizationRegistry];

pub(super) fn class_spec(c: ObjectClass) -> Option<&'static ClassSpec> {
    (!REFUSED.contains(&c)).then(|| c.slot_table())
}

pub(super) fn class_by_name(n: &str) -> Option<&'static ClassSpec> {
    crate::heap::CLASSES
        .iter()
        .find(|s| s.name == n && !REFUSED.contains(&s.class))
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
