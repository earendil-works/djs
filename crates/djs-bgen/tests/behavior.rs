//! Runs the generated builtins (builtins/gen/prelude.js) in qjs against qjs's native
//! Array.prototype methods (tests/behavior.js) and expects identical results, error
//! messages and proxy trap orders. Skipped (with a note) when qjs is not available.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Hang protection only (D1200): a normal run takes well under a second.
const TIMEOUT: Duration = Duration::from_secs(60);

/// Runs `qjs --script path`, killing it after `limit`. `None` on a timeout. The pipes
/// are drained on threads so a large output cannot block the child.
fn run_with_timeout(qjs: &Path, path: &Path, limit: Duration) -> Option<Output> {
    let mut child = Command::new(qjs)
        .arg("--script")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let drain = |mut r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Some(Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    })
}

fn qjs() -> PathBuf {
    if let Some(p) = std::env::var_os("DJS_QJS") {
        return PathBuf::from(p);
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Development/dquickts/qjs")
}

#[test]
fn generated_builtins_match_qjs() {
    let qjs = qjs();
    if !qjs.is_file() {
        eprintln!("skipping: qjs not found at {} (set DJS_QJS)", qjs.display());
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prelude = std::fs::read_to_string(root.join("../../builtins/gen/prelude.js")).unwrap();
    // `%Name(` only appears as intrinsic calls: the prelude has no `%` in strings.
    let prelude = prelude.replace('%', "I.");
    let script = std::fs::read_to_string(root.join("tests/behavior.js"))
        .unwrap()
        .replace("PRELUDE;", &prelude);
    let path = std::env::temp_dir().join(format!("djs-bgen-behavior-{}.js", std::process::id()));
    std::fs::write(&path, script).unwrap();
    let out = run_with_timeout(&qjs, &path, TIMEOUT);
    let _ = std::fs::remove_file(&path);
    let out = match out {
        Some(out) => out,
        None => panic!(
            "timeout: no result within {}s (a builtin or a comparison loops in qjs)",
            TIMEOUT.as_secs()
        ),
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "qjs failed:\n{stdout}\n{stderr}");
    let last = stdout.lines().last().unwrap_or("");
    assert!(
        last.starts_with("ok ") && !last.contains("mismatch"),
        "{stdout}{stderr}"
    );
    let n: usize = last[3..].trim().parse().unwrap();
    assert!(n > 2000, "only {n} comparisons ran");
}
