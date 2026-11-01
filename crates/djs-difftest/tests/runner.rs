//! Runner behavior on a scratch corpus: comparison, missing expectations, timeouts,
//! `skip-djs`, engine panics, and `.expected` collisions. The qjs part is skipped (with a
//! note) when qjs is not available.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use djs_difftest::case;
use djs_difftest::djs::Djs;
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
        stress: Vec::new(),
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

/// Cases that djs can already run, whose transcripts must be the same on qjs (with the
/// shared prelude) and on djs (with its native codemode globals, D1160).
const GLUE_CASES: &[(&str, &str)] = &[
    (
        "console.js",
        "console.log(\"str\", 1, -0, 1.5, NaN, true, null, undefined, [1, \"a\", null], {a: 1, b: [2]});\nconsole.info(\"info\"); console.warn(\"w\", 2); console.error(); console.debug(\"d\");",
    ),
    (
        "text.js",
        "text(\"s\"); text(-0); text(NaN); text(undefined); text(null); text(false); text([1, NaN]); text({x: \"y\"});",
    ),
    (
        "ret-string.js",
        "console.log(\"before\");\nreturn \"the\\nresult\";",
    ),
    ("ret-object.js", "return {a: [1, 2], b: null, c: \"x\"};"),
    ("ret-undefined.js", "let x = 1;"),
    ("ret-function.js", "return function f() {};"),
    ("ret-number.js", "return -0;"),
    ("throw-string.js", "throw \"boom\";"),
    ("throw-number.js", "throw 42;"),
    ("throw-object.js", "throw {code: 1};"),
    ("reference.js", "missingName + 1;"),
    (
        "exit.js",
        "console.log(\"a\");\nexit();\nconsole.log(\"never\");",
    ),
    (
        "store.js",
        "store(\"k\", {v: [1, 2]});\nconsole.log(load(\"k\"), load(\"nope\"));\nstore(\"k\", undefined);\nconsole.log(load(\"k\"));",
    ),
    ("syntax.js", "let = ;"),
];

fn glue_corpus(name: &str) -> (PathBuf, Vec<case::Case>) {
    let dir = scratch(name);
    for (name, src) in GLUE_CASES {
        write(&dir, &format!("area/{name}"), src);
    }
    // djs-only expectations: tools return directly until djs has promises.
    write(
        &dir,
        "area/tools.js",
        "console.log(await tools.echo({p: \"x\", n: [1, true]}), ALL_TOOLS.length);",
    );
    write(&dir, "area/fail.js", "await tools.fail({message: \"denied\"});");
    write(
        &dir,
        "area/fuel.js",
        "// difftest: fuel 5000\n// difftest: timeout 0.5\nfor (;;) {}",
    );
    let cases = case::discover(&dir, &[]).unwrap();
    (dir, cases)
}

#[test]
fn djs_glue_matches_qjs() {
    let (dir, cases) = glue_corpus("glue-qjs");
    let prelude = dir.join("prelude.js");
    let djs_out = runner::run_all(&Djs::default(), &cases, &prelude, &opts(true)).unwrap();
    let by_name = |name: &str| {
        let i = cases
            .iter()
            .position(|c| c.name == format!("area/{name}"))
            .unwrap();
        &djs_out[i]
    };
    assert_eq!(
        by_name("tools.js").transcript,
        "{\"p\":\"x\",\"n\":[1,true]} 2\n=== completed\n"
    );
    assert_eq!(by_name("fail.js").transcript, "=== throw\nError: denied\n");
    assert_eq!(by_name("fuel.js").done, Some(Done::Timeout));

    let qjs = Qjs::default_binary();
    if !qjs.is_file() {
        eprintln!("skipping the qjs half: qjs not found at {}", qjs.display());
        return;
    }
    let qjs_out = runner::run_all(&Qjs { binary: qjs }, &cases, &prelude, &opts(false)).unwrap();
    let mut problems = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let name = c.name.trim_start_matches("area/");
        if GLUE_CASES.iter().any(|(n, _)| *n == name)
            && qjs_out[i].transcript != djs_out[i].transcript
        {
            problems.push(format!(
                "{name}:\n  qjs: {:?}\n  djs: {:?}",
                qjs_out[i].transcript, djs_out[i].transcript
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn djs_stress_variants_agree() {
    let (dir, cases) = glue_corpus("glue-stress");
    let mut o = opts(true);
    o.stress = vec![
        (
            "suspend".into(),
            Box::new(Djs {
                suspend_every: Some(7),
                gc_stress: false,
            }),
        ),
        (
            "gc".into(),
            Box::new(Djs {
                suspend_every: None,
                gc_stress: true,
            }),
        ),
    ];
    let out = runner::run_all(&Djs::default(), &cases, &dir.join("prelude.js"), &o).unwrap();
    for (c, o) in cases.iter().zip(&out) {
        // A fuel limit of 5000 at 7 per slice still ends in a timeout: same transcript.
        assert!(o.stress.is_empty(), "{}: {:?}", c.name, o.stress);
    }
    // A stressed engine that behaves differently is reported.
    struct Liar;
    impl Engine for Liar {
        fn name(&self) -> &str {
            "liar"
        }
        fn run(&self, _: &Job<'_>) -> RunResult {
            RunResult {
                output: vec!["lie".into()],
                done: Done::Completed,
            }
        }
    }
    let mut o = opts(true);
    o.stress = vec![("liar".into(), Box::new(Liar))];
    let out = runner::run_all(&Djs::default(), &cases[..1], &dir.join("prelude.js"), &o).unwrap();
    assert_eq!(out[0].stress.len(), 1);
    assert_eq!(out[0].stress[0].label, "liar");
    fs::remove_dir_all(&dir).unwrap();
}
