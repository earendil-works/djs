//! The engine track: JS microbenchmarks (`tests/bench/*.js`) run in the codemode model
//! through djs-difftest's `Engine` trait, on qjs (with the shared prelude) and on djs
//! in-process (D1191), with the djs/qjs time ratio per script. Each script starts with `// bench: expect <result>` (the returned value
//! as the model sees it, or `completed`), which every run is checked against.
//!
//! A sample is one whole run (process start included for qjs). `empty.js` measures that
//! overhead; the other scripts report their time minus its median.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use djs_difftest::djs::Djs;
use djs_difftest::engine::{Done, Engine, Job};
use djs_difftest::qjs::Qjs;

use crate::runner::{Runner, Unit};

pub struct Script {
    pub name: String,
    pub file: PathBuf,
    pub source: String,
    pub expect: Done,
}

pub fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

/// The scripts in `dir`, sorted, with `empty.js` first.
pub fn load_scripts(dir: &Path) -> Result<Vec<Script>, String> {
    let mut scripts = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries.filter_map(|e| e.ok()) {
        let file = e.path();
        if file.extension().is_none_or(|x| x != "js") {
            continue;
        }
        let source =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let expect = source
            .lines()
            .find_map(|l| l.trim().strip_prefix("// bench: expect "))
            .map(|v| {
                if v == "completed" {
                    Done::Completed
                } else {
                    Done::Return(v.to_string())
                }
            })
            .ok_or_else(|| format!("{}: missing `// bench: expect <result>`", file.display()))?;
        scripts.push(Script {
            name,
            file,
            source,
            expect,
        });
    }
    scripts.sort_by_key(|s| (s.name != "empty", s.name.clone()));
    Ok(scripts)
}

/// Runs a script once on `engine`; returns the wall time, or why the run is invalid.
pub fn run_once(
    engine: &dyn Engine,
    script: &Script,
    prelude_file: &Path,
    prelude: &str,
) -> Result<Duration, String> {
    let job = Job {
        name: &script.name,
        file: &script.file,
        source: &script.source,
        ts: false,
        prelude_file,
        prelude,
        timeout: Duration::from_secs(60),
        fuel: None,
        memory_limit: None,
        now: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0),
    };
    let start = Instant::now();
    let result = engine.run(&job);
    let elapsed = start.elapsed();
    if result.done != script.expect {
        return Err(format!(
            "{}: expected {:?}, got {:?}",
            script.name, script.expect, result.done
        ));
    }
    Ok(elapsed)
}

pub fn engine(r: &mut Runner) {
    let root = workspace_root();
    let mut engines: Vec<(&str, Box<dyn Engine>)> = Vec::new();
    let qjs = Qjs::default_binary();
    if qjs.is_file() {
        engines.push(("qjs", Box::new(Qjs { binary: qjs })));
    } else {
        r.skip("engine/qjs", format!("qjs not found at {}", qjs.display()));
    }
    // djs in-process (the difftest djs engine, D1160). Scripts it cannot run yet are
    // skipped with the reason.
    engines.push(("djs", Box::new(Djs::default())));
    // The same with every builtin as bytecode (0063): what the @native builtins buy.
    engines.push((
        "djs-bytecode",
        Box::new(Djs {
            bytecode_builtins: true,
            ..Djs::default()
        }),
    ));
    let scripts = match load_scripts(&root.join("tests/bench")) {
        Ok(s) => s,
        Err(e) => return r.skip("engine", e),
    };
    let prelude_file = root.join("tests/corpus/prelude.js");
    let prelude = std::fs::read_to_string(&prelude_file).unwrap_or_default();
    for (engine_name, engine) in &engines {
        let mut startup = Duration::ZERO;
        for s in &scripts {
            let name = format!("engine/{engine_name}/{}", s.name);
            if s.name != "empty" && r.config.quick {
                // Even one run each takes seconds; quick mode only checks the setup.
                continue;
            }
            let overhead = startup;
            r.time_with(&name, || {
                run_once(engine.as_ref(), s, &prelude_file, &prelude)
                    .map(|d| d.saturating_sub(overhead))
            });
            if s.name == "empty"
                && let Some(m) = r.results.last().filter(|m| m.name == name)
            {
                startup = Duration::from_secs_f64(m.value / 1e3);
            }
        }
    }
    // djs time / qjs time per script that ran on both (lower is better; 1.0 is parity),
    // and the bytecode backend's time / the native backend's (0063).
    for s in scripts.iter().filter(|s| s.name != "empty") {
        let value = |engine: &str| {
            let name = format!("engine/{engine}/{}", s.name);
            r.results.iter().find(|m| m.name == name).map(|m| m.value)
        };
        let (d, q, b) = (value("djs"), value("qjs"), value("djs-bytecode"));
        if let (Some(d), Some(q)) = (d, q)
            && q > 0.0
        {
            r.value(&format!("engine/djs÷qjs/{}", s.name), Unit::Ratio, d / q);
        }
        if let (Some(b), Some(d)) = (b, d)
            && d > 0.0
        {
            r.value(
                &format!("engine/bytecode÷native/{}", s.name),
                Unit::Ratio,
                b / d,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_have_expectations() {
        let scripts = load_scripts(&workspace_root().join("tests/bench")).unwrap();
        assert!(scripts.len() >= 8);
        assert_eq!(scripts[0].name, "empty");
        assert_eq!(scripts[0].expect, Done::Completed);
    }
}
