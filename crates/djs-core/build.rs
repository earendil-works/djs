//! Computes `SOURCE_HASH` (D1330): a hash of everything that goes into the library and so
//! into its heap images: the non-test Rust sources under `src/` and the generated prelude.
//! `vm::BUILD_HASH` includes it, so an image only loads into the build that made it, even
//! when a change keeps the layout, op and intrinsic tables (a reordered realm slot, a new
//! object class id, a changed builtin).
//!
//! Test files (`*tests.rs`) are left out, so editing a test does not rebuild the library
//! and the base snapshot. Line endings are normalized, so a CRLF checkout hashes the same.

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

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut files = Vec::new();
    collect(&root.join("src"), &mut files);
    files.push(root.join("../../builtins/gen/prelude.js"));
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
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("source_hash.rs");
    std::fs::write(
        out,
        format!("/// See `build.rs`.\npub const SOURCE_HASH: u32 = {h:#010x};\n"),
    )
    .expect("cannot write source_hash.rs");
}
