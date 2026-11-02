//! The base snapshot (ARCH G2, "Snapshots"; D1321): the image of an idle VM with its
//! realm and prelude, made by `build.rs` when djs-core changes and embedded here. A new
//! VM is a copy of it, with no compile at startup.
//!
//! What the base holds: the atom table, the realm (global object, intrinsics, builtin
//! functions), the compiled prelude and the interpreter state of an idle VM. Nothing per
//! VM: no handles, no host functions, no pending calls; embedders add those after
//! [`new_vm`].
//!
//! `Vm::new` (which compiles the prelude) stays for tests and for making this image.

#![no_std]

extern crate alloc;

#[cfg(feature = "registry")]
pub mod registry;

use djs_core::heap::{HeapError, LoadError};
use djs_core::vm::{Vm, VmOptions};

/// The base image (`Vm::bytes` of a fresh, collected VM of this build).
#[cfg(feature = "raw")]
pub static BASE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/base.img"));

/// The base image coded with djs-lzrc (D2252): for the wasm module, which decodes it once
/// with [`unpack_base`].
#[cfg(feature = "packed")]
pub static BASE_PACKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/base.lzr"));

/// The base image, decoded from [`BASE_PACKED`].
#[cfg(feature = "packed")]
pub fn unpack_base() -> alloc::vec::Vec<u8> {
    unpack_base_into(alloc::vec::Vec::new())
}

/// [`unpack_base`] into `out` (cleared first), keeping its capacity: a buffer sized for
/// the arena it will become (D2060).
#[cfg(feature = "packed")]
pub fn unpack_base_into(out: alloc::vec::Vec<u8>) -> alloc::vec::Vec<u8> {
    // Coded and decoded by this build's build.rs: a failure is a build bug.
    djs_lzrc::decode_into(BASE_PACKED, usize::MAX, out)
        .unwrap_or_else(|e| panic!("the embedded base snapshot does not decode: {e:?}"))
}

/// The length of the base image (decoded from [`BASE_PACKED`]).
#[cfg(feature = "packed")]
pub fn base_len() -> usize {
    djs_lzrc::decoded_len(BASE_PACKED).expect("the coded base has a header")
}

/// Whether `bytes` are the base image, by the coded image's length and FNV-1a hash: for a
/// copy a host decoded once for many instances (D2252).
#[cfg(feature = "packed")]
pub fn is_base(bytes: &[u8]) -> bool {
    djs_lzrc::matches(BASE_PACKED, bytes)
}

/// A new VM from [`BASE`]. `opts.prelude` is ignored: the base has the prelude.
/// Fails only if the memory limit is smaller than the base.
#[cfg(feature = "raw")]
pub fn new_vm(opts: VmOptions) -> Result<Vm, HeapError> {
    new_vm_reserved(opts, 0)
}

/// [`new_vm`] with the arena capacity of a heap of `live` bytes
/// ([`Vm::from_base_reserved`]).
#[cfg(feature = "raw")]
pub fn new_vm_reserved(opts: VmOptions, live: usize) -> Result<Vm, HeapError> {
    new_vm_from_reserved(BASE, opts, live)
}

/// A new VM from `base`, which is this build's base image ([`BASE`] or
/// [`unpack_base`]'s).
pub fn new_vm_from(base: &[u8], opts: VmOptions) -> Result<Vm, HeapError> {
    new_vm_from_reserved(base, opts, 0)
}

/// [`new_vm_from`] with the arena capacity of a heap of `live` bytes.
pub fn new_vm_from_reserved(base: &[u8], opts: VmOptions, live: usize) -> Result<Vm, HeapError> {
    match Vm::from_base_reserved(base, opts, live) {
        Ok(vm) => Ok(vm),
        Err(LoadError::TooLarge) => Err(HeapError::OutOfMemory),
        // Made by this build's build.rs and verified there: anything else is a build bug.
        Err(e) => panic!("the embedded base snapshot does not load: {e}"),
    }
}

/// A new VM whose arena is `base`, this build's base image in a buffer the caller owns
/// (D2060): no copy. With the arena capacity of a heap of `live` bytes.
pub fn new_vm_owned(
    base: alloc::vec::Vec<u8>,
    opts: VmOptions,
    live: usize,
) -> Result<Vm, HeapError> {
    match Vm::from_base_vec(base, opts, live) {
        Ok(vm) => Ok(vm),
        Err(LoadError::TooLarge) => Err(HeapError::OutOfMemory),
        Err(e) => panic!("the embedded base snapshot does not load: {e}"),
    }
}

#[cfg(all(test, feature = "raw"))]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn fresh_image() -> Vec<u8> {
        let mut vm = Vm::new(VmOptions::default()).unwrap();
        vm.prepare_base_image().unwrap();
        vm.bytes().to_vec()
    }

    /// The image is deterministic: building it again gives the same bytes, and they are
    /// what build.rs embedded.
    #[test]
    fn deterministic() {
        let a = fresh_image();
        assert_eq!(a, fresh_image());
        assert!(
            a == BASE,
            "BASE differs from a fresh build of the same djs-core"
        );
    }

    /// The builtin prefix references nothing above it (D2063): the per-VM cells of the
    /// tail are reached only from the roots cell and from each other.
    #[test]
    fn the_prefix_references_only_itself() {
        use djs_core::heap::walk::{Slot, for_each_slot};
        use djs_core::heap::{Heap, Ref};
        let h = Heap::from_bytes_trusted(BASE.to_vec(), usize::MAX).unwrap();
        let end = h.base_end();
        assert!(end > 0 && end < BASE.len() as u32);
        let a = h.arena();
        let mut out = Vec::new();
        for c in h.cells().filter(|c| c.offset > 0 && c.offset < end) {
            for_each_slot(c.kind, c.offset, c.size, |s| {
                let t = match s {
                    Slot::Value(s) => a.read::<djs_core::value::Value>(s).ref_offset(),
                    Slot::Ref(s) => a.read::<Ref>(s).get(),
                };
                if t.is_some_and(|t| t >= end) {
                    out.push((c.kind, c.offset));
                }
            });
        }
        assert!(out.is_empty(), "prefix cells referencing the tail: {out:?}");
    }

    /// The embedded base passes full verification.
    #[test]
    fn verifies() {
        Vm::from_bytes(BASE.to_vec(), VmOptions::default()).unwrap();
    }

    #[test]
    fn new_vm_and_limits() {
        let vm = new_vm(VmOptions::default()).unwrap();
        assert_eq!(vm.bytes(), BASE);
        let small = VmOptions {
            memory_limit: BASE.len() - 8,
            ..VmOptions::default()
        };
        assert_eq!(new_vm(small).err(), Some(HeapError::OutOfMemory));
    }

    /// The coded image decodes to the image.
    #[cfg(feature = "packed")]
    #[test]
    fn packed_is_the_base() {
        assert_eq!(unpack_base(), BASE);
        assert!(BASE_PACKED.len() < BASE.len() / 3);
    }
}
