//! The layout description: every cell kind, declared once.
//!
//! # Cell header
//!
//! Every cell starts at an 8-aligned offset with an 8-byte header:
//!
//! ```text
//! byte 0     kind      CellKind (0 is invalid)
//! byte 1     gc        GC bits (see `gc_bits`); 0 outside a collection
//! bytes 2-3  flags     u16, meaning depends on the kind (see `flags`)
//! bytes 4-7  size      u32, total cell size in bytes, header included, a multiple of 8
//! ```
//!
//! Cells are laid out back to back, so walking the arena from offset 0 by `size` visits
//! every cell. Offset 0 holds the [`Roots`] cell, which is how offset 0 doubles as null:
//! nothing ever refers to the roots cell.
//!
//! Field offsets below are relative to the cell start and include the header.
//!
//! # Adding a cell kind
//!
//! 1. Declare it in `cells!` below with the next free kind id. Give every field that holds
//!    a reference the type [`Value`] or [`Ref`], never a plain `u32`: those two types *are*
//!    the trace description. The GC (`gc.rs`), the walker and `verify` then handle the kind
//!    automatically, including a tail of values or of a record declared in `records!`.
//! 2. Zero bytes must be a valid initial state for tracing (they are, for `Value` and
//!    `Ref`). Do not cache arena offsets anywhere else: tables keyed by offsets would need
//!    rehashing after every collection.
//! 3. Optionally add kind-specific checks to `walk::cell_invariants`, for anything the heap
//!    code relies on to terminate (counts, ref target kinds).
//!
//! The layout hash changes with any edit here, which invalidates old heap images.

use super::layout::{Field, Ref, cells, records};
use crate::value::Value;

/// The common header fields.
pub struct Header;

impl Header {
    pub const KIND: Field<u8> = Field::new(0);
    pub const GC: Field<u8> = Field::new(1);
    pub const FLAGS: Field<u16> = Field::new(2);
    pub const SIZE: Field<u32> = Field::new(4);
}

/// GC bits in header byte 1. Reserved for the collector; always 0 between collections.
pub mod gc_bits {
    /// Mark bit (for a future mark phase over the old generation).
    pub const MARK: u8 = 1;
    /// The cell has been copied; the new offset is stored in the first payload word.
    pub const FORWARDED: u8 = 2;
    /// The cell belongs to the non-moving old generation (ARCH "Deltas").
    pub const OLD: u8 = 4;
}

/// Per-kind header flags.
pub mod flags {
    /// Rope: the content has a code unit above 0xFF (some leaf is a [`Str16`](super::Str16)).
    pub const ROPE_TWO_BYTE: u16 = 1;
    /// Rope: flattened. `LEFT` points to the flat string, `RIGHT` is null.
    pub const ROPE_FLATTENED: u16 = 2;
    /// Object: new properties may be added.
    pub const OBJ_EXTENSIBLE: u16 = 1;
    /// Object: the property table holds array-index keys (or the class has exotic
    /// integer-keyed behavior). Element fast paths must then fall back.
    pub const OBJ_INDEXED_PROPS: u16 = 2;
    /// Array: `length` is not writable.
    pub const OBJ_LENGTH_READONLY: u16 = 4;
}

/// `"DJS\0"` little-endian, in [`Roots::MAGIC`].
pub const MAGIC: u32 = u32::from_le_bytes(*b"DJS\0");

records! {
    /// One property in a [`PropTable`]. For a data property `VALUE` is the value and
    /// `SETTER` is unused (0). For an accessor `VALUE` is the getter and `SETTER` the
    /// setter (`undefined` when absent). A deleted entry has `KEY` 0 and zeroed slots.
    PropEntry[24] {
        /// The key: an [`Atom`](super::Atom) id. 0 marks a deleted entry.
        KEY: u32 = 0,
        /// [`PropFlags`](super::PropFlags) bits.
        FLAGS: u32 = 4,
        VALUE: Value = 8,
        SETTER: Value = 16,
    }

    /// One slot of the [`HandleTable`]. A live slot has `NEXT` = [`HANDLE_LIVE`]; a free
    /// slot holds `undefined` and links to the next free slot (`index + 1`, 0 = end).
    HandleSlot[16] {
        VALUE: Value = 0,
        NEXT: u32 = 8,
    }
}

