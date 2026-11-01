//! Makes the base snapshot (ARCH "Snapshots", D1321): a VM with the prelude installed,
//! collected, as a heap image in `$OUT_DIR/base.img`. The GC is deterministic (D1090), so
//! the same djs-core gives the same bytes; the image is verified here once, so loading it
//! can skip verification.

use djs_core::vm::{Vm, VmOptions};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut vm = Vm::new(VmOptions::default()).expect("out of memory making the base");
    vm.collect().expect("out of memory collecting the base");
    let image = vm.bytes().to_vec();
    // Full verification (cell structure, references, build hash) at build time.
    if let Err(e) = Vm::from_bytes(image.clone(), VmOptions::default()) {
        panic!("the base image does not load: {e}");
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("base.img");
    std::fs::write(&out, &image).expect("cannot write the base image");
}
