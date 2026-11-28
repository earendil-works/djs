//! Makes the base snapshot (ARCH "Images and snapshots", D1321): a VM with the prelude installed,
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
    let base_end = djs_core::heap::Heap::image_base_end(&image);
    std::fs::write(
        dir.join("base_end.rs"),
        format!("/// The base image's builtin prefix (`Roots::BASE_END`).\npub const BASE_END: u32 = {base_end};\n"),
    )
    .expect("cannot write the base's prefix length");
    // The build hash of the djs-core that made the image, which lib.rs compares with the
    // library's: the two are separate builds whose features cargo resolves apart (D2411).
    std::fs::write(
        dir.join("built_by.rs"),
        format!(
            "const BUILT_BY: u32 = {:#010x};\n",
            djs_core::vm::BUILD_HASH
        ),
    )
    .expect("cannot write the build hash");
    // The coded image of the `packed` feature (D2252), checked by decoding it.
    if std::env::var_os("CARGO_FEATURE_PACKED").is_some() {
        let packed = djs_lzrc::encode(&image);
        assert_eq!(
            djs_lzrc::decode(&packed, image.len()).as_deref(),
            Ok(&image[..]),
            "the coded base image does not decode"
        );
        std::fs::write(dir.join("base.lzr"), &packed).expect("cannot write the coded image");
        let header: Vec<String> = packed[..16].iter().map(|b| b.to_string()).collect();
        std::fs::write(
            dir.join("base_header.rs"),
            format!(
                "/// The coded base's header (length and hash of the image).\n\
                 pub const PACKED_HEADER: [u8; 16] = [{}];\n\
                 /// The coded base's length.\n\
                 pub const PACKED_LEN: usize = {};\n\
                 /// `base_sample` of the image.\n\
                 const BASE_SAMPLE: u64 = {};\n\
                 /// The image's length.\n\
                 const BASE_LEN: usize = {};\n",
                header.join(", "),
                packed.len(),
                base_sample(&image),
                image.len()
            ),
        )
        .expect("cannot write the coded image's header");
        // The wasm module's custom section (lib.rs), as an array literal.
        let bytes: Vec<String> = packed.iter().map(|b| b.to_string()).collect();
        std::fs::write(
            dir.join("base_section.rs"),
            format!(
                "#[unsafe(link_section = \"djs_base\")]\n\
                 pub static BASE_SECTION: [u8; {}] = [{}];\n",
                packed.len(),
                bytes.join(",")
            ),
        )
        .expect("cannot write the coded image's section");
    }
}

/// djs-prelude's `base_sample` (lib.rs; its tests check that the two agree).
fn base_sample(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i + 8 <= b.len() {
        h = (h ^ u64::from_le_bytes(b[i..i + 8].try_into().unwrap())).wrapping_mul(0x100_0000_01b3);
        i += if i < 256 { 8 } else { 504 };
    }
    h
}
