//! Runs the fixture suite in `tests/fixture/test262` with the qjs engine and checks that
//! every `pass/` test passes and every `fail/` test fails for the expected reason.
//! This guards against a runner that is too lenient, which a run of the real test262
//! with qjs (which passes nearly everything) would not reveal.
//!
//! Skipped (with a note) when the qjs binary is not available.

use std::path::Path;
use std::time::Duration;

use djs_test262::config::Config;
use djs_test262::engine::Verdict;
use djs_test262::qjs::Qjs;
use djs_test262::runner::{self, Options};
use djs_test262::suite::Suite;

const CONFIG: &str = r#"
[[group]]
name = "later"
kind = "later"
description = "features implied by includes count"
features = ["implied-*"]
"#;

/// Expected failure reasons (substrings) for the `fail/` tests.
const FAILURES: &[(&str, &str)] = &[
    (
        "fail/assert.js",
        "Test262Error: one is two Expected SameValue(1, 2)",
    ),
    ("fail/async-failure.js", "async: TypeError: nope"),
    ("fail/async-never.js", "async: $DONE was not called"),
    ("fail/async-sync-throw.js", "RangeError: sync part"),
    ("fail/async-twice.js", "async: $DONE was called 2 times"),
    ("fail/bad-frontmatter.js", "bad frontmatter"),
    (
        "fail/harness-throws.js",
        "harness broken.js: TypeError: bad harness",
    ),
    ("fail/missing-include.js", "missing harness file nope.js"),
    (
        "fail/negative-not-thrown.js",
        "expected runtime TypeError, but no error was thrown",
    ),
    (
        "fail/negative-wrong-phase.js",
        "expected parse SyntaxError, got runtime SyntaxError: late",
    ),
    (
        "fail/negative-wrong-type.js",
        "expected runtime TypeError, got runtime ReferenceError",
    ),
    ("fail/parse-error.js", "parse: SyntaxError"),
    ("fail/strict-only-parse-error.js", "parse: SyntaxError"),
    ("fail/tamper-throw.js", "Test262Error: after tampering é\n"),
    ("fail/throw-primitive.js", "number: 1"),
    ("fail/throws.js", "Test262Error: boom"),
    ("fail/timeout.js", "timeout"),
];

#[test]
fn fixture_suite_with_qjs() {
    let qjs = Qjs::default_binary();
    if !qjs.is_file() {
        eprintln!("skipping: qjs not found at {} (set DJS_QJS)", qjs.display());
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/test262");
    let suite = Suite::open(&root).unwrap();
    let config = Config::parse(CONFIG).unwrap();
    let paths = suite.discover(&[]).unwrap();
    let tests = suite.load(&config, paths, 4);

    let include = tests.iter().find(|t| t.path == "pass/include.js").unwrap();
    assert_eq!(config.groups[include.group].name, "later");
    assert_eq!(include.features, ["implied-feature"]);

    let engine = Qjs { binary: qjs };
    let opts = Options {
        jobs: 8,
        timeout: Duration::from_secs(1),
        memory_limit: None,
        progress: false,
    };
    let refs: Vec<_> = tests.iter().collect();
    let outcomes = runner::run_all(&engine, &suite, &refs, &opts);

    let mut problems = Vec::new();
    let mut seen_failures = 0;
    for (t, o) in tests.iter().zip(&outcomes) {
        match (&o.verdict, t.path.starts_with("pass/")) {
            (Verdict::Pass, true) => {}
            (Verdict::Fail(r), true) => {
                problems.push(format!("{} should pass, failed: {r}", t.path))
            }
            (Verdict::Pass, false) => problems.push(format!("{} should fail, passed", t.path)),
            (Verdict::Fail(r), false) => {
                seen_failures += 1;
                match FAILURES.iter().find(|(p, _)| *p == t.path) {
                    Some((_, want)) if r.contains(want) => {}
                    Some((_, want)) => {
                        problems.push(format!("{}: reason `{r}` lacks `{want}`", t.path))
                    }
                    None => {
                        problems.push(format!("{}: no expected reason listed (got `{r}`)", t.path))
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
    assert_eq!(seen_failures, FAILURES.len());
    // The timeout test must not hold the run up for long.
    let timeout = tests
        .iter()
        .position(|t| t.path == "fail/timeout.js")
        .unwrap();
    assert!(outcomes[timeout].elapsed < Duration::from_secs(3));
}
