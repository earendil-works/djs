//! The layout description machinery.
//!
//! Every cell kind is declared once, in `cells.rs`, with the `cells!` macro: its kind
//! id, its fixed fields (name, type, byte offset) and an optional variable-length tail.
//! From that one declaration the macro generates:
//!
//! - typed field handles (`Str8::LEN: Field<u32>`) used with [`Arena::get`] and
//!   [`Arena::set`](super::Arena::set),
//! - a const [`CellLayout`] per kind and the table [`LAYOUTS`](super::cells::LAYOUTS) of
//!   all of them, which is what the GC uses to trace cells, what the JS-side reader will be
//!   generated from, and what [`LAYOUT_HASH`](super::cells::LAYOUT_HASH) (part of the
//!   snapshot build hash) is computed from,
//! - compile-time checks: fields start after the header, are naturally aligned, do not
//!   overlap, and the tail starts after the fixed part.
//!
//! Tails are arrays of a primitive type or of a *record* (a fixed group of fields declared
//! with `records!`). The number of elements a tail can hold follows from the cell size in
//! the header. How many are in use is up to the cell kind (usually a `LEN` field).
//!
//! Traced field types are [`Value`] (traced when it is a reference) and [`Ref`] (traced
//! when nonzero). All-zero bytes are a valid, non-reference state for both, so
//! zero-filled memory is always safe to trace.
//!
//! [`Arena::get`]: super::Arena::get

use core::marker::PhantomData;

use crate::value::Value;

/// A raw, untyped reference to a cell: its arena offset, or 0 for null.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(transparent)]
pub struct Ref(pub u32);

impl Ref {
    pub const NULL: Ref = Ref(0);

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// `None` for null.
    #[inline]
    pub const fn get(self) -> Option<u32> {
        if self.0 == 0 { None } else { Some(self.0) }
    }
}

/// The storage type of a field or tail element.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldTy {
    U8,
    U16,
    U32,
    I32,
    U64,
    F64,
    /// A NaN-boxed [`Value`]; traced when it is a reference.
    Value,
    /// A [`Ref`]; traced when nonzero.
    Ref,
}

impl FieldTy {
    pub const fn size(self) -> u32 {
        match self {
            FieldTy::U8 => 1,
            FieldTy::U16 => 2,
            FieldTy::U32 | FieldTy::I32 | FieldTy::Ref => 4,
            FieldTy::U64 | FieldTy::F64 | FieldTy::Value => 8,
        }
    }

    /// Natural alignment, equal to the size. Fields must be naturally aligned so the JS
    /// side can read them through typed-array views.
    pub const fn align(self) -> u32 {
        self.size()
    }

    /// Whether the GC must look at this field.
    pub const fn is_traced(self) -> bool {
        matches!(self, FieldTy::Value | FieldTy::Ref)
    }

    /// A stable small number, used for the layout hash.
    const fn code(self) -> u32 {
        match self {
            FieldTy::U8 => 1,
            FieldTy::U16 => 2,
            FieldTy::U32 => 3,
            FieldTy::I32 => 4,
            FieldTy::U64 => 5,
            FieldTy::F64 => 6,
            FieldTy::Value => 7,
            FieldTy::Ref => 8,
        }
    }
}

/// One field of a cell or record.
#[derive(Clone, Copy, Debug)]
pub struct FieldInfo {
    pub name: &'static str,
    pub offset: u32,
    pub ty: FieldTy,
}

/// The element type of a tail: a primitive (one unnamed field at offset 0) or a record.
#[derive(Clone, Copy, Debug)]
pub struct ElemInfo {
    pub name: &'static str,
    pub size: u32,
    pub fields: &'static [FieldInfo],
}

/// The variable-length tail of a cell.
#[derive(Clone, Copy, Debug)]
pub struct TailInfo {
    pub name: &'static str,
    pub offset: u32,
    pub elem: ElemInfo,
}

/// The layout of one cell kind.
#[derive(Clone, Copy, Debug)]
pub struct CellLayout {
    pub name: &'static str,
    pub kind: u8,
    pub fields: &'static [FieldInfo],
    pub tail: Option<TailInfo>,
    /// The smallest valid cell size: the end of the fixed fields, or the tail offset,
    /// rounded up to 8.
    pub min_size: u32,
}

