mod container;
mod disasm;
mod heap;
mod repl;
mod run;
mod strip;
mod tools;

const USAGE: &str = "usage: djs <command> ...\n\
  djs run [--body] [--ts] [--gc-stress] [--suspend-every N] [--backend B] <file.js>\n\
  djs disasm [--body] <file.js>\n\
  djs heap [--json] [--top N] [--reference FILE] [--diff FILE|base|none] <dump>\n\
  djs repl [--ts] [--gc-stress] [--no-history]\n\
  djs strip-types <file.ts>";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("disasm") => disasm::main(&args[1..]),
        Some("heap") => heap::main(&args[1..]),
        Some("run") => run::main(&args[1..]),
        Some("repl") => repl::main(&args[1..]),
        Some("strip-types") => strip::main(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}
