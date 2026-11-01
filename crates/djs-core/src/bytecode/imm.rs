//! Immediate operand kinds (docs/compiler.md 4.1).
//!
//! Every immediate in the table has a kind. The kind fixes the width and encoding
//! (little-endian, unaligned), the Rust type of the emitter argument, and how the
//! disassembler prints it.

use super::code::{CodeBuf, Label, LabelMode};
use crate::heap::Atom as AtomId;

/// The kind of an immediate operand.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum ImmKind {
    /// A small enum or flag set (`special_object` kind, `copy_data_props` mode).
    U8,
    /// A small signed integer constant.
    I8,
    /// A signed integer constant.
    I32,
    /// An element count (`array_from`, `concat`): popped values.
    Count,
    /// A local slot index.
    Loc,
    /// An argument slot index.
    Arg,
    /// A capture index.
    Cap,
    /// A call argument count.
    Argc,
    /// A scope id (phase 1).
    Scope,
    /// An intrinsic id (privileged code).
    Intrinsic,
    /// A constant pool index.
    Const,
    /// A child template index.
    Child,
    /// A name-ref side table index (phase 1).
    NameRef,
    /// The id of a `label` pseudo-op (phase 1).
    LabelId,
    /// An atom id.
    Atom,
    /// A jump target: a label id in phase-1 code, an `i32` offset relative to the end of
    /// the instruction in final code. Always the last immediate.
    Target,
}

impl ImmKind {
    /// Encoded width in bytes.
    pub const fn width(self) -> u8 {
        match self {
            ImmKind::U8 | ImmKind::I8 => 1,
            ImmKind::Count
            | ImmKind::Loc
            | ImmKind::Arg
            | ImmKind::Cap
            | ImmKind::Argc
            | ImmKind::Scope
            | ImmKind::Intrinsic => 2,
            ImmKind::I32
            | ImmKind::Const
            | ImmKind::Child
            | ImmKind::NameRef
            | ImmKind::LabelId
            | ImmKind::Atom
            | ImmKind::Target => 4,
        }
    }

    /// Whether the raw value is sign-extended when decoded.
    pub const fn is_signed(self) -> bool {
        matches!(self, ImmKind::I8 | ImmKind::I32 | ImmKind::Target)
    }
}

/// Maps an immediate kind to its emitter argument type and encoding. Implemented by the
/// marker types below, which the `ops!` table names.
pub trait Imm {
    const KIND: ImmKind;
    type Ty: Copy;
    fn emit<M: LabelMode>(buf: &mut CodeBuf<M>, v: Self::Ty);
}

macro_rules! plain_imm {
    ($($name:ident: $ty:ty => |$v:ident| $bytes:expr;)*) => {$(
        #[doc = concat!("Marker for [`ImmKind::", stringify!($name), "`].")]
        pub struct $name;
        impl Imm for $name {
            const KIND: ImmKind = ImmKind::$name;
            type Ty = $ty;
            #[inline]
            fn emit<M: LabelMode>(buf: &mut CodeBuf<M>, $v: $ty) {
                buf.push_bytes(&$bytes);
            }
        }
    )*};
}

plain_imm! {
    U8: u8 => |v| [v];
    I8: i8 => |v| v.to_le_bytes();
    I32: i32 => |v| v.to_le_bytes();
    Count: u16 => |v| v.to_le_bytes();
    Loc: u16 => |v| v.to_le_bytes();
    Arg: u16 => |v| v.to_le_bytes();
    Cap: u16 => |v| v.to_le_bytes();
    Argc: u16 => |v| v.to_le_bytes();
    Scope: u16 => |v| v.to_le_bytes();
    Intrinsic: u16 => |v| v.to_le_bytes();
    Const: u32 => |v| v.to_le_bytes();
    Child: u32 => |v| v.to_le_bytes();
    NameRef: u32 => |v| v.to_le_bytes();
    LabelId: Label => |v| v.0.to_le_bytes();
    Atom: AtomId => |v| v.id().to_le_bytes();
}

/// Marker for [`ImmKind::Target`].
pub struct Target;

impl Imm for Target {
    const KIND: ImmKind = ImmKind::Target;
    type Ty = Label;
    #[inline]
    fn emit<M: LabelMode>(buf: &mut CodeBuf<M>, v: Label) {
        buf.push_target(v);
    }
}

// The table's `ImmKind`s and the marker types must agree on the width.
const _: () = {
    assert!(<U8 as Imm>::KIND.width() == 1 && <I8 as Imm>::KIND.width() == 1);
    assert!(<Loc as Imm>::KIND.width() == 2 && <Atom as Imm>::KIND.width() == 4);
    assert!(<Target as Imm>::KIND.width() == 4 && <LabelId as Imm>::KIND.width() == 4);
};
