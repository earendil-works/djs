//! Runner behavior on a scratch corpus: comparison, missing expectations, timeouts,
//! `skip-djs`, engine panics, and `.expected` collisions. The qjs part is skipped (with a
//! note) when qjs is not available.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use djs_difftest::case;
use djs_difftest::engine::{Done, Engine, Job, RunResult};
use djs_difftest::qjs::Qjs;
use djs_difftest::runner::{self, Options, Verdict};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("djs-difftest-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("area")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/prelude.js"),
        dir.join("prelude.js"),
    )
    .unwrap();
    dir
}

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).unwrap();
}

/// Generous: these cases take milliseconds, and a loaded machine must not turn that
/// into a timeout (0020). Only `timeout.js` has a short limit, via its directive.
fn opts(is_djs: bool) -> Options {
    Options {
        jobs: 4,
        timeout: Duration::from_secs(60),
        memory_limit: None,
        progress: false,
        is_djs,
    }
}

#[test]
fn qjs_comparisons() {
    let qjs = Qjs::default_binary();
    if !qjs.is_file() {
        eprintln!("skipping: qjs not found at {} (set DJS_QJS)", qjs.display());
        return;
    }
    let dir = scratch("qjs");
    write(
        &dir,
        "area/pass.js",
        "console.log('hi', {a: 1});\nreturn [1];\n",
    );
    write(
        &dir,
        "area/pass.expected",
        "hi {\"a\":1}\n=== return\n[1]\n",
    );
    write(&dir, "area/differs.js", "console.log('actual');\n");
    write(&dir, "area/differs.expected", "expected\n=== completed\n");
    write(&dir, "area/missing.js", "1;\n");
    write(
        &dir,
        "area/timeout.js",
        "// difftest: timeout 0.5\nfor (;;) {}\n",
    );
    write(&dir, "area/timeout.expected", "=== completed\n");
    // Stack frames are not compared: an expected file without them matches.
    write(
        &dir,
        "area/stack.js",
        "function f() { throw new Error('deep'); }\nf();\n",
    );
    write(&dir, "area/stack.expected", "=== throw\nError: deep\n");
    write(&dir, "area/typed.ts", "const n: number = 1;\nreturn n;\n");
    write(&dir, "area/typed.expected", "=== return\n1\n");

    let cases = case::discover(&dir, &[]).unwrap();
    let engine = Qjs { binary: qjs };
    let outcomes = runner::run_all(&engine, &cases, &dir.join("prelude.js"), &opts(false)).unwrap();
    let verdict = |name: &str| {
        let i = cases.iter().position(|c| c.name == name).unwrap();
        (&outcomes[i].verdict, &outcomes[i].done)
    };
    assert_eq!(verdict("area/pass.js").0, &Verdict::Pass);
    assert_eq!(verdict("area/stack.js").0, &Verdict::Pass);
    assert_eq!(verdict("area/typed.ts").0, &Verdict::Pass);
    match verdict("area/differs.js").0 {
        Verdict::Fail { reason, diff } => {
            assert_eq!(reason, "line 1: expected `expected`, got `actual`");
            assert!(
                diff.contains("-expected") && diff.contains("+actual"),
                "{diff}"
            );
        }
        v => panic!("{v:?}"),
    }
    assert!(
        matches!(verdict("area/missing.js").0, Verdict::Fail { reason, .. } if reason.contains("no .expected"))
    );
    let (v, done) = verdict("area/timeout.js");
    assert!(
        matches!(v, Verdict::Fail { reason, .. } if reason == "timeout: no result within 0.5s"),
        "{v:?}"
    );
    assert_eq!(done, &Some(Done::Timeout));
    fs::remove_dir_all(&dir).unwrap();
}

/// An in-process engine that prints the case source and returns.
struct Echo;

impl Engine for Echo {
    fn name(&self) -> &str {
        "echo"
    }
    fn run(&self, job: &Job<'_>) -> RunResult {
        assert!(job.prelude.contains("(function (host)"));
        if job.source.contains("panic") {
            panic!("engine bug");
        }
        RunResult {
            output: vec![job.source.trim().to_string()],
            done: Done::Completed,
        }
    }
}

#[test]
fn fake_engine_skips_and_panics() {
    let dir = scratch("fake");
    write(&dir, "area/a.js", "// difftest: fuel 5\nhello");
    write(
        &dir,
        "area/a.expected",
        "// difftest: fuel 5\nhello\n=== completed\n",
    );
    write(
        &dir,
        "area/skipped.js",
        "// difftest: skip-djs needs eval\nx",
    );
    write(&dir, "area/panic.js", "panic");
    let cases = case::discover(&dir, &[]).unwrap();
    assert_eq!(
        cases
            .iter()
            .find(|c| c.name == "area/a.js")
            .unwrap()
            .meta
            .fuel,
        Some(5)
    );
    let outcomes = runner::run_all(&Echo, &cases, &dir.join("prelude.js"), &opts(true)).unwrap();
    let names: Vec<&str> = cases.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["area/a.js", "area/panic.js", "area/skipped.js"]);
    assert_eq!(outcomes[0].verdict, Verdict::Pass);
    assert_eq!(
        outcomes[1].done,
        Some(Done::Crash("engine panicked: engine bug".into()))
    );
    assert_eq!(outcomes[2].verdict, Verdict::Skipped("needs eval".into()));
    // Not djs: skip-djs does not apply.
    let outcomes = runner::run_all(&Echo, &cases, &dir.join("prelude.js"), &opts(false)).unwrap();
    assert!(matches!(outcomes[2].verdict, Verdict::Fail { .. }));

    write(&dir, "area/a.ts", "x");
    assert!(
        case::discover(&dir, &[])
            .unwrap_err()
            .contains("share one .expected")
    );
    // Filters select by prefix or glob.
    fs::remove_file(dir.join("area/a.ts")).unwrap();
    assert_eq!(case::discover(&dir, &["area/s*".into()]).unwrap().len(), 1);
    assert_eq!(case::discover(&dir, &["area".into()]).unwrap().len(), 3);
    fs::remove_dir_all(&dir).unwrap();
}
