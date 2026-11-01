//! The engine track: JS microbenchmarks (`tests/bench/*.js`) run in the codemode model
//! through djs-difftest's `Engine` trait and prelude, on qjs now and on djs once it runs
//! JS (D1191). Each script starts with `// bench: expect <result>` (the returned value
//! as the model sees it, or `completed`), which every run is checked against.
//!
//! A sample is one whole run (process start included for qjs). `empty.js` measures that
//! overhead; the other scripts report their time minus its median.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use djs_difftest::engine::{Done, Engine, Job};
use djs_difftest::qjs::Qjs;

use crate::runner::Runner;

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
    // djs joins here once it runs JS (0014/0015), and the ratio djs/qjs is reported.
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
