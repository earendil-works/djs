mod disasm;
mod run;
mod strip;

const USAGE: &str = "usage: djs <command> ...\n\
  djs run [--body] [--ts] [--gc-stress] [--suspend-every N] <file.js>\n\
  djs disasm [--body] <file.js>\n\
  djs strip-types <file.ts>";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("disasm") => disasm::main(&args[1..]),
        Some("run") => run::main(&args[1..]),
        Some("strip-types") => strip::main(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}
