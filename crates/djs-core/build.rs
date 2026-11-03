//! Computes `SOURCE_HASH` (D1330): a hash of everything that goes into the library and so
//! into its heap images: the non-test Rust sources under `src/` and the generated prelude.
//! `vm::BUILD_HASH` includes it and so identifies the exact build (`djs_build_hash`, run
//! journals, djs-prelude's check that the base image is this build's). Images are not keyed
//! on it but on their dependency descriptor (below, D2265).
//!
//! Test files (`*tests.rs`) are left out, so editing a test does not rebuild the library
//! and the base snapshot. Line endings are normalized, so a CRLF checkout hashes the same.
//!
//! Behavior also depends on things outside the sources (D1750): the enabled cargo
//! features (`\p{..}` is a SyntaxError without `regexp-unicode-props`) and the `libm`
//! version (Math results). Both are hashed too, and listed in `BUILD_INPUTS`. `libm` is
//! pinned with `=` in Cargo.toml, so the version read from there is the one compiled
//! (`vm::tests` checks it against Cargo.lock).
//!
//! It also computes two components of an image's dependency descriptor (`vm::ImageDeps`,
//! D2265), which unlike `SOURCE_HASH` key image compatibility: `RECORDS_HASH` (D2266), the
//! id tables and slot conventions images store numbers of (the regions `compat_inputs.rs`
//! lists, with comments and whitespace removed), and `FEATURES` (D2267), the mask of the
//! enabled features that change images.

use std::path::{Path, PathBuf};

mod compat_inputs {
    include!("compat_inputs.rs");
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("cannot read src/")
        .map(|e| e.expect("cannot read src/").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs")
            && !p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("tests.rs"))
        {
            out.push(p);
        }
    }
}

/// FNV-1a.
fn fnv(mut h: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        if b == b'\r' {
            continue;
        }
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    h
}

/// The exact `libm` requirement (`libm = "=X.Y.Z"`) from Cargo.toml.
fn libm_version(manifest: &str) -> String {
    for line in manifest.lines() {
        if let Some(rest) = line.trim().strip_prefix("libm") {
            let req = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            let version = req
                .strip_prefix('=')
                .expect("libm must be pinned with `=` in Cargo.toml (D1750)");
            return version.to_string();
        }
    }
    panic!("no libm dependency in Cargo.toml");
}

/// The enabled features of this crate, sorted, without `default` and `unicode` (which only
/// name others, D2411) and `unsafe-heap`, which changes how the arena is accessed but no
/// result or image (D2520): the base snapshot, built by a host build without it, loads into
/// a build with it, and images move freely between the two. Nor `dump` (0167), which only
/// adds `Vm::dump`, a reader of the heap: a run never calls it.
fn features() -> Vec<String> {
    enabled_features()
        .into_iter()
        // `default` and `unicode` only name others (D2411): a build that turns on the
        // same effective features through other names hashes the same.
        .filter(|f| f != "default" && f != "unicode" && f != "unsafe-heap" && f != "dump")
        .collect()
}

/// Every enabled feature of this crate, sorted (umbrellas too): what `compat_inputs.rs`
/// classifies, so that no feature escapes the classification (D2267).
fn enabled_features() -> Vec<String> {
    let mut out: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_")
                .map(|f| f.to_lowercase().replace('_', "-"))
        })
        .collect();
    out.sort();
    out
}

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut files = Vec::new();
    collect(&root.join("src"), &mut files);
    files.push(root.join("../../builtins/gen/prelude.js"));
    files.push(root.join("../../builtins/gen/prelude-slim.js"));
    let mut h: u32 = 0x811c_9dc5;
    for f in &files {
        // Every hashed file reruns the script. A new source file needs a `mod` line in an
        // existing one, so that is covered too.
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(&root).unwrap_or(f);
        h = fnv(h, rel.to_string_lossy().replace('\\', "/").as_bytes());
        h = fnv(h, &[0]);
        h = fnv(h, &std::fs::read(f).expect("cannot read a source file"));
        h = fnv(h, &[0]);
    }
    // Cargo reruns the script when the features change; Cargo.toml holds the libm pin.
    println!("cargo:rerun-if-changed=Cargo.toml");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
    let mut inputs: Vec<String> = features()
        .into_iter()
        .map(|f| format!("feature {f}"))
        .collect();
    inputs.push(format!("libm {}", libm_version(&manifest)));
    for input in &inputs {
        h = fnv(h, input.as_bytes());
        h = fnv(h, &[0]);
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=compat_inputs.rs");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let records = compat_inputs::records_hash(&root)
        .unwrap_or_else(|e| panic!("compat_inputs.rs: a region is missing: {e}"));
    let mask = compat_inputs::feature_mask(&enabled_features()).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(
        out_dir.join("image_deps.rs"),
        format!(
            "/// See `build.rs` and `compat_inputs.rs` (D2266).\n\
             pub const RECORDS_HASH: u32 = {records:#010x};\n\
             /// The enabled `IMAGE_FEATURES`, by bit (D2267).\n\
             pub const FEATURES: u32 = {mask:#x};\n\
             /// `FEATURES` bit names (`compat_inputs.rs`).\n\
             pub const FEATURE_NAMES: &[&str] = &{names:?};\n",
            names = compat_inputs::IMAGE_FEATURES
        ),
    )
    .expect("cannot write image_deps.rs");
    let out = out_dir.join("source_hash.rs");
    let list: String = inputs.iter().map(|i| format!("{i:?}, ")).collect();
    std::fs::write(
        out,
        format!(
            "/// See `build.rs`.\npub const SOURCE_HASH: u32 = {h:#010x};\n\
             /// What `SOURCE_HASH` covers besides the sources and the prelude: the enabled\n\
             /// cargo features and the libm version (build.rs, D1750).\n\
             pub const BUILD_INPUTS: &[&str] = &[{list}];\n"
        ),
    )
    .expect("cannot write source_hash.rs");
}
