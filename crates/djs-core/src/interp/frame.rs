//! Frame records on the value stack (docs/compiler.md 7.1, D1086, D1150).
//!
//! ```text
//! bp      callee            the function object
//! bp+1    this
//! bp+2    arg0 .. arg(n-1)  n = max(argc, PARAMS), missing args padded with undefined
//! hp      prev_bp           int32
//! hp+1    prev_hp           int32
//! hp+2    ret_pc            int32: the caller's pc to resume (after the call instruction)
//! hp+3    argc              int32: actual argument count
//! hp+4    flags             int32: continuation (low 4 bits) | construct bit
//! hp+5    new_target        Value
//! lp      locals            lp = hp + HEADER, filled with undefined
//! sb      operand stack     sb = lp + LOCALS
//! ```
//!
//! The header needs `prev_hp` next to `prev_bp`: a frame's header position depends on its
//! padded argument count, which is only recorded in that header, so the caller's `hp`
//! cannot be recomputed from its `bp`. All header fields are plain Values, so the GC
//! traces the stack without knowing about frames.

/// Header slots.
pub const HEADER: u32 = 6;
pub const PREV_BP: u32 = 0;
pub const PREV_HP: u32 = 1;
pub const RET_PC: u32 = 2;
pub const ARGC: u32 = 3;
pub const FLAGS: u32 = 4;
pub const NEW_TARGET: u32 = 5;

/// `flags` bit: the frame was entered by `construct`.
pub const FLAG_CONSTRUCT: i32 = 1 << 4;

/// What happens with a frame's return value (docs/compiler.md 7.3, D1085).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Continuation {
    /// Push it on the caller's operand stack (calls, getters, slow paths).
    PushResult = 0,
    /// Drop it (setters).
    DiscardResult = 1,
    /// If it is not an object, use the frame's `this` (constructors).
    ConstructResult = 2,
    /// The bottom frame of a `Vm::call`: `run` returns `Done(value)`.
    Entry = 3,
}

impl Continuation {
    pub const fn from_flags(flags: i32) -> Continuation {
        match flags & 0xF {
            0 => Continuation::PushResult,
            1 => Continuation::DiscardResult,
            2 => Continuation::ConstructResult,
            _ => Continuation::Entry,
        }
    }
}
