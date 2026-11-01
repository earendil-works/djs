mod disasm;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("disasm") => disasm::main(&args[1..]),
        _ => {
            eprintln!("usage: djs disasm [--body] <file.js>");
            2
        }
    };
    std::process::exit(code);
}
