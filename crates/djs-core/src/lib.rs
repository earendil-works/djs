//! djs-core: lexer, compiler, heap, GC, interpreter, leaf intrinsics, snapshots.
//!
//! `no_std` + `alloc`. See ARCH.md at the repository root.
#![no_std]

extern crate alloc;

pub mod bytecode;
pub mod heap;
pub mod json;
pub mod lexer;
pub mod num;
pub mod regexp;
pub mod unicode;
pub mod value;
