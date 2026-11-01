//! Checks the jobs the runner prepares, with fake in-process engines (no qjs needed).

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use djs_test262::config::Config;
use djs_test262::engine::{Engine, Job, RunResult, Status, Verdict};
use djs_test262::runner::{self, Options};
use djs_test262::suite::{Suite, TestCase};

/// (path, harness names, strict, async)
type Recorded = (String, Vec<String>, bool, bool);

/// Records every job and reports success.
struct Recorder(Mutex<Vec<Recorded>>);

impl Engine for Recorder {
    fn name(&self) -> &str {
        "recorder"
    }
    fn run(&self, job: &Job<'_>) -> RunResult {
        assert!(job.source.contains("/*---"));
        assert!(job.file.is_file());
        let names = job.harness.iter().map(|h| h.name.clone()).collect();
        self.0
            .lock()
            .unwrap()
            .push((job.path.to_string(), names, job.strict, job.is_async));
        RunResult::new(Status::Completed)
    }
}

struct Panicker;

impl Engine for Panicker {
    fn name(&self) -> &str {
        "panicker"
    }
    fn run(&self, _: &Job<'_>) -> RunResult {
        panic!("engine bug");
    }
}

fn fixture() -> (Suite, Vec<TestCase>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/test262");
    let suite = Suite::open(&root).unwrap();
    let paths = suite.discover(&["pass".to_string()]).unwrap();
    let tests = suite.load(&Config::parse("").unwrap(), paths, 2);
    (suite, tests)
}

fn opts() -> Options {
    Options {
        jobs: 4,
        timeout: Duration::from_secs(5),
        memory_limit: None,
        progress: false,
    }
}

#[test]
fn jobs_follow_the_execution_model() {
    let (suite, tests) = fixture();
    let rec = Recorder(Mutex::new(Vec::new()));
    let refs: Vec<_> = tests.iter().collect();
    runner::run_all(&rec, &suite, &refs, &opts());
    let jobs = rec.0.into_inner().unwrap();
    let find = |p: &str| {
        jobs.iter()
            .find(|j| j.0 == p)
            .unwrap_or_else(|| panic!("no job for {p}"))
    };

    let (_, harness, strict, is_async) = find("pass/plain.js");
    assert_eq!(harness, &["assert.js", "sta.js"]);
    assert!(*strict && !*is_async);

    let (_, harness, _, is_async) = find("pass/async.js");
    assert_eq!(harness, &["assert.js", "sta.js", "doneprintHandle.js"]);
    assert!(*is_async);

    let (_, harness, ..) = find("pass/include.js");
    assert_eq!(harness, &["assert.js", "sta.js", "inc.js"]);

    let (_, harness, strict, _) = find("pass/raw-sloppy.js");
    assert!(harness.is_empty() && !*strict);
}

#[test]
fn engine_panics_fail_only_that_test() {
    let (suite, tests) = fixture();
    let refs: Vec<_> = tests.iter().take(2).collect();
    let outcomes = runner::run_all(&Panicker, &suite, &refs, &opts());
    assert_eq!(outcomes.len(), 2);
    for o in outcomes {
        assert_eq!(
            o.verdict,
            Verdict::Fail("crash: engine panicked: engine bug".into())
        );
    }
}