impl CellLayout {
    pub const fn new(
        name: &'static str,
        kind: u8,
        fields: &'static [FieldInfo],
        tail: Option<TailInfo>,
    ) -> CellLayout {
        let mut end = super::HEADER_SIZE;
        let mut i = 0;
        while i < fields.len() {
            let e = fields[i].offset + fields[i].ty.size();
            if e > end {
                end = e;
            }
            i += 1;
        }
        if let Some(t) = &tail
            && t.offset > end
        {
            end = t.offset;
        }
        CellLayout {
            name,
            kind,
            fields,
            tail,
            min_size: super::align8(end),
        }
    }

    /// Number of tail elements a cell of `size` bytes can hold.
    pub const fn tail_capacity(&self, size: u32) -> u32 {
        match &self.tail {
            Some(t) if size > t.offset => (size - t.offset) / t.elem.size,
            _ => 0,
        }
    }
}

/// A Rust type that can be stored in a field.
pub trait FieldType: Copy {
    const TY: FieldTy;
    fn read(bytes: &[u8], off: usize) -> Self;
    fn write(bytes: &mut [u8], off: usize, v: Self);
}

#[inline]
fn read_array<const N: usize>(bytes: &[u8], off: usize) -> [u8; N] {
    let mut a = [0; N];
    a.copy_from_slice(&bytes[off..off + N]);
    a
}

macro_rules! prim_field {
    ($($t:ty => $ty:ident, $n:literal;)*) => {$(
        impl FieldType for $t {
            const TY: FieldTy = FieldTy::$ty;
            #[inline]
            fn read(bytes: &[u8], off: usize) -> Self {
                <$t>::from_le_bytes(read_array::<$n>(bytes, off))
            }
            #[inline]
            fn write(bytes: &mut [u8], off: usize, v: Self) {
                bytes[off..off + $n].copy_from_slice(&v.to_le_bytes());
            }
        }
        impl Elem for $t {
            const INFO: ElemInfo = ElemInfo {
                name: stringify!($t),
                size: $n,
                fields: &[FieldInfo { name: "", offset: 0, ty: FieldTy::$ty }],
            };
        }
    )*};
}

prim_field! {
    u8 => U8, 1;
    u16 => U16, 2;
    u32 => U32, 4;
    i32 => I32, 4;
    u64 => U64, 8;
    f64 => F64, 8;
}

impl FieldType for Value {
    const TY: FieldTy = FieldTy::Value;
    #[inline]
    fn read(bytes: &[u8], off: usize) -> Self {
        Value::from_raw(u64::from_le_bytes(read_array::<8>(bytes, off)))
    }
    #[inline]
    fn write(bytes: &mut [u8], off: usize, v: Self) {
        bytes[off..off + 8].copy_from_slice(&v.to_bits().to_le_bytes());
    }
}

impl Elem for Value {
    const INFO: ElemInfo = ElemInfo {
        name: "Value",
        size: 8,
        fields: &[FieldInfo {
            name: "",
            offset: 0,
            ty: FieldTy::Value,
        }],
    };
}

impl FieldType for Ref {
    const TY: FieldTy = FieldTy::Ref;
    #[inline]
    fn read(bytes: &[u8], off: usize) -> Self {
        Ref(u32::from_le_bytes(read_array::<4>(bytes, off)))
    }
    #[inline]
    fn write(bytes: &mut [u8], off: usize, v: Self) {
        bytes[off..off + 4].copy_from_slice(&v.0.to_le_bytes());
    }
}

impl Elem for Ref {
    const INFO: ElemInfo = ElemInfo {
        name: "Ref",
        size: 4,
        fields: &[FieldInfo {
            name: "",
            offset: 0,
            ty: FieldTy::Ref,
        }],
    };
}

/// A type that can be a tail element.
pub trait Elem {
    const INFO: ElemInfo;
}

/// A typed field handle: a byte offset relative to a cell (or record) start.
pub struct Field<T> {
    pub offset: u32,
    _t: PhantomData<fn() -> T>,
}

impl<T> Field<T> {
    pub const fn new(offset: u32) -> Field<T> {
        Field {
            offset,
            _t: PhantomData,
        }
    }
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Field<T> {}

/// A typed tail handle.
pub struct Tail<T> {
    pub offset: u32,
    _t: PhantomData<fn() -> T>,
}

impl<T: Elem> Tail<T> {
    pub const fn new(offset: u32) -> Tail<T> {
        Tail {
            offset,
            _t: PhantomData,
        }
    }

