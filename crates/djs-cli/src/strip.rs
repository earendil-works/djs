//! `djs strip-types <file.ts>`: print the source with its types erased to white space,
//! like ts-blank-space (the parse-only oracle for the type skipper, TESTING.md).
//!
//! `djs strip-types --compare <expected-dir> <corpus-dir>`: for every file under
//! `expected-dir` (ts-blank-space's output, scripts/strip-types/oracle.sh), strip the
//! file of the same path under `corpus-dir` and compare. Prints one line per failure
//! (`differ <path> line <n>` or `error <path>: <message>`) and a summary; exits 1 on a
//! difference.

use std::io::Write;
use std::path::{Path, PathBuf};

use djs_core::compiler::{CompileError, CompileOptions, strip_types};
use djs_core::heap::Heap;

const USAGE: &str = "usage: djs strip-types <file.ts>\n\
  djs strip-types --compare <expected-dir> <corpus-dir>";

pub fn main(args: &[String]) -> i32 {
    match args {
        [flag, expected, corpus] if flag == "--compare" => {
            compare(Path::new(expected), Path::new(corpus))
        }
        [h] if h == "-h" || h == "--help" => {
            println!("{USAGE}");
            0
        }
        [path] if !path.starts_with('-') => strip_file(path),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

fn strip(path: &str, src: &[u8]) -> Result<Vec<u8>, CompileError> {
    let mut heap = Heap::new().expect("heap");
    // A codemode body: top-level `await` and `return` parse.
    strip_types(
        &mut heap,
        src,
        &CompileOptions::async_body(path, crate::run::BODY_PARAMS),
    )
}

fn strip_file(path: &str) -> i32 {
    let src = match std::fs::read(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("djs: {path}: {e}");
            return 2;
        }
    };
    match strip(path, &src) {
        Ok(out) => {
            let _ = std::io::stdout().lock().write_all(&out);
            0
        }
        Err(e) => {
            eprintln!("{}:{}:{}: SyntaxError: {}", path, e.line, e.col, e.message);
            1
        }
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn compare(expected: &Path, corpus: &Path) -> i32 {
    let mut files = Vec::new();
    walk(expected, &mut files);
    files.sort();
    let (mut pass, mut differ, mut errors) = (0, 0, 0);
    let mut out = std::io::stdout().lock();
    for exp in &files {
        let rel = exp.strip_prefix(expected).expect("under expected");
        let (Ok(want), Ok(src)) = (std::fs::read(exp), std::fs::read(corpus.join(rel))) else {
            let _ = writeln!(out, "error {}: unreadable", rel.display());
            errors += 1;
            continue;
        };
        match strip(&rel.to_string_lossy(), &src) {
            Ok(got) if got == want => pass += 1,
            Ok(got) => {
                differ += 1;
                let line = got
                    .split(|&b| b == b'\n')
                    .zip(want.split(|&b| b == b'\n'))
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                let _ = writeln!(out, "differ {} line {}", rel.display(), line + 1);
            }
            Err(e) => {
                errors += 1;
                let _ = writeln!(
                    out,
                    "error {}:{}:{}: {}",
                    rel.display(),
                    e.line,
                    e.col,
                    e.message
                );
            }
        }
    }
    let total = files.len();
    let _ = writeln!(
        out,
        "strip-types: {pass}/{total} match ts-blank-space, {differ} differ, {errors} are syntax errors for djs"
    );
    i32::from(differ > 0)
}
