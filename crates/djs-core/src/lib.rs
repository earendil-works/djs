//! djs-core: lexer, compiler, heap, GC, interpreter, leaf intrinsics, snapshots.
//!
//! `no_std` + `alloc`. See ARCH.md at the repository root.
#![no_std]
// No `unsafe` in djs-core (AGENTS.md), except the arena's unchecked accessors in
// heap/layout/access.rs with the `unsafe-heap` feature (D2520).
#![cfg_attr(not(feature = "unsafe-heap"), forbid(unsafe_code))]
#![cfg_attr(feature = "unsafe-heap", deny(unsafe_code))]

extern crate alloc;

/// Whether this build has what Rust hosts get beyond the wasm modules (the `embed` feature,
/// D3175, D3180), ES modules among them (D3171, D3193: without `embed` the module goal is a
/// SyntaxError and the module API fails). A constant rather than `#[cfg]` where code is
/// cut, so that every build type-checks the same code and the optimizer drops what this
/// one can't reach.
pub const EMBED: bool = cfg!(feature = "embed");

/// Whether this build has the Unicode tables (`\p{..}` in regular expressions and full
/// normalization: the `unicode` feature, D2411).
pub const UNICODE: bool = cfg!(all(
    feature = "regexp-unicode-props",
    feature = "unicode-normalization"
));

pub(crate) use buf::PushOut;

pub mod bigint;
mod buf;
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
mod swar;
#[macro_use]
pub(crate) mod text;
pub mod unicode;
pub mod value;
pub mod vm;
pub mod wire;