    /// The element size in bytes.
    pub const fn elem_size(self) -> u32 {
        T::INFO.size
    }

    /// The arena offset of element `i` of the tail of the cell at `cell`.
    #[inline]
    pub const fn at(self, cell: u32, i: u32) -> u32 {
        cell + self.offset + i * T::INFO.size
    }

    /// The cell size (rounded up to 8) needed for `n` elements.
    pub const fn cell_size(self, n: u32) -> Option<u32> {
        match n.checked_mul(T::INFO.size) {
            Some(b) => match b.checked_add(self.offset + 7) {
                Some(s) => Some(s & !7),
                None => None,
            },
            None => None,
        }
    }
}

impl<T> Clone for Tail<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Tail<T> {}

/// Compile-time check of a group of fields: naturally aligned, sorted, non-overlapping,
/// starting at `start` and ending at or before `end`.
const fn validate_fields(fields: &[FieldInfo], start: u32, end: u32) {
    let mut pos = start;
    let mut i = 0;
    while i < fields.len() {
        let f = &fields[i];
        assert!(f.offset >= pos, "layout: field overlaps or is out of order");
        assert!(
            f.offset.is_multiple_of(f.ty.align()),
            "layout: field is not naturally aligned"
        );
        pos = f.offset + f.ty.size();
        i += 1;
    }
    assert!(pos <= end, "layout: field extends past its end");
}

const fn elem_align(e: &ElemInfo) -> u32 {
    let mut a = 1;
    let mut i = 0;
    while i < e.fields.len() {
        if e.fields[i].ty.align() > a {
            a = e.fields[i].ty.align();
        }
        i += 1;
    }
    a
}

/// Compile-time check of a record.
pub const fn validate_elem(e: &ElemInfo) {
    validate_fields(e.fields, 0, e.size);
    assert!(
        e.size.is_multiple_of(elem_align(e)),
        "layout: record size is not a multiple of its alignment"
    );
}

/// Compile-time check of a cell layout.
pub const fn validate_cell(l: &CellLayout) {
    assert!(l.kind != 0, "layout: kind 0 is reserved");
    let end = match &l.tail {
        Some(t) => t.offset,
        None => u32::MAX,
    };
    validate_fields(l.fields, super::HEADER_SIZE, end);
    if let Some(t) = &l.tail {
        assert!(
            t.offset >= super::HEADER_SIZE,
            "layout: tail overlaps header"
        );
        validate_elem(&t.elem);
        assert!(
            t.offset.is_multiple_of(elem_align(&t.elem)),
            "layout: tail is not aligned"
        );
    }
}

const fn fnv_u32(mut h: u32, v: u32) -> u32 {
    let b = v.to_le_bytes();
    let mut i = 0;
    while i < 4 {
        h = (h ^ b[i] as u32).wrapping_mul(0x0100_0193);
        i += 1;
    }
    h
}

const fn fnv_str(mut h: u32, s: &str) -> u32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        h = (h ^ b[i] as u32).wrapping_mul(0x0100_0193);
        i += 1;
    }
    fnv_u32(h, b.len() as u32)
}

const fn hash_fields(mut h: u32, fields: &[FieldInfo]) -> u32 {
    let mut i = 0;
    while i < fields.len() {
        h = fnv_str(h, fields[i].name);
        h = fnv_u32(h, fields[i].offset);
        h = fnv_u32(h, fields[i].ty.code());
        i += 1;
    }
    fnv_u32(h, fields.len() as u32)
}

/// FNV-1a over every name, offset and type in the layout table, plus `extra` strings
/// (the well-known atom list, whose order fixes atom ids).
pub const fn hash_layouts(layouts: &[CellLayout], extra: &[&str]) -> u32 {
    let mut h = 0x811C_9DC5;
    let mut i = 0;
    while i < layouts.len() {
        let l = &layouts[i];
        h = fnv_str(h, l.name);
        h = fnv_u32(h, l.kind as u32);
        h = hash_fields(h, l.fields);
        if let Some(t) = &l.tail {
            h = fnv_str(h, t.name);
            h = fnv_u32(h, t.offset);
            h = fnv_str(h, t.elem.name);
            h = fnv_u32(h, t.elem.size);
            h = hash_fields(h, t.elem.fields);
        }
        i += 1;
    }
    let mut j = 0;
    while j < extra.len() {
        h = fnv_str(h, extra[j]);
        j += 1;
    }
    h
}

