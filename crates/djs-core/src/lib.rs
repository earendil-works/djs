//! djs-core: lexer, compiler, heap, GC, interpreter, leaf intrinsics, snapshots.
//!
//! `no_std` + `alloc`. See ARCH.md at the repository root.
#![no_std]
// No `unsafe` in djs-core (AGENTS.md), except the arena's unchecked accessors in
// heap/layout/access.rs with the `unsafe-heap` feature (D2520).
#![cfg_attr(not(feature = "unsafe-heap"), forbid(unsafe_code))]
#![cfg_attr(feature = "unsafe-heap", deny(unsafe_code))]

extern crate alloc;

/// Whether this build has every builtin family; without the `full` feature it is the slim
/// build (D2411, docs/slim-codemode.md). A constant rather than `#[cfg]` where code is
/// cut, so that both builds type-check the same code and the optimizer drops what the
/// slim one can't reach.
pub const FULL: bool = cfg!(feature = "full");

pub mod bigint;
pub(crate) mod builtins;
pub mod bytecode;
pub mod compiler;
mod detail;
pub mod heap;
pub mod interp;
pub mod json;
pub mod lexer;
pub mod num;
pub mod regexp;
mod sort;
#[macro_use]
pub(crate) mod text;
pub mod unicode;
pub mod value;
pub mod vm;
pub mod wire;