/// [`HandleSlot::NEXT`] of a live handle.
pub const HANDLE_LIVE: u32 = u32::MAX;

cells! {
    /// The root set, always at offset 0. Everything live is reachable from here. The GC
    /// copies this cell first and then scans the to-space from offset 0, so scanning the
    /// roots and scanning the heap are the same loop.
    1 => Roots {
        /// [`MAGIC`].
        MAGIC: u32 = 8,
        /// [`LAYOUT_HASH`] of the build that created the heap.
        VERSION: u32 = 12,
        /// The [`AtomTable`].
        ATOMS: Ref = 16,
        /// The value stack area (interpreter, issue 0006). Null for now.
        STACK: Ref = 20,
        /// The realm record (global object, intrinsics). `undefined` until a realm exists.
        REALM: Value = 24,
        /// The microtask queue. Null for now.
        JOBS: Ref = 32,
        /// The pending host call table. Null for now.
        HOST_CALLS: Ref = 36,
        /// The [`HandleTable`] (values the embedder or Rust code hold across a GC). Null
        /// until the first handle is created.
        HANDLES: Ref = 40,
        /// Reserved, 0.
        RESERVED: u32 = 44,
    };

    /// A flat one-byte (Latin-1) string. Holds only strings with every code unit <= 0xFF;
    /// constructors narrow, so a string that fits here is never a [`Str16`].
    2 => Str8 {
        /// Length in code units (= bytes).
        LEN: u32 = 8,
        /// Cached string hash; 0 means not computed yet.
        HASH: u32 = 12,
    } tail DATA: [u8] = 16;

    /// A flat two-byte (UTF-16LE) string. Always contains a code unit above 0xFF.
    3 => Str16 {
        /// Length in code units.
        LEN: u32 = 8,
        /// Cached string hash; 0 means not computed yet.
        HASH: u32 = 12,
    } tail DATA: [u16] = 16;

    /// A concatenation of two strings, flattened lazily. Header flags:
    /// [`flags::ROPE_TWO_BYTE`], [`flags::ROPE_FLATTENED`].
    4 => Rope {
        /// Length in code units: `len(LEFT) + len(RIGHT)`.
        LEN: u32 = 8,
        /// Cached string hash; 0 means not computed yet.
        HASH: u32 = 12,
        /// The left string, or the flat result once flattened.
        LEFT: Ref = 16,
        /// The right string; null once flattened.
        RIGHT: Ref = 20,
    };

    /// A symbol. Every symbol is also an atom, so it can be a property key.
    5 => Symbol {
        /// The symbol's [`Atom`](super::Atom) id.
        ATOM: u32 = 8,
        /// A string, or `undefined`.
        DESCRIPTION: Value = 16,
    };

    /// An object. Header flags: [`flags::OBJ_EXTENSIBLE`], [`flags::OBJ_INDEXED_PROPS`],
    /// [`flags::OBJ_LENGTH_READONLY`].
    6 => Object {
        /// [`ObjectClass`](super::ObjectClass).
        CLASS: u32 = 8,
        /// The prototype object, or null.
        PROTO: Ref = 12,
        /// The [`PropTable`], or null when there are no properties.
        PROPS: Ref = 16,
        /// The dense elements: a [`ValueArray`] (`LEN` = dense length; slots may be
        /// holes), or null.
        ELEMENTS: Ref = 20,
    } tail
        /// Internal slots, class specific. Arrays: slot 0 is the length (a Number).
        SLOTS: [Value] = 24;

    /// Property storage. Small tables are searched linearly; above
    /// `object::LINEAR_MAX` entries a hash index ([`U32Array`]) is built.
    7 => PropTable {
        /// Entries in use, deleted ones included. Entries are in insertion order.
        COUNT: u32 = 8,
        /// Deleted entries (only in hashed mode; linear mode compacts on delete).
        DELETED: u32 = 12,
        /// Hash index: buckets holding `entry + 1`, 0 empty, `u32::MAX` deleted. Null in
        /// linear mode.
        INDEX: Ref = 16,
    } tail ENTRIES: [PropEntry] = 24;

    /// A growable array of values (elements, the atom table, later the value stack).
    /// Slots at and beyond `LEN` are 0.
    8 => ValueArray {
        /// Slots in use.
        LEN: u32 = 8,
    } tail VALUES: [Value] = 16;

    /// A fixed array of `u32` (hash indexes). Not traced.
    9 => U32Array {
        LEN: u32 = 8,
    } tail DATA: [u32] = 12;

    /// The atom table: atom index -> string or symbol, plus the intern hash index.
    10 => AtomTable {
        /// Number of string atoms (for the index load factor).
        STRINGS: u32 = 8,
        /// [`ValueArray`] indexed by atom index; slot 0 is unused.
        ENTRIES: Ref = 12,
        /// [`U32Array`] of buckets holding string atom indexes (0 = empty).
        INDEX: Ref = 16,
    };

    /// The host handle table: values held by the embedder or by Rust code across a
    /// collection. Grown by copying.
    11 => HandleTable {
        /// Slots ever used (live or free); slots at and beyond are 0.
        LEN: u32 = 8,
        /// Head of the free list: `index + 1`, 0 = empty.
        FREE: u32 = 12,
        /// Live handles.
        LIVE: u32 = 16,
    } tail SLOTS: [HandleSlot] = 24;
}

