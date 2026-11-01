//! djs-core: lexer, compiler, heap, GC, interpreter, leaf intrinsics, snapshots.
//!
//! `no_std` + `alloc`. See ARCH.md at the repository root.
#![no_std]

extern crate alloc;

pub(crate) mod builtins;
pub mod bytecode;
pub mod compiler;
pub mod heap;
pub mod interp;
pub mod json;
pub mod lexer;
pub mod num;
pub mod regexp;
pub mod unicode;
pub mod value;
pub mod vm;
pub mod wire;
