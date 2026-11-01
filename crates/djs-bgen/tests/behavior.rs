//! Runs the generated builtins (builtins/gen/prelude.js) in qjs against qjs's native
//! Array.prototype methods (tests/behavior.js) and expects identical results, error
//! messages and proxy trap orders. Skipped (with a note) when qjs is not available.

use std::path::{Path, PathBuf};
use std::process::Command;

fn qjs() -> PathBuf {
    if let Some(p) = std::env::var_os("DJS_QJS") {
        return PathBuf::from(p);
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Development/dquickts/qjs")
}

#[test]
fn generated_builtins_match_qjs() {
    let qjs = qjs();
    if !qjs.is_file() {
        eprintln!("skipping: qjs not found at {} (set DJS_QJS)", qjs.display());
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prelude = std::fs::read_to_string(root.join("../../builtins/gen/prelude.js")).unwrap();
    // `%Name(` only appears as intrinsic calls: the prelude has no `%` in strings.
    let prelude = prelude.replace('%', "I.");
    let script = std::fs::read_to_string(root.join("tests/behavior.js"))
        .unwrap()
        .replace("PRELUDE;", &prelude);
    let path = std::env::temp_dir().join(format!("djs-bgen-behavior-{}.js", std::process::id()));
    std::fs::write(&path, script).unwrap();
    let out = Command::new(&qjs)
        .arg("--script")
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "qjs failed:\n{stdout}\n{stderr}");
    let last = stdout.lines().last().unwrap_or("");
    assert!(
        last.starts_with("ok ") && !last.contains("mismatch"),
        "{stdout}{stderr}"
    );
    let n: usize = last[3..].trim().parse().unwrap();
    assert!(n > 2000, "only {n} comparisons ran");
}