// The GC stores the forwarding offset in the first payload word, so every cell needs one.
const _: () = {
    let mut i = 0;
    while i < LAYOUTS.len() {
        assert!(
            LAYOUTS[i].min_size >= 16,
            "layout: cells need a payload word"
        );
        i += 1;
    }
};

/// Hash of the layout table and the well-known atom list. Stored in [`Roots::VERSION`];
/// part of the snapshot build hash.
pub const LAYOUT_HASH: u32 =
    super::layout::hash_layouts(LAYOUTS, super::atom::WELL_KNOWN_NAMES_FOR_HASH);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::layout::FieldTy;

    #[test]
    fn kinds_match_table() {
        for (i, l) in LAYOUTS.iter().enumerate() {
            assert_eq!(l.kind as usize, i + 1);
            let k = CellKind::from_u8(l.kind).unwrap();
            assert_eq!(k.layout().name, l.name);
        }
        assert_eq!(CellKind::from_u8(0), None);
        assert_eq!(CellKind::from_u8(LAYOUTS.len() as u8 + 1), None);
    }

    #[test]
    fn generated_layouts() {
        assert_eq!(Str8::LEN.offset, 8);
        assert_eq!(Str8::DATA.offset, 16);
        assert_eq!(Str8::LAYOUT.min_size, 16);
        assert_eq!(Rope::LAYOUT.min_size, 24);
        assert_eq!(Symbol::LAYOUT.min_size, 24);
        assert_eq!(Object::LAYOUT.min_size, 24);
        assert_eq!(Roots::LAYOUT.min_size, 48);
        let t = PropTable::LAYOUT.tail.unwrap();
        assert_eq!(t.elem.size, 24);
        assert_eq!(t.elem.name, "PropEntry");
        let traced: alloc::vec::Vec<_> = t
            .elem
            .fields
            .iter()
            .filter(|f| f.ty.is_traced())
            .map(|f| f.name)
            .collect();
        assert_eq!(traced, ["VALUE", "SETTER"]);
        assert_eq!(Rope::LAYOUT.fields[2].ty, FieldTy::Ref);
        assert_eq!(PropTable::ENTRIES.cell_size(2), Some(72));
        assert_eq!(Str16::DATA.cell_size(3), Some(24));
        assert_eq!(Str8::DATA.cell_size(u32::MAX), None);
        assert_eq!(Object::LAYOUT.tail_capacity(40), 2);
        assert_eq!(U32Array::LAYOUT.tail_capacity(16), 1);
    }

    #[test]
    fn layout_hash_is_stable_within_build() {
        assert_ne!(LAYOUT_HASH, 0);
        assert_eq!(
            LAYOUT_HASH,
            super::super::layout::hash_layouts(
                LAYOUTS,
                super::super::atom::WELL_KNOWN_NAMES_FOR_HASH
            )
        );
    }
}
