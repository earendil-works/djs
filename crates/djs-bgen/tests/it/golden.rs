//! Golden files: the emitter and the report on a fixture with every construct, and the
//! checked-in builtins/gen output (the freshness check, so `cargo test` catches stale
//! generated files too). Set BGEN_BLESS=1 to rewrite the fixture's expected files.

use std::fs;
use std::path::Path;

fn compare(path: &Path, actual: &str) {
    if std::env::var_os("BGEN_BLESS").is_some() {
        fs::write(path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_default();
    assert!(
        expected == actual,
        "{} differs (BGEN_BLESS=1 to update):\n{actual}",
        path.display()
    );
}

#[test]
fn fixture() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let decls = fs::read_to_string(dir.join("intrinsics.d.ts")).unwrap();
    let forms = fs::read_to_string(dir.join("forms.ts")).unwrap();
    let out = djs_bgen::compile(
        ("golden/intrinsics.d.ts", &decls),
        &[("golden/forms.ts".into(), forms)],
    )
    .unwrap_or_else(|e| panic!("{e}"));
    compare(&dir.join("forms.js"), &out.prelude);
    compare(&dir.join("forms.report.txt"), &out.report);
}

/// The Rust backend (0063) on a fixture of @native builtins.
#[test]
fn native_fixture() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let decls = fs::read_to_string(dir.join("intrinsics.d.ts")).unwrap();
    let native = fs::read_to_string(dir.join("native.ts")).unwrap();
    let out = djs_bgen::compile(
        ("golden/intrinsics.d.ts", &decls),
        &[("golden/native.ts".into(), native)],
    )
    .unwrap_or_else(|e| panic!("{e}"));
    compare(&dir.join("native.rs.txt"), &out.natives);
    compare(&dir.join("native.js"), &out.prelude);
}

/// The same files `scripts/bgen.sh --check` compares, so the gate needs no run of the
/// djs-bgen program of its own (0197).
#[test]
fn builtins_gen_is_fresh() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../builtins");
    let out = djs_bgen::compile_dir(&dir).unwrap_or_else(|e| panic!("{e}"));
    for (path, text) in [
        (djs_bgen::PRELUDE_PATH, &out.prelude),
        (djs_bgen::PRELUDE_SLIM_PATH, &out.prelude_slim),
        (djs_bgen::REPORT_PATH, &out.report),
        (djs_bgen::NATIVES_PATH, &out.natives),
    ] {
        let on_disk = fs::read_to_string(dir.join(path)).unwrap_or_default();
        assert!(
            &on_disk == text,
            "builtins/{path} is out of date: run scripts/bgen.sh"
        );
    }
}
