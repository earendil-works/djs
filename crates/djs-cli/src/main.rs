mod disasm;
mod dumps;
mod heap;
mod json;
mod repl;
mod run;
mod snapshots;
mod strip;
mod tools;

const USAGE: &str = "usage: djs <command> ...\n\
  djs run [--body] [--ts] [--gc-stress] [--suspend-every N]\n\
          [--tools answers.json] <file.js>\n\
  djs resume [--tools answers.json] <file.snapshot | dump.js>\n\
  djs describe <file.snapshot>\n\
  djs disasm [--body] <file.js>\n\
  djs repl [--ts] [--gc-stress] [--no-history]\n\
  djs strip-types <file.ts>";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("disasm") => disasm::main(&args[1..]),
        Some("run") => run::main(&args[1..]),
        Some("resume") => run::resume_main(&args[1..]),
        Some("describe") => snapshots::describe_main(&args[1..]),
        Some("repl") => repl::main(&args[1..]),
        Some("strip-types") => strip::main(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}
