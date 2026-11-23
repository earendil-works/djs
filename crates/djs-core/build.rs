//! Computes `SOURCE_HASH` (D1330): a hash of everything that goes into the library and so
//! into its heap images: the non-test Rust sources under `src/`, those of djs-libm (Math
//! results come from it, D3174) and the generated prelude.
//! `vm::BUILD_HASH` includes it and so identifies the exact build (`djs_build_hash`, run
//! journals, djs-prelude's check that the base image is this build's, and the build a VM
//! image loads into, D3641).
//!
//! Test files (`*tests.rs`) are left out, so editing a test does not rebuild the library
//! and the base snapshot. Line endings are normalized, so a CRLF checkout hashes the same.
//!
//! Behavior also depends on something outside the sources (D1750): the enabled cargo
//! features (`\p{..}` is a SyntaxError without `regexp-unicode-props`). They are hashed
//! too, and listed in `BUILD_INPUTS`.

use std::path::{Path, PathBuf};

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

/// The enabled features of this crate, sorted, without `default` and `unicode` (which only
/// name others) and `unsafe-heap`, which changes how the arena is accessed but no
/// result or image (D2520): the base snapshot, built by a host build without it, loads into
/// a build with it, and images move freely between the two. Nor `dump` (0167), which only
/// adds `Vm::dump`, a reader of the heap: a run never calls it. Nor `lazy` (D2990): the base
/// holds no lazy template (the prelude is privileged), and lazy compilation is a VM option.
/// Nor `prelude`
/// (D3094): it only decides whether a wasm build can run the prelude source. Nor
/// `intrinsic-names` (D3340): names in messages and listings. Nor `cold-unicode` (D3350):
/// where the property tables come from. `embed` counts (D3175): its
/// base has builtins the others leave out.
fn features() -> Vec<String> {
    enabled_features()
        .into_iter()
        // `default` and `unicode` only name others: a build that turns on the
        // same effective features through other names hashes the same.
        .filter(|f| {
            f != "default"
                && f != "unicode"
                && f != "unsafe-heap"
                && f != "dump"
                && f != "lazy"
                && f != "prelude"
                && f != "intrinsic-names"
                && f != "cold-unicode"
        })
        .collect()
}

/// Every enabled feature of this crate, sorted (umbrellas too).
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
    // `djs_prelude` (D3094): this build can compile and run the prelude source (`Vm::new`
    // with `VmOptions::prelude`, the `%Install*` intrinsics it calls). Always natively; in
    // wasm only with the `prelude` feature, so that the shipped modules, which start from
    // the base image, carry none of it.
    println!("cargo::rustc-check-cfg=cfg(djs_prelude)");
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    if !wasm || std::env::var_os("CARGO_FEATURE_PRELUDE").is_some() {
        println!("cargo::rustc-cfg=djs_prelude");
    }
    // `djs_intrinsic_names` (D3340): the intrinsics' names (8 KB of static data), which
    // only privileged code (the prelude source), the disassembler and the registry read;
    // elsewhere they are in messages of engine bugs, which then say `%?`.
    println!("cargo::rustc-check-cfg=cfg(djs_intrinsic_names)");
    if !wasm
        || std::env::var_os("CARGO_FEATURE_PRELUDE").is_some()
        || std::env::var_os("CARGO_FEATURE_INTRINSIC_NAMES").is_some()
    {
        println!("cargo::rustc-cfg=djs_intrinsic_names");
    }
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut files = Vec::new();
    collect(&root.join("src"), &mut files);
    // djs-libm's tests are in its tests/, which is not hashed.
    collect(&root.join("../djs-libm/src"), &mut files);
    files.push(root.join("../../builtins/gen/prelude.js"));
    files.push(root.join("../../builtins/gen/prelude-noembed.js"));
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
    // Cargo reruns the script when the features change.
    let inputs: Vec<String> = features()
        .into_iter()
        .map(|f| format!("feature {f}"))
        .collect();
    for input in &inputs {
        h = fnv(h, input.as_bytes());
        h = fnv(h, &[0]);
    }
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let out = out_dir.join("source_hash.rs");
    let list: String = inputs.iter().map(|i| format!("{i:?}, ")).collect();
    std::fs::write(
        out,
        format!(
            "/// See `build.rs`.\npub const SOURCE_HASH: u32 = {h:#010x};\n\
             /// What `SOURCE_HASH` covers besides the sources and the prelude: the enabled\n\
             /// cargo features (build.rs, D1750).\n\
             pub const BUILD_INPUTS: &[&str] = &[{list}];\n"
        ),
    )
    .expect("cannot write source_hash.rs");
}
