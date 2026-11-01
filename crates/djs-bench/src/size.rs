//! The size track: a release wasm32 build (the `wasm` profile) of the size probe in
//! tests/bench/size, a cdylib that references djs-core's main entry points. Reports the
//! file size, the size without custom sections (names, producers), the gzip -9 size,
//! and the `wasm-opt -Oz` size when wasm-opt is installed.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::engine::workspace_root;
use crate::runner::{Runner, Unit};

pub fn size(r: &mut Runner) {
    if !r.wants("size/") {
        return;
    }
    if r.config.quick {
        return r.skip("size/wasm", "not in quick mode (needs a wasm build)");
    }
    let root = workspace_root();
    let target_dir = root.join("target/bench-size");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--manifest-path"])
        .arg(root.join("tests/bench/size/Cargo.toml"))
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            "wasm",
            "--target-dir",
        ])
        .arg(&target_dir)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        return r.skip(
            "size/wasm",
            "building the size probe failed (is the wasm32-unknown-unknown target installed?)",
        );
    }
    let wasm = target_dir.join("wasm32-unknown-unknown/wasm/djs_size_probe.wasm");
    let Ok(bytes) = std::fs::read(&wasm) else {
        return r.skip(
            "size/wasm",
            format!("{} missing after the build", wasm.display()),
        );
    };
    r.value("size/wasm file", Unit::Bytes, bytes.len() as f64);
    match without_custom_sections(&bytes) {
        Some(n) => r.value("size/wasm without custom sections", Unit::Bytes, n as f64),
        None => r.skip(
            "size/wasm without custom sections",
            "could not parse the wasm file",
        ),
    }
    match gzip_size(&bytes) {
        Some(n) => r.value("size/wasm gzip -9", Unit::Bytes, n as f64),
        None => r.skip("size/wasm gzip -9", "gzip not available"),
    }
    match wasm_opt_size(&wasm) {
        Some(n) => r.value("size/wasm-opt -Oz", Unit::Bytes, n as f64),
        None => r.skip("size/wasm-opt -Oz", "wasm-opt not installed"),
    }
}

/// Size of the module minus its custom sections (id 0).
pub fn without_custom_sections(wasm: &[u8]) -> Option<usize> {
    if wasm.len() < 8 || &wasm[..4] != b"\0asm" {
        return None;
    }
    let mut pos = 8;
    let mut custom = 0;
    while pos < wasm.len() {
        let id = wasm[pos];
        let (len, n) = leb128(&wasm[pos + 1..])?;
        let total = 1 + n + len as usize;
        if pos + total > wasm.len() {
            return None;
        }
        if id == 0 {
            custom += total;
        }
        pos += total;
    }
    Some(wasm.len() - custom)
}

fn leb128(b: &[u8]) -> Option<(u32, usize)> {
    let mut v: u32 = 0;
    for (i, &byte) in b.iter().enumerate().take(5) {
        v |= ((byte & 0x7f) as u32) << (7 * i);
        if byte & 0x80 == 0 {
            return Some((v, i + 1));
        }
    }
    None
}

fn gzip_size(bytes: &[u8]) -> Option<usize> {
    let mut child = Command::new("gzip")
        .arg("-9c")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let data = bytes.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&data));
    let out = child.wait_with_output().ok()?;
    writer.join().ok()?.ok()?;
    out.status.success().then_some(out.stdout.len())
}

fn wasm_opt_size(wasm: &Path) -> Option<usize> {
    let out = wasm.with_extension("opt.wasm");
    let status = Command::new("wasm-opt")
        .arg("-Oz")
        .arg(wasm)
        .arg("-o")
        .arg(&out)
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    std::fs::metadata(&out).ok().map(|m| m.len() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_sections_are_subtracted() {
        // header, a type section (id 1, 3 bytes), a custom section (id 0, 5 bytes)
        let mut m = b"\0asm\x01\0\0\0".to_vec();
        m.extend_from_slice(&[1, 3, 1, 2, 3]);
        m.extend_from_slice(&[0, 5, 1, b'n', 0, 0, 0]);
        assert_eq!(without_custom_sections(&m), Some(m.len() - 7));
        assert_eq!(without_custom_sections(b"nope"), None);
        assert_eq!(leb128(&[0xe5, 0x8e, 0x26]), Some((624485, 3)));
    }
}
