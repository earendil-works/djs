use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use djs_bench::runner::{Config, Measurement, Runner};
use djs_bench::{engine, run_all, scores};

const USAGE: &str = "\
usage: djs-bench [options] [filter...]

Runs the benchmarks (lex, num, regexp, json, gc, engine, size); filters select results
whose name contains one of the given substrings. Use scripts/bench.sh, which builds
with --release and fetches the inputs.

options:
  --quick             every benchmark once with small inputs (smoke test)
  --record            append the results to tests/scores/bench.tsv
  --compare[=REV]     compare with the last recorded run on this machine (or with the
                      run of commit REV) and exit 1 on regressions beyond the threshold
  --threshold PCT     noise threshold for --compare in percent (default 10; sizes 0.5)
  --budget SECS       sampling time per benchmark (default 1)
  --tsv FILE          the scores file (default tests/scores/bench.tsv)
";

fn main() -> ExitCode {
    let mut config = Config::default();
    let mut record = false;
    let mut compare: Option<Option<String>> = None;
    let mut threshold = 10.0;
    let mut tsv = engine::workspace_root().join("tests/scores/bench.tsv");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        let r: Result<(), String> = (|| {
            match a.as_str() {
                "-h" | "--help" => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                "--quick" => config.quick = true,
                "--record" => record = true,
                "--compare" => compare = Some(None),
                _ if a.starts_with("--compare=") => {
                    compare = Some(Some(a["--compare=".len()..].to_string()))
                }
                "--threshold" => {
                    threshold = value(&a)?
                        .parse()
                        .map_err(|_| "bad --threshold".to_string())?
                }
                "--budget" => {
                    let secs: f64 = value(&a)?.parse().map_err(|_| "bad --budget".to_string())?;
                    config.budget = Duration::from_secs_f64(secs);
                }
                "--tsv" => tsv = PathBuf::from(value(&a)?),
                _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
                _ => config.filters.push(a.clone()),
            }
            Ok(())
        })();
        if let Err(e) = r {
            eprintln!("djs-bench: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }
    if cfg!(debug_assertions) && !config.quick {
        eprintln!(
            "djs-bench: warning: debug build; use `cargo run --release -p djs-bench` (scripts/bench.sh)"
        );
    }
    if record && config.quick {
        eprintln!("djs-bench: --record does not make sense with --quick");
        return ExitCode::from(2);
    }

    let machine = scores::machine();
    let commit = scores::commit();
    println!(
        "djs-bench · {commit} · {machine}{}\n",
        if config.quick { " · quick" } else { "" }
    );
    let mut r = Runner::new(config);
    r.on_result = Some(Box::new(|m: &Measurement| {
        println!(
            "  {:52} {:>12} {:7} (best {}, n={})",
            m.name,
            scores::format_value(m.value),
            m.unit.as_str(),
            scores::format_value(m.best),
            m.samples
        );
    }));
    run_all(&mut r);
    for (name, why) in &r.skipped {
        println!("  {name:52} skipped: {why}");
    }

    let mut code = ExitCode::SUCCESS;
    if let Some(rev) = &compare {
        let rows = match scores::load(&tsv) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("djs-bench: {e}");
                return ExitCode::from(2);
            }
        };
        let base = scores::baseline(&rows, &machine, rev.as_deref());
        if base.is_empty() {
            eprintln!(
                "djs-bench: no recorded run to compare with in {}",
                tsv.display()
            );
            return ExitCode::from(2);
        }
        if base[0].machine != machine {
            println!(
                "\nnote: no run recorded on {machine}; comparing with {}",
                base[0].machine
            );
        }
        println!(
            "\ncompared with {} ({}), noise threshold {threshold}%:\n",
            base[0].commit, base[0].date
        );
        let (table, worse) = scores::compare(&base, &r.results, threshold);
        print!("{table}");
        if worse > 0 {
            println!("\n{worse} result(s) worse than the threshold");
            code = ExitCode::from(1);
        }
    }
    if record {
        if commit.ends_with("-dirty") {
            eprintln!("djs-bench: warning: recording a run of a dirty worktree ({commit})");
        }
        if let Err(e) = scores::append(&tsv, &scores::now_utc(), &commit, &machine, &r.results) {
            eprintln!("djs-bench: {e}");
            return ExitCode::from(2);
        }
        println!(
            "\nrecorded {} results in {}",
            r.results.len(),
            tsv.display()
        );
    }
    code
}
