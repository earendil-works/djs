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

use djs_core::heap::{HeapError, LoadError};
use djs_core::vm::{Vm, VmOptions};

/// The base image (`Vm::bytes` of a fresh, collected VM of this build).
pub static BASE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/base.img"));

/// A new VM from [`BASE`]. `opts.prelude` is ignored: the base has the prelude.
/// Fails only if the memory limit is smaller than the base.
pub fn new_vm(opts: VmOptions) -> Result<Vm, HeapError> {
    match Vm::from_base(BASE, opts) {
        Ok(vm) => Ok(vm),
        Err(LoadError::TooLarge) => Err(HeapError::OutOfMemory),
        // Made by this build's build.rs and verified there: anything else is a build bug.
        Err(e) => panic!("the embedded base snapshot does not load: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn fresh_image() -> Vec<u8> {
        let mut vm = Vm::new(VmOptions::default()).unwrap();
        vm.heap_mut().compact_for_image().unwrap();
        vm.collect().unwrap();
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
}
