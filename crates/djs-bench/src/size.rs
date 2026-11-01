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
    // The probe keeps its result names (`size/wasm ...`) so its history stays comparable.
    let probe = ["--manifest-path", "tests/bench/size/Cargo.toml"];
    measure(r, "size/wasm", &probe, &target_dir, "djs_size_probe.wasm");
    // The real module (0023).
    let module = ["-p", "djs-wasm"];
    measure(r, "size/djs-wasm", &module, &target_dir, "djs_wasm.wasm");
}

/// Builds a wasm32 cdylib with the `wasm` profile and reports its sizes under `prefix`.
fn measure(r: &mut Runner, prefix: &str, what: &[&str], target_dir: &Path, file: &str) {
    if !r.wants(prefix) {
        return;
    }
    let root = workspace_root();
    let status = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(["build", "--quiet"])
        .args(what)
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            "wasm",
            "--target-dir",
        ])
        .arg(target_dir)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        return r.skip(
            prefix,
            "building failed (is the wasm32-unknown-unknown target installed?)",
        );
    }
    let wasm = target_dir.join("wasm32-unknown-unknown/wasm").join(file);
    let Ok(bytes) = std::fs::read(&wasm) else {
        return r.skip(
            prefix,
            format!("{} missing after the build", wasm.display()),
        );
    };
    let name = |what: &str| format!("{prefix} {what}");
    r.value(&name("file"), Unit::Bytes, bytes.len() as f64);
    match without_custom_sections(&bytes) {
        Some(n) => r.value(&name("without custom sections"), Unit::Bytes, n as f64),
        None => r.skip(
            &name("without custom sections"),
            "could not parse the wasm file",
        ),
    }
    // The sections that matter separately (0043): code, data (mostly the base image) and
    // the name section (stripped for shipping).
    if let Some(sections) = section_sizes(&bytes) {
        for (what, id, custom) in [("code", 10, ""), ("data", 11, ""), ("names", 0, "name")] {
            let n: usize = sections
                .iter()
                .filter(|s| s.0 == id && s.1 == custom)
                .map(|s| s.2)
                .sum();
            r.value(&name(what), Unit::Bytes, n as f64);
        }
    }
    match gzip_size(&bytes) {
        Some(n) => r.value(&name("gzip -9"), Unit::Bytes, n as f64),
        None => r.skip(&name("gzip -9"), "gzip not available"),
    }
    // `size/wasm-opt -Oz` is the probe's historical name.
    let opt_name = if prefix == "size/wasm" {
        "size/wasm-opt -Oz".to_string()
    } else {
        name("wasm-opt -Oz")
    };
    match wasm_opt_size(&wasm) {
        Some(n) => r.value(&opt_name, Unit::Bytes, n as f64),
        None => r.skip(&opt_name, "wasm-opt not installed"),
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

/// `(id, custom section name, payload size)` of every section.
pub fn section_sizes(wasm: &[u8]) -> Option<Vec<(u8, String, usize)>> {
    if wasm.len() < 8 || &wasm[..4] != b"\0asm" {
        return None;
    }
    let mut out = Vec::new();
    let mut pos = 8;
    while pos < wasm.len() {
        let id = wasm[pos];
        let (len, n) = leb128(&wasm[pos + 1..])?;
        let start = pos + 1 + n;
        let end = start
            .checked_add(len as usize)
            .filter(|&e| e <= wasm.len())?;
        let name = if id == 0 {
            let (nlen, k) = leb128(&wasm[start..end])?;
            let s = wasm.get(start + k..start + k + nlen as usize)?;
            String::from_utf8_lossy(s).into_owned()
        } else {
            String::new()
        };
        out.push((id, name, len as usize));
        pos = end;
    }
    Some(out)
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
        assert_eq!(
            section_sizes(&m),
            Some(vec![(1, String::new(), 3), (0, "n".into(), 5)])
        );
        assert_eq!(without_custom_sections(b"nope"), None);
        assert_eq!(leb128(&[0xe5, 0x8e, 0x26]), Some((624485, 3)));
    }
}
