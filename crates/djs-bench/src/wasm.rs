//! The wasm group: the boundary in djs-wasm, measured in node (0023's
//! `js/bench/wasm.bench.ts`). It builds djs-wasm with the `wasm` profile, writes the
//! boundary datasets as JSON and reads the script's TSV output:
//! - `wasm/decode` and `wasm/parse_json`: wire and JSON into a fresh VM;
//! - `wasm/readValue` and `wasm/json_stringify`: the two ways out;
//! - `wasm/instantiate`, `wasm/vm_new` (from the base snapshot) and `wasm/vm_new compiling
//!   the prelude`: startup.
//!
//! Skipped without node or the wasm32 target, and in quick mode.

use std::process::Command;

use crate::engine::workspace_root;
use crate::runner::{Runner, Unit};

pub fn wasm(r: &mut Runner) {
    if !r.wants_group("wasm/") {
        return;
    }
    if r.config.quick {
        return r.skip("wasm/", "not in quick mode (needs a wasm build and node)");
    }
    let root = workspace_root();
    // Its own target dir: the `compiled-prelude` feature (for the startup comparison)
    // must not flip the size track's build.
    let target_dir = root.join("target/bench-wasm-build");
    let built = Command::new(env!("CARGO"))
        .current_dir(&root)
        .env("CARGO_ENCODED_RUSTFLAGS", crate::size::shipped_rustflags())
        .args([
            "build",
            "--quiet",
            "-p",
            "djs-wasm",
            "--features",
            "compiled-prelude",
        ])
        .args(["--target", "wasm32-unknown-unknown", "--profile", "wasm"])
        .arg("--target-dir")
        .arg(&target_dir)
        .status();
    if !matches!(built, Ok(s) if s.success()) {
        return r.skip(
            "wasm/",
            "building djs-wasm failed (wasm32 target installed?)",
        );
    }
    let data_dir = root.join("target/bench-wasm");
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        return r.skip("wasm/", format!("{}: {e}", data_dir.display()));
    }
    for (name, data) in crate::data::datasets(false) {
        let path = data_dir.join(format!("{name}.json"));
        if let Err(e) = std::fs::write(&path, data.to_json()) {
            return r.skip("wasm/", format!("{}: {e}", path.display()));
        }
    }
    let out = Command::new("node")
        .arg(root.join("js/bench/wasm.bench.ts"))
        .arg(&data_dir)
        .arg("--tsv")
        .env(
            "DJS_WASM",
            target_dir.join("wasm32-unknown-unknown/wasm/djs_wasm.wasm"),
        )
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            let last = err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("");
            return r.skip("wasm/", format!("wasm.bench.ts failed: {last}"));
        }
        Err(e) => return r.skip("wasm/", format!("cannot run node: {e}")),
    };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut f = line.split('\t');
        let (Some(name), Some(v), Some(unit)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if let (Ok(v), Some(unit)) = (v.parse::<f64>(), Unit::parse(unit)) {
            r.value(name, unit, v);
        }
    }
}
