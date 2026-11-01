use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use djs_test262::cache::{Cached, QjsCache};
use djs_test262::config::{Config, Kind};
use djs_test262::engine::{Engine, Verdict};
use djs_test262::par;
use djs_test262::qjs::Qjs;
use djs_test262::ratchet;
use djs_test262::report::{self, ReportOptions};
use djs_test262::runner::{self, Outcome};
use djs_test262::suite::{self, Suite, TestCase};

const USAGE: &str = "\
usage: djs-test262 <list|run> [options] [filter...]

Filters select tests by path relative to test/: a directory or file prefix
(built-ins/Array) or a glob (built-ins/*/from/*.js, language/**/S11*.js).

commands:
  list                classify the selected tests and print counts per group and dir
  run                 run the selected tests and print a summary

options:
  --group NAME        only tests in this group (repeatable). Default for run: all
                      active and later groups; for list: all groups
  --depth N           directory depth of the per-dir table (default 1)
  --features          also print the per-feature table
  -j, --jobs N        worker threads (default: $DJS_JOBS, else the number of CPUs)
  --test262 DIR       test262 checkout (default: $DJS_TEST262, else vendor/test262
                      in the primary checkout)
  --config FILE       grouping file (default: tests/test262.toml)

list options:
  --paths             print every selected test with its group and matching rule

run options:
  --engine NAME       qjs (default) or djs (in-process)
  --backend B         djs only: run the @native builtins as `native` code (default)
                      or as `bytecode` (0063)
  --qjs PATH          qjs binary (default: $DJS_QJS, else ~/Development/dquickts/qjs)
  --no-cache          qjs: run every test instead of reusing cached results (the
                      cache sits next to the test262 checkout, keyed by the qjs
                      binary, the test262 commit and the driver; D1531)
  --timeout SECS      wall-clock limit per test (default 20)
  --memory-limit N    memory limit per test in bytes, K/M/G suffixes allowed
                      (default 512M; 0 means no limit)
  --failures          list failing tests with a one-line reason
  --clusters          group the failures by reason (values normalized), largest first
  --slowest N         list the N slowest tests
  --expected FILE     compare against a failures file (`path: reason` lines);
                      exit 1 on failures not listed there, report stale entries
  --update            rewrite the --expected file from this run
  -q, --quiet         no progress line
";

struct Args {
    command: String,
    filters: Vec<String>,
    groups: Vec<String>,
    depth: usize,
    features: bool,
    jobs: usize,
    test262: Option<PathBuf>,
    config: PathBuf,
    paths: bool,
    bytecode_builtins: bool,
    engine: String,
    qjs: PathBuf,
    timeout: Duration,
    memory_limit: Option<u64>,
    failures: bool,
    no_cache: bool,
    clusters: bool,
    slowest: usize,
    expected: Option<PathBuf>,
    update: bool,
    quiet: bool,
}

fn parse_size(s: &str) -> Result<u64, String> {
    let (num, mul) = match s.as_bytes().last() {
        Some(b'K' | b'k') => (&s[..s.len() - 1], 1u64 << 10),
        Some(b'M' | b'm') => (&s[..s.len() - 1], 1 << 20),
        Some(b'G' | b'g') => (&s[..s.len() - 1], 1 << 30),
        _ => (s, 1),
    };
    num.parse::<u64>()
        .map(|n| n * mul)
        .map_err(|_| format!("bad size `{s}`"))
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let mut args = Args {
        command: String::new(),
        filters: Vec::new(),
        groups: Vec::new(),
        depth: 1,
        features: false,
        jobs: par::default_jobs(),
        test262: None,
        config: suite::workspace_root().join("tests/test262.toml"),
        paths: false,
        bytecode_builtins: false,
        engine: "qjs".into(),
        qjs: Qjs::default_binary(),
        timeout: Duration::from_secs(20),
        memory_limit: Some(512 << 20),
        failures: false,
        no_cache: false,
        clusters: false,
        slowest: 0,
        expected: None,
        update: false,
        quiet: false,
    };
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        let number = |v: String, name: &str| {
            v.parse::<usize>()
                .map_err(|_| format!("{name}: bad number `{v}`"))
        };
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--group" => args.groups.push(value(&a)?),
            "--depth" => args.depth = number(value(&a)?, &a)?.max(1),
            "--features" => args.features = true,
            "-j" | "--jobs" => args.jobs = number(value(&a)?, &a)?.max(1),
            "--test262" => args.test262 = Some(value(&a)?.into()),
            "--config" => args.config = value(&a)?.into(),
            "--paths" => args.paths = true,
            "--engine" => args.engine = value(&a)?,
            "--backend" => match value(&a)?.as_str() {
                "native" => args.bytecode_builtins = false,
                "bytecode" => args.bytecode_builtins = true,
                other => return Err(format!("--backend: native or bytecode, not `{other}`")),
            },
            "--qjs" => args.qjs = value(&a)?.into(),
            "--timeout" => {
                let v = value(&a)?;
                let secs: f64 = v
                    .parse()
                    .map_err(|_| format!("--timeout: bad number `{v}`"))?;
                args.timeout = Duration::from_secs_f64(secs);
            }
            "--memory-limit" => {
                let n = parse_size(&value(&a)?)?;
                args.memory_limit = (n > 0).then_some(n);
            }
            "--no-cache" => args.no_cache = true,
            "--failures" => args.failures = true,
            "--clusters" => args.clusters = true,
            "--slowest" => args.slowest = number(value(&a)?, &a)?,
            "--expected" => args.expected = Some(value(&a)?.into()),
            "--update" => args.update = true,
            "-q" | "--quiet" => args.quiet = true,
            _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
            _ if args.command.is_empty() => args.command = a,
            _ => args.filters.push(a),
        }
    }
    match args.command.as_str() {
        "list" | "run" => {}
        "" => return Err("missing command".into()),
        c => return Err(format!("unknown command `{c}`")),
    }
    if args.update && args.expected.is_none() {
        return Err("--update needs --expected FILE".into());
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("djs-test262: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match real_main(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("djs-test262: {e}");
            ExitCode::from(2)
        }
    }
}

