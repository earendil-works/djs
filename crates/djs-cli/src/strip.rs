//! `djs strip-types <file.ts>`: print the source with its types erased to white space,
//! like ts-blank-space (the parse-only oracle for the type skipper, TESTING.md).

use std::io::Write;

use djs_core::compiler::{CompileOptions, strip_types};
use djs_core::heap::Heap;

const USAGE: &str = "usage: djs strip-types <file.ts>";

pub fn main(args: &[String]) -> i32 {
    let mut path = None;
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            _ if path.is_none() && !a.starts_with('-') => path = Some(a.as_str()),
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return 2;
    };
    let src = match std::fs::read(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("djs: {path}: {e}");
            return 2;
        }
    };
    let mut heap = match Heap::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("djs: {e}");
            return 2;
        }
    };
    // A codemode body: top-level `await` and `return` parse.
    let params = ["tools", "console"];
    match strip_types(&mut heap, &src, &CompileOptions::async_body(path, &params)) {
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
