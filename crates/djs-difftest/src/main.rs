use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use djs_difftest::case::{self, Case};
use djs_difftest::djs::Djs;
use djs_difftest::engine::{Done, Engine};
use djs_difftest::qjs::Qjs;
use djs_difftest::runner::{self, Outcome, Verdict};
use djs_test262::ratchet;
use djs_test262::report::table;

const USAGE: &str = "\
usage: djs-difftest [options] [filter...]

Runs the differential corpus (tests/corpus/<area>/<name>.{js,ts}) in the codemode
model and compares each transcript with <name>.expected. Filters select cases by path
relative to the corpus: a prefix (console) or a glob (dquickts/*/test_op*.js).

options:
  --engine NAME       qjs (default) or djs (in-process)
  --suspend-every N   djs only: also run every case suspending every N fuel through
                      heap images; its output must equal the plain run (hard failure)
  --gc-stress         djs only: also run every case collecting at every safepoint;
                      its output must equal the plain run (hard failure)
  --plain-dumps       djs only: also run every case writing a dump (a plain Script,
                      D2830) at each of its first 50 idle points and continuing in a
                      fresh VM that runs it as a Script (hard failure; a refused
                      dump ends the dumps, and the run goes on); with frozen builtins
                      (D2836), compared only for cases with idle points
  --plain-out DIR     --plain-dumps, writing each dump to DIR/<case>/<link>.js and the
                      output before it to <link>.pre (scripts/plain-node.mjs runs them)
  --plain-from K      with --plain-dumps: the chain starts at idle point K (the
                      earlier ones are not dumped)
  --determinism       djs only: also run every case suspending every 1009 fuel (and 97
                      without --suspend-every) through heap images, and require the
                      suspended runs to leave the same heap image as the plain run, raw
                      and by canonical hash (hard failure; GC stress is not compared, D1720)
                      The stressed runs get 3x the time limit; cases marked
                      `difftest: no-stress` skip them
  --backend B         djs only: run the @native builtins as `native` code (default)
                      or as `bytecode` (both must behave the same, 0063)
  --clusters          group the failures by the first line of what went wrong
  --idle-points       djs only: list the idle points (tool results delivered by
                      `host.later`, D2590) per case; the total is always printed
  --qjs PATH          qjs binary (default: $DJS_QJS, else ~/Development/dquickts/qjs)
  --update            rewrite .expected files from qjs (never from djs)
  --ratchet           compare failures with tests/corpus/expected-failures.txt;
                      exit 1 on failures not listed there
  --ratchet-file FILE use another failures file (implies --ratchet)
  --update-ratchet    rewrite the failures file from this run
  --no-diff           list failures without diffs
  --fuel N            djs only: fuel budget per case without a `difftest: fuel`
                      directive, the deterministic limit (default 1e9)
  --timeout SECS      wall-clock limit per case (default 10 for qjs; for djs only a hang
                      guard, 120, since fuel decides, D1790)
  --memory-limit N    memory limit per case in bytes, K/M/G suffixes (default 256M,
                      as in pi's codemode; 0 means no limit)
  -j, --jobs N        worker threads (default: $DJS_JOBS, else the number of CPUs)
  --corpus DIR        corpus root (default: tests/corpus)
  --times FILE        start the runs that took longest in FILE first, then rewrite FILE
                      with this run's times (the gate keeps one per leg)
  -q, --quiet         no progress line
";

struct Args {
    filters: Vec<String>,
    engine: String,
    qjs: PathBuf,
    update: bool,
    ratchet: Option<PathBuf>,
    update_ratchet: bool,
    diff: bool,
    timeout: Option<Duration>,
    fuel: Option<u64>,
    memory_limit: Option<u64>,
    jobs: usize,
    corpus: PathBuf,
    times: Option<PathBuf>,
    quiet: bool,
    suspend_every: Option<u64>,
    gc_stress: bool,
    plain_dumps: bool,
    plain_from: u32,
    determinism: bool,
    bytecode_builtins: bool,
    clusters: bool,
    idle_points: bool,
}

fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::canonicalize(&root).unwrap_or(root)
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
    let corpus = workspace_root().join("tests/corpus");
    let mut args = Args {
        filters: Vec::new(),
        engine: "qjs".into(),
        qjs: Qjs::default_binary(),
        update: false,
        ratchet: None,
        update_ratchet: false,
        diff: true,
        timeout: None,
        fuel: None,
        memory_limit: Some(256 << 20),
        jobs: djs_test262::par::default_jobs(),
        corpus,
        times: None,
        quiet: false,
        suspend_every: None,
        gc_stress: false,
        plain_dumps: false,
        plain_from: 0,
        determinism: false,
        bytecode_builtins: false,
        clusters: false,
        idle_points: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--engine" => args.engine = value(&a)?,
            "--qjs" => args.qjs = value(&a)?.into(),
            "--update" => args.update = true,
            "--ratchet" => {
                args.ratchet
                    .get_or_insert_with(|| PathBuf::from("expected-failures.txt"));
            }
            "--ratchet-file" => args.ratchet = Some(value(&a)?.into()),
            "--update-ratchet" => args.update_ratchet = true,
            "--no-diff" => args.diff = false,
            "--timeout" => {
                let v = value(&a)?;
                let secs: f64 = v
                    .parse()
                    .map_err(|_| format!("--timeout: bad number `{v}`"))?;
                args.timeout = Some(Duration::from_secs_f64(secs));
            }
            "--fuel" => {
                let v = value(&a)?;
                let n: f64 = v.parse().map_err(|_| format!("--fuel: bad number `{v}`"))?;
                args.fuel = Some(n as u64);
            }
            "--memory-limit" => {
                let n = parse_size(&value(&a)?)?;
                args.memory_limit = (n > 0).then_some(n);
            }
            "-j" | "--jobs" => {
                let v = value(&a)?;
                args.jobs = v
                    .parse::<usize>()
                    .map_err(|_| format!("-j: bad number `{v}`"))?
                    .max(1);
            }
            "--corpus" => args.corpus = value(&a)?.into(),
            "--times" => args.times = Some(value(&a)?.into()),
            "--suspend-every" => {
                let v = value(&a)?;
                let n = v
                    .parse::<u64>()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or_else(|| format!("--suspend-every: bad number `{v}`"))?;
                args.suspend_every = Some(n);
            }
            "--gc-stress" => args.gc_stress = true,
            "--plain-dumps" => args.plain_dumps = true,
            "--plain-out" => {
                args.plain_dumps = true;
                let _ = djs_difftest::djs::PLAIN_OUT.set(value(&a)?.into());
            }
            "--plain-from" => {
                let v = value(&a)?;
                args.plain_from = v
                    .parse()
                    .map_err(|_| format!("--plain-from: bad number `{v}`"))?;
            }
            "--determinism" => args.determinism = true,
            "--backend" => match value("--backend")?.as_str() {
                "native" => args.bytecode_builtins = false,
                "bytecode" => args.bytecode_builtins = true,
                other => return Err(format!("--backend: native or bytecode, not `{other}`")),
            },
            "--clusters" => args.clusters = true,
            "--idle-points" => args.idle_points = true,
            "-q" | "--quiet" => args.quiet = true,
            // `run` is accepted for symmetry with djs-test262.
            "run" if args.filters.is_empty() => {}
            _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
            _ => args.filters.push(a),
        }
    }
    if let Some(r) = &args.ratchet
        && r.is_relative()
        && r == Path::new("expected-failures.txt")
    {
        args.ratchet = Some(args.corpus.join(r));
    }
    if args.update_ratchet && args.ratchet.is_none() {
        args.ratchet = Some(args.corpus.join("expected-failures.txt"));
    }
    if (args.suspend_every.is_some() || args.gc_stress || args.determinism || args.plain_dumps)
        && args.engine != "djs"
    {
        return Err(
            "--suspend-every, --gc-stress, --determinism and --plain-dumps need --engine djs"
                .into(),
        );
    }
    if args.update && args.engine != "qjs" {
        return Err("--update records the oracle; it only works with --engine qjs".into());
    }
    Ok(args)
}

/// The idle points (D2590): a total line per area, and with `list` one line per case
/// that has any. Empty for engines without idle points.
fn idle_report(cases: &[Case], outcomes: &[Outcome], list: bool) -> String {
    let mut areas: BTreeMap<&str, (u32, usize, usize)> = BTreeMap::new();
    let mut lines = String::new();
    for (c, o) in cases.iter().zip(outcomes) {
        let Some(n) = o.idle_points else { continue };
        let e = areas.entry(c.area()).or_default();
        e.0 += n;
        e.1 += 1;
        if n > 0 {
            e.2 += 1;
            if list {
                lines += &format!("  {n:>5}  {}\n", c.name);
            }
        }
    }
    if areas.is_empty() {
        return String::new();
    }
    let (total, with) = areas
        .values()
        .fold((0, 0), |(t, w), &(n, _, k)| (t + n, w + k));
    let per_area: Vec<String> = areas
        .iter()
        .filter(|(_, (n, _, _))| *n > 0)
        .map(|(a, (n, _, k))| format!("{a} {n} in {k}"))
        .collect();
    let mut s = format!(
        "\nidle points: {total} in {with} cases ({})\n",
        per_area.join(", ")
    );
    s += &lines;
    s
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("djs-difftest: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match real_main(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("djs-difftest: {e}");
            ExitCode::from(2)
        }
    }
}

fn real_main(args: &Args) -> Result<ExitCode, String> {
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
        "djs" => Box::new(Djs {
            image: args.determinism,
            bytecode_builtins: args.bytecode_builtins,
            ..Djs::default()
        }),
        e => return Err(format!("unknown engine `{e}`")),
    };
    let cases = case::discover(&args.corpus, &args.filters)?;
    if cases.is_empty() {
        return Err("no cases match the filters".into());
    }
    let opts = runner::Options {
        jobs: args.jobs,
        // djs is limited by fuel, which does not depend on machine load; the wall clock
        // only guards against a hang outside the interpreter (D1790).
        timeout: args
            .timeout
            .unwrap_or(Duration::from_secs(if args.engine == "djs" {
                120
            } else {
                10
            })),
        fuel: args.fuel,
        memory_limit: args.memory_limit,
        progress: !args.quiet,
        is_djs: args.engine == "djs",
        stress: stress_variants(args),
        times: args.times.clone(),
    };
    let prelude = args.corpus.join("prelude.js");
    let start = Instant::now();
    let outcomes = runner::run_all(engine.as_ref(), &cases, &prelude, &opts)?;
    let elapsed = start.elapsed();

    if args.update {
        return update_expected(&cases, &outcomes);
    }

    let labels: Vec<&str> = opts.stress.iter().map(|(l, _)| l.as_str()).collect();
    println!(
        "difftest · engine {}{} · {} cases in {:.2}s\n",
        engine.name(),
        if labels.is_empty() {
            String::new()
        } else {
            format!(" (+ {})", labels.join(", "))
        },
        cases.len(),
        elapsed.as_secs_f64()
    );
    print!("{}", summary(&cases, &outcomes));
    print!("{}", idle_report(&cases, &outcomes, args.idle_points));

    let failures: Vec<(&Case, &str, &str)> = cases
        .iter()
        .zip(&outcomes)
        .filter_map(|(c, o)| match &o.verdict {
            Verdict::Fail { reason, diff } => Some((c, reason.as_str(), diff.as_str())),
            _ => None,
        })
        .collect();
    if !failures.is_empty() {
        println!("\nfailures ({}):", failures.len());
        for (c, reason, diff) in &failures {
            if args.diff && !diff.is_empty() {
                println!(
                    "\n--- {} (- expected, + actual)\n{}",
                    c.name,
                    diff.trim_end()
                );
            } else {
                println!("{}: {reason}", c.name);
            }
        }
    }

    if args.clusters && !failures.is_empty() {
        print!("\nfailure clusters:\n{}", clusters(&cases, &outcomes));
    }
    // Stress variants must behave exactly like the plain run: any difference, a timeout
    // included, is a hard failure (D1161, D1530).
    let mismatches: Vec<(&Case, &runner::StressMismatch)> = cases
        .iter()
        .zip(&outcomes)
        .flat_map(|(c, o)| o.stress.iter().map(move |m| (c, m)))
        .collect();
    if !labels.is_empty() {
        let unstressed = cases
            .iter()
            .zip(&outcomes)
            .filter(|(c, o)| {
                c.meta.no_stress.is_some() && !matches!(o.verdict, Verdict::Skipped(_))
            })
            .count();
        let note = if unstressed == 0 {
            String::new()
        } else {
            format!(" ({unstressed} cases marked no-stress)")
        };
        if mismatches.is_empty() {
            println!(
                "\nstress: OK, {} identical to the plain run{note}",
                labels.join(" and ")
            );
        } else {
            println!(
                "\nstress: {} transcripts differ from the plain run{note}:",
                mismatches.len()
            );
            for (c, m) in &mismatches {
                println!(
                    "\n--- {} [{}]: {} (- plain, + stressed)\n{}",
                    c.name,
                    m.label,
                    m.reason,
                    m.diff.trim_end()
                );
            }
        }
    }
    let stress_failed = !mismatches.is_empty();

    let Some(ratchet_file) = &args.ratchet else {
        return Ok(if failures.is_empty() && !stress_failed {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    };
    // Skipped cases did not run: their entries are neither stale nor replaced. Control
    // characters in a reason (a NUL from a failed string assertion) are escaped, so the
    // ratchet file stays text.
    let reasons: Vec<Option<String>> = outcomes
        .iter()
        .map(|o| match &o.verdict {
            Verdict::Fail { reason, .. } => Some(
                reason
                    .chars()
                    .map(|c| {
                        if c.is_control() {
                            format!("\\u{{{:x}}}", c as u32)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect(),
            ),
            _ => None,
        })
        .collect();
    let results: Vec<(&str, Option<&str>)> = cases
        .iter()
        .zip(&outcomes)
        .zip(&reasons)
        .filter_map(|((c, o), r)| match &o.verdict {
            Verdict::Pass => Some((c.name.as_str(), None)),
            Verdict::Fail { .. } => Some((c.name.as_str(), r.as_deref())),
            Verdict::Skipped(_) => None,
        })
        .collect();
    let expected = ratchet::load(ratchet_file)?;
    if args.update_ratchet {
        let updated = ratchet::update(&expected, &results);
        save_ratchet(ratchet_file, &updated)?;
        println!(
            "\nratchet: wrote {} entries to {}",
            updated.len(),
            ratchet_file.display()
        );
        return Ok(if stress_failed {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        });
    }
    let cmp = ratchet::compare(&expected, &results);
    println!();
    if !cmp.stale.is_empty() {
        println!(
            "ratchet: {} stale entries (these pass now; run with --update-ratchet):",
            cmp.stale.len()
        );
        for p in &cmp.stale {
            println!("  {p}");
        }
    }
    if cmp.new_failures.is_empty() {
        println!("ratchet: OK, no new failures ({} expected)", failures.len());
        Ok(if stress_failed {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        })
    } else {
        println!(
            "ratchet: {} new failures (not in {}):",
            cmp.new_failures.len(),
            ratchet_file.display()
        );
        for (p, r) in &cmp.new_failures {
            println!("  {p}: {r}");
        }
        Ok(ExitCode::from(1))
    }
}

fn save_ratchet(path: &Path, map: &ratchet::Expected) -> Result<(), String> {
    let mut text = String::from(
        "# difftest cases known to fail, one `case: reason` line each (paths relative to tests/corpus/).\n\
         # Generated by `djs-difftest --engine <engine> --update-ratchet`; do not edit by hand.\n",
    );
    for (name, reason) in map {
        text.push_str(&format!("{name}: {reason}\n"));
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the transcripts as `.expected`. Timeouts and crashes are never recorded: they
/// mean the case or the runner is broken.
fn update_expected(cases: &[Case], outcomes: &[Outcome]) -> Result<ExitCode, String> {
    let (mut added, mut changed, mut same) = (0, 0, 0);
    let mut errors = Vec::new();
    for (c, o) in cases.iter().zip(outcomes) {
        match &o.done {
            Some(Done::Timeout) => errors.push(format!("{}: timeout", c.name)),
            Some(Done::Crash(msg)) => errors.push(format!("{}: crash: {msg}", c.name)),
            None => {}
            Some(_) => {
                let old = fs::read_to_string(&c.expected_file).ok();
                match old {
                    Some(old) if old == o.transcript => same += 1,
                    old => {
                        fs::write(&c.expected_file, &o.transcript)
                            .map_err(|e| format!("{}: {e}", c.expected_file.display()))?;
                        if old.is_some() {
                            changed += 1;
                            println!("changed: {}", c.name);
                        } else {
                            added += 1;
                        }
                    }
                }
            }
        }
    }
    println!("update: {added} new, {changed} changed, {same} unchanged .expected files");
    if errors.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        println!("not recorded ({}):", errors.len());
        for e in &errors {
            println!("  {e}");
        }
        Ok(ExitCode::from(1))
    }
}

fn summary(cases: &[Case], outcomes: &[Outcome]) -> String {
    let mut by_area: BTreeMap<&str, [usize; 4]> = BTreeMap::new();
    for (c, o) in cases.iter().zip(outcomes) {
        let row = by_area.entry(c.area()).or_default();
        row[0] += 1;
        match o.verdict {
            Verdict::Pass => row[1] += 1,
            Verdict::Fail { .. } => row[2] += 1,
            Verdict::Skipped(_) => row[3] += 1,
        }
    }
    let mut total = [0; 4];
    let mut rows = Vec::new();
    for (area, r) in &by_area {
        for i in 0..4 {
            total[i] += r[i];
        }
        rows.push(row(area, r));
    }
    rows.push(row("total", &total));
    table(&["area", "cases", "pass", "fail", "skip"], &rows)
}

fn row(name: &str, r: &[usize; 4]) -> Vec<String> {
    let mut v = vec![name.to_string()];
    v.extend(r.iter().map(|n| n.to_string()));
    v
}

/// The stress variants requested on the command line.
fn stress_variants(args: &Args) -> Vec<(String, Box<dyn Engine>)> {
    let mut v: Vec<(String, Box<dyn Engine>)> = Vec::new();
    // `--determinism`: at least two slice sizes through heap images, their images compared.
    let mut slices: Vec<u64> = args.suspend_every.into_iter().collect();
    if args.determinism {
        for n in [97, 1009] {
            if !slices.contains(&n) && (n == 1009 || args.suspend_every.is_none()) {
                slices.push(n);
            }
        }
    }
    for n in slices {
        let e = Djs {
            suspend_every: Some(n),
            gc_stress: false,
            image: args.determinism,
            bytecode_builtins: args.bytecode_builtins,
            dump_every_idle: false,
            plain_from: 0,
        };
        v.push((e.label(), Box::new(e)));
    }
    if args.gc_stress {
        let e = Djs {
            suspend_every: None,
            gc_stress: true,
            image: false,
            bytecode_builtins: args.bytecode_builtins,
            dump_every_idle: false,
            plain_from: 0,
        };
        v.push((e.label(), Box::new(e)));
    }
    if args.plain_dumps {
        let e = Djs {
            dump_every_idle: true,
            plain_from: args.plain_from,
            bytecode_builtins: args.bytecode_builtins,
            ..Djs::default()
        };
        v.push((e.label(), Box::new(e)));
    }
    v
}

/// What went wrong in a failing case, as a cluster key: the first line of the uncaught
/// error the engine reported, or the kind of failure.
fn cluster_key(o: &Outcome) -> String {
    match &o.done {
        Some(Done::Throw(msg)) => format!("throws {}", msg.lines().next().unwrap_or("")),
        Some(Done::Crash(msg)) => format!("crash: {}", msg.lines().next().unwrap_or("")),
        Some(Done::Timeout) => "timeout".into(),
        Some(Done::Return(_) | Done::Completed) => "completes with different output".into(),
        None => "skipped".into(),
    }
}

/// Failures grouped by `cluster_key`, largest first, with a few example cases.
fn clusters(cases: &[Case], outcomes: &[Outcome]) -> String {
    let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (c, o) in cases.iter().zip(outcomes) {
        if matches!(o.verdict, Verdict::Fail { .. }) {
            groups.entry(cluster_key(o)).or_default().push(&c.name);
        }
    }
    let mut groups: Vec<(String, Vec<&str>)> = groups.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    let rows: Vec<Vec<String>> = groups
        .iter()
        .map(|(k, names)| {
            let mut ex = names.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            if names.len() > 3 {
                ex.push_str(", ...");
            }
            vec![names.len().to_string(), k.clone(), ex]
        })
        .collect();
    table(&["cases", "cluster", "examples"], &rows)
}