fn real_main(args: &Args) -> Result<ExitCode, String> {
    let config = Config::load(&args.config)?;
    let root = match &args.test262 {
        Some(p) => p.clone(),
        None => suite::locate_test262()?,
    };
    let suite = Suite::open(&root)?;
    let commit = suite::checkout_commit(&root);
    if commit.as_deref() != Some(suite::PINNED_COMMIT) {
        eprintln!(
            "djs-test262: warning: {} is at {}, not the pinned {}",
            root.display(),
            commit.as_deref().unwrap_or("an unknown commit"),
            suite::PINNED_COMMIT
        );
    }

    // Group selection.
    let mut selected = vec![false; config.groups.len()];
    if args.groups.is_empty() {
        for (i, g) in config.groups.iter().enumerate() {
            selected[i] = args.command == "list" || g.kind != Kind::Skip;
        }
    } else {
        for name in &args.groups {
            let i = config.group_index(name).ok_or_else(|| {
                let names: Vec<&str> = config.groups.iter().map(|g| g.name.as_str()).collect();
                format!("unknown group `{name}` (groups: {})", names.join(", "))
            })?;
            selected[i] = true;
        }
    }

    let paths = suite.discover(&args.filters)?;
    if paths.is_empty() {
        return Err("no tests match the filters".into());
    }
    let all = suite.load(&config, paths, args.jobs);
    let bad: Vec<&TestCase> = all.iter().filter(|t| t.error.is_some()).collect();
    for t in &bad {
        eprintln!(
            "djs-test262: warning: {}: {}",
            t.path,
            t.error.as_deref().unwrap_or("")
        );
    }
    let short = commit
        .as_deref()
        .map(|c| &c[..c.len().min(7)])
        .unwrap_or("?")
        .to_string();
    let report_opts = ReportOptions {
        depth: args.depth,
        features: args.features,
    };

    if args.command == "list" {
        let tests: Vec<TestCase> = all.into_iter().filter(|t| selected[t.group]).collect();
        if args.paths {
            for t in &tests {
                println!("{} [{}: {}]", t.path, config.groups[t.group].name, t.reason);
            }
            return Ok(ExitCode::SUCCESS);
        }
        println!("test262 @{short}: {} tests\n", tests.len());
        let none: Vec<Option<&Outcome>> = vec![None; tests.len()];
        print!("{}", report::summary(&config, &tests, &none, &report_opts));
        return Ok(ExitCode::SUCCESS);
    }

    // run
    let engine: Box<dyn Engine> = match args.engine.as_str() {
        "qjs" => {
            if !args.qjs.is_file() {
                return Err(format!(
                    "qjs binary not found at {} (use --qjs or DJS_QJS)",
                    args.qjs.display()
                ));
            }
            Box::new(Qjs {
                binary: args.qjs.clone(),
            })
        }
        "djs" => Box::new(djs_test262::djs::Djs {
            bytecode_builtins: args.bytecode_builtins,
            ..Default::default()
        }),
        e => return Err(format!("unknown engine `{e}`")),
    };
    // qjs results only change with the binary, the test262 commit or the driver (D1531).
    let cache = match (args.engine.as_str(), &commit) {
        ("qjs", Some(commit)) => Some(QjsCache::open(
            &root.parent().unwrap_or(&root).join("qjs-cache"),
            &args.qjs,
            commit,
            djs_test262::qjs::DRIVER,
            args.memory_limit,
            !args.no_cache,
        )?),
        _ => None,
    };
    let cached;
    let engine: &dyn Engine = match &cache {
        Some(cache) => {
            cached = Cached {
                inner: engine.as_ref(),
                cache,
            };
            &cached
        }
        None => engine.as_ref(),
    };
    let (run_tests, other): (Vec<TestCase>, Vec<TestCase>) =
        all.into_iter().partition(|t| selected[t.group]);
    if run_tests.is_empty() {
        return Err(
            "no tests selected (the matching tests are in skip groups; see --group)".into(),
        );
    }
    let opts = runner::Options {
        jobs: args.jobs,
        timeout: args.timeout,
        memory_limit: args.memory_limit,
        progress: !args.quiet,
    };
    let start = Instant::now();
    let refs: Vec<&TestCase> = run_tests.iter().collect();
    let outcomes = runner::run_all(engine, &suite, &refs, &opts);
    let elapsed = start.elapsed();
    let cache_note = match &cache {
        Some(c) => {
            c.save()?;
            format!(" ({} cached)", c.hits())
        }
        None => String::new(),
    };

    // The tables also count the unselected tests (as not run), so skip shows up.
    let mut tests = run_tests;
    let ran = tests.len();
    let mut outs: Vec<Option<&Outcome>> = outcomes.iter().map(Some).collect();
    tests.extend(other);
    outs.resize(tests.len(), None);

    println!(
        "test262 @{short} · engine {} · {ran} tests{cache_note} in {:.1}s (-j {})\n",
        engine.name(),
        elapsed.as_secs_f64(),
        args.jobs
    );
    print!("{}", report::summary(&config, &tests, &outs, &report_opts));

    // Reasons are single lines of bounded length, so the failures file stays readable.
    let reasons: Vec<Option<String>> = outcomes
        .iter()
        .map(|o| match &o.verdict {
            Verdict::Pass => None,
            Verdict::Fail(r) => Some(one_line_reason(r)),
        })
        .collect();
    let results: Vec<(&str, Option<&str>)> = tests[..ran]
        .iter()
        .zip(&reasons)
        .map(|(t, r)| (t.path.as_str(), r.as_deref()))
        .collect();

    if args.failures {
        let failed: Vec<_> = results
            .iter()
            .filter_map(|(p, r)| r.map(|r| (p, r)))
            .collect();
        println!("\nfailures ({}):", failed.len());
        for (p, r) in failed {
            println!("{p}: {r}");
        }
    }
    if args.clusters {
        let failed: Vec<(&str, &str)> = results
            .iter()
            .filter_map(|(p, r)| r.map(|r| (*p, r)))
            .collect();
        println!("\nfailure clusters ({} failures):", failed.len());
        print!("{}", djs_test262::report::clusters(&failed, 40));
    }
    if args.slowest > 0 {
        let mut by_time: Vec<(&TestCase, &Outcome)> = tests[..ran].iter().zip(&outcomes).collect();
        by_time.sort_by_key(|a| std::cmp::Reverse(a.1.elapsed));
        println!("\nslowest:");
        for (t, o) in by_time.iter().take(args.slowest) {
            println!("{:8.2}s {}", o.elapsed.as_secs_f64(), t.path);
        }
    }

    let failed_count = results.iter().filter(|(_, r)| r.is_some()).count();
    let Some(expected_path) = &args.expected else {
        return Ok(if failed_count == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    };
    let expected = ratchet::load(expected_path)?;
    if args.update {
        let updated = ratchet::update(&expected, &results);
        ratchet::save(expected_path, &updated)?;
        println!(
            "\nratchet: wrote {} entries to {}",
            updated.len(),
            expected_path.display()
        );
        return Ok(ExitCode::SUCCESS);
    }
    let cmp = ratchet::compare(&expected, &results);
    println!();
    if !cmp.stale.is_empty() {
        println!(
            "ratchet: {} stale entries (these pass now; run with --update):",
            cmp.stale.len()
        );
        for p in &cmp.stale {
            println!("  {p}");
        }
    }
    if cmp.new_failures.is_empty() {
        println!("ratchet: OK, no new failures ({failed_count} expected)");
        Ok(ExitCode::SUCCESS)
    } else {
        println!(
            "ratchet: {} new failures (not in {}):",
            cmp.new_failures.len(),
            expected_path.display()
        );
        for (p, r) in &cmp.new_failures {
            println!("  {p}: {r}");
        }
        Ok(ExitCode::from(1))
    }
}

fn one_line_reason(r: &str) -> String {
    const MAX: usize = 160;
    let mut s: String = r.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > MAX {
        s = s.chars().take(MAX - 1).collect();
        s.push('…');
    }
    s
}
