//! Makes the base snapshot (ARCH "Snapshots", D1321): a VM with the prelude installed,
//! collected, as a heap image in `$OUT_DIR/base.img`. The GC is deterministic (D1090), so
//! the same djs-core gives the same bytes; the image is verified here once, so loading it
//! can skip verification.

use djs_core::vm::{Vm, VmOptions};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut vm = Vm::new(VmOptions::default()).expect("out of memory making the base");
    // Without growth slack (0043), canonical (0101), like cells together (D2251).
    vm.prepare_base_image()
        .expect("out of memory making the base image");
    let image = vm.bytes().to_vec();
    // Full verification (cell structure, references, build hash) at build time.
    if let Err(e) = Vm::from_bytes(image.clone(), VmOptions::default()) {
        panic!("the base image does not load: {e}");
    }
    let dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(dir.join("base.img"), &image).expect("cannot write the base image");
    // The coded image of the `packed` feature (D2252), checked by decoding it.
    if std::env::var_os("CARGO_FEATURE_PACKED").is_some() {
        let packed = djs_lzrc::encode(&image);
        assert_eq!(
            djs_lzrc::decode(&packed, image.len()).as_deref(),
            Ok(&image[..]),
            "the coded base image does not decode"
        );
        std::fs::write(dir.join("base.lzr"), &packed).expect("cannot write the coded image");
    }
}
