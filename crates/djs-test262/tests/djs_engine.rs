//! The djs engine on a fixture suite that only uses what djs runs today
//! (tests/fixture/djs): every `pass/` test passes and every `fail/` test fails for the
//! expected reason. Covers Scripts and the harness, negative tests by phase, the
//! `Unsupported` parse errors, `$262` (global, evalScript, gc, detachArrayBuffer),
//! `print` and `$DONE`, and the fuel and memory limits.

use std::path::Path;
use std::time::Duration;

use djs_test262::config::Config;
use djs_test262::djs::Djs;
use djs_test262::engine::Verdict;
use djs_test262::runner::{self, Options};
use djs_test262::suite::Suite;

const FAILURES: &[(&str, &str)] = &[
    ("fail/assert.js", "Test262Error: one is two"),
    ("fail/async-failure.js", "async: bad"),
    ("fail/async-never.js", "async: $DONE was not called"),
    ("fail/detach.js", "TypeError: not an ArrayBuffer"),
    (
        "fail/harness-throws.js",
        "harness broken.js: TypeError: cannot read property 'x' of null",
    ),
    (
        "fail/negative-not-thrown.js",
        "expected runtime TypeError, but no error was thrown",
    ),
    (
        "fail/negative-wrong-type.js",
        "expected runtime TypeError, got runtime ReferenceError",
    ),
    ("fail/oom.js", "oom"),
    ("fail/parse-error.js", "parse: SyntaxError"),
    ("fail/throw-primitive.js", "number: 1"),
    ("fail/throws.js", "Test262Error: boom"),
    ("fail/timeout.js", "timeout: the fuel budget is used up"),
    (
        "fail/unsupported-negative.js",
        "expected parse SyntaxError, got parse Unsupported: generators are not supported yet",
    ),
];

#[test]
fn djs_engine_fixture_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/djs");
    let suite = Suite::open(&root).unwrap();
    let config = Config::parse("").unwrap();
    let tests = suite.load(&config, suite.discover(&[]).unwrap(), 4);
    // A small fuel budget keeps the infinite loop short; 4 MiB keeps the oom test quick.
    let engine = Djs { fuel: 2_000_000 };
    let opts = Options {
        jobs: 4,
        timeout: Duration::from_secs(60),
        memory_limit: Some(4 << 20),
        progress: false,
    };
    let refs: Vec<_> = tests.iter().collect();
    let outcomes = runner::run_all(&engine, &suite, &refs, &opts);

    let mut problems = Vec::new();
    let mut failures = 0;
    for (t, o) in tests.iter().zip(&outcomes) {
        match (&o.verdict, t.path.starts_with("pass/")) {
            (Verdict::Pass, true) => {}
            (Verdict::Fail(r), true) => {
                problems.push(format!("{} should pass, failed: {r}", t.path))
            }
            (Verdict::Pass, false) => problems.push(format!("{} should fail, passed", t.path)),
            (Verdict::Fail(r), false) => {
                failures += 1;
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
    assert_eq!(failures, FAILURES.len());
}
