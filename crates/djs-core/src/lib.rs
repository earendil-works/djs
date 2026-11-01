//! djs-core: lexer, compiler, heap, GC, interpreter, leaf intrinsics, snapshots.
//!
//! `no_std` + `alloc`. See ARCH.md at the repository root.
#![no_std]

extern crate alloc;

pub mod heap;
pub mod lexer;
pub mod unicode;
pub mod value;
