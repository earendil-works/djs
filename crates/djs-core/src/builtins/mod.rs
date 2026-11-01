//! Leaf intrinsics of the builtins (ARCH.md "The leaf rule"), one module per area.
//!
//! The builtins themselves are in `builtins/*.ts` (the builtin language): they do the
//! conversions and checks that may run user code, then call these leaf intrinsics,
//! which work on the heap and never run JS. `interp/run/intrinsics.rs` dispatches
//! `call_intrinsic` to them.

pub(crate) mod array;
pub(crate) mod function;
pub(crate) mod number;
pub(crate) mod object;
pub(crate) mod string;
