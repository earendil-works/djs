//! `djs disasm [--body] <file.js>`: compile a file and print its bytecode.

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{CompileOptions, compile};
use djs_core::heap::Heap;

const USAGE: &str = "usage: djs disasm [--body] <file.js>\n\
  --body   compile as a codemode async body with parameters (tools, console), the CLI's default bindings";

pub fn main(args: &[String]) -> i32 {
    let mut body = false;
    let mut path = None;
    for a in args {
        match a.as_str() {
            "--body" => body = true,
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
            return 1;
        }
    };
    let params = crate::run::BODY_PARAMS;
    let opts = if body {
        CompileOptions::async_body(path, params)
    } else {
        CompileOptions::script(path)
    };
    let mut heap = match Heap::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("djs: {e}");
            return 1;
        }
    };
    match compile(&mut heap, &src, &opts) {
        Ok(t) => {
            print!("{}", disasm_template(&heap, t));
            0
        }
        Err(e) => {
            eprintln!("{}:{}:{}: {}", path, e.line, e.col, e.message);
            1
        }
    }
}