/// `opt!()` is `None`, `opt!(e)` is `Some(e)`. For optional macro groups.
macro_rules! opt {
    () => {
        None
    };
    ($e:expr) => {
        Some($e)
    };
}

/// Declares records: fixed groups of fields used as tail elements.
///
/// ```ignore
/// records! {
///     /// Docs.
///     PropEntry[24] { KEY: u32 = 0, FLAGS: u32 = 4, VALUE: Value = 8, SETTER: Value = 16, }
/// }
/// ```
macro_rules! records {
    ($(
        $(#[$meta:meta])*
        $name:ident [$size:literal] {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ty = $off:literal, )*
        }
    )*) => {$(
        $(#[$meta])*
        pub struct $name;

        impl $name {
            $( $(#[$fmeta])* pub const $field: $crate::heap::layout::Field<$fty> =
                $crate::heap::layout::Field::new($off); )*
        }

        impl $crate::heap::layout::Elem for $name {
            const INFO: $crate::heap::layout::ElemInfo = $crate::heap::layout::ElemInfo {
                name: stringify!($name),
                size: $size,
                fields: &[ $( $crate::heap::layout::FieldInfo {
                    name: stringify!($field),
                    offset: $off,
                    ty: <$fty as $crate::heap::layout::FieldType>::TY,
                }, )* ],
            };
        }

        const _: () = $crate::heap::layout::validate_elem(
            &<$name as $crate::heap::layout::Elem>::INFO,
        );
    )*};
}

/// Declares all cell kinds. See the module docs.
///
/// ```ignore
/// cells! {
///     /// Docs.
///     2 => Str8 { LEN: u32 = 8, HASH: u32 = 12, } tail DATA: [u8] = 16;
/// }
/// ```
macro_rules! cells {
    ($(
        $(#[$meta:meta])*
        $id:literal => $name:ident {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ty = $off:literal, )*
        }
        $( tail $(#[$tmeta:meta])* $tail:ident : [$tty:ty] = $toff:literal )? ;
    )*) => {
        /// The kind byte of a cell header. 0 is never a valid kind, so zeroed memory is
        /// detected by the heap walker.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[repr(u8)]
        pub enum CellKind {
            $( $name = $id, )*
        }

        impl CellKind {
            pub const fn from_u8(b: u8) -> Option<CellKind> {
                match b {
                    $( $id => Some(CellKind::$name), )*
                    _ => None,
                }
            }

            pub const fn layout(self) -> &'static $crate::heap::layout::CellLayout {
                match self {
                    $( CellKind::$name => &$name::LAYOUT, )*
                }
            }
        }

        $(
            $(#[$meta])*
            pub struct $name;

            impl $name {
                pub const KIND: CellKind = CellKind::$name;
                $( $(#[$fmeta])* pub const $field: $crate::heap::layout::Field<$fty> =
                    $crate::heap::layout::Field::new($off); )*
                $( $(#[$tmeta])* pub const $tail: $crate::heap::layout::Tail<$tty> =
                    $crate::heap::layout::Tail::new($toff); )?
                pub const LAYOUT: $crate::heap::layout::CellLayout =
                    $crate::heap::layout::CellLayout::new(
                        stringify!($name),
                        $id,
                        &[ $( $crate::heap::layout::FieldInfo {
                            name: stringify!($field),
                            offset: $off,
                            ty: <$fty as $crate::heap::layout::FieldType>::TY,
                        }, )* ],
                        $crate::heap::layout::opt!($( $crate::heap::layout::TailInfo {
                            name: stringify!($tail),
                            offset: $toff,
                            elem: <$tty as $crate::heap::layout::Elem>::INFO,
                        } )?),
                    );
            }

            const _: () = $crate::heap::layout::validate_cell(&$name::LAYOUT);
        )*

        /// The layout of every cell kind, in kind order.
        pub const LAYOUTS: &[$crate::heap::layout::CellLayout] = &[ $( $name::LAYOUT, )* ];
    };
}

pub(crate) use {cells, opt, records};
