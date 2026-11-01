//! The module's Rust heap allocator, in one place so it can be replaced as a unit.
//!
//! [`Base`] is the allocator underneath everything: on wasm32, where djs-wasm is `no_std`
//! (D2332), the `dlmalloc` crate, the same allocator std uses on this target; natively
//! std's system allocator. It is the global allocator, unless the `alloc-stats` feature
//! wraps it in a counting allocator (lib.rs).
//!
//! (Not named `alloc`: that is the `alloc` crate inside this crate.)

#[cfg(target_arch = "wasm32")]
pub(crate) use dlmalloc::GlobalDlmalloc as Base;
// Used natively only by the `alloc-stats` wrapper.
#[cfg(all(not(target_arch = "wasm32"), feature = "alloc-stats"))]
pub(crate) use std::alloc::System as Base;

// Natively std's allocator is already the global one.
#[cfg(all(target_arch = "wasm32", not(feature = "alloc-stats")))]
#[global_allocator]
static ALLOC: Base = Base;
