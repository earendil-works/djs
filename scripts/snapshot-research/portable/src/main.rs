//! Prototype of portable snapshots (issue 0141, docs/portable-snapshots.md): a waiting
//! codemode session exported by one djs build as an engine-independent graph and
//! imported by another build from a different commit.
//!
//! portable run SCRIPT                 run to the end, print the output
//! portable export SCRIPT K OUT.json   deliver K tool results, export the waiting VM
//! portable import IN.json             import, deliver the rest, print the output
//! portable roundtrip SCRIPT...        export and import at every idle point, same build
//! portable harvest [...] CASE...       export/import at the idle points of harvested cases
//! portable import-all DIR             import snapshots saved by `harvest --save DIR`

mod export;
mod heapx;
mod import;
mod roles;
mod session;
mod sites;

use session::Session;

fn report(s: &Session) -> String {
    let mut out = String::new();
    for o in &s.output {
        out += o;
        out.push('\n');
    }
    out += &format!("[{}]\n", s.finished.clone().unwrap_or("not finished".into()));
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("run") => {
            let src = std::fs::read_to_string(&args[1]).unwrap();
            let mut s = Session::start(&src);
            s.run_to_end();
            print!("{}", report(&s));
        }
        Some("export") => {
            let src = std::fs::read_to_string(&args[1]).unwrap();
            let k: u32 = args[2].parse().unwrap();
            let mut s = Session::start(&src);
            for _ in 0..k {
                s.deliver_one();
            }
            match export::export(&mut s) {
                Ok(j) => {
                    let text = serde_json::to_string_pretty(&j).unwrap();
                    std::fs::write(&args[3], &text).unwrap();
                    eprintln!(
                        "exported after {k} results: {} nodes, {} pending calls, {} bytes",
                        j["nodes"].as_array().unwrap().len(),
                        j["pending"].as_array().unwrap().len(),
                        text.len()
                    );
                }
                Err(e) => {
                    eprintln!("export refused: {e:?}");
                    std::process::exit(1);
                }
            }
        }
        Some("import") => {
            let text = std::fs::read_to_string(&args[1]).unwrap();
            let j: serde_json::Value = serde_json::from_str(&text).unwrap();
            match import::import(&j) {
                Ok(mut s) => {
                    s.run();
                    s.run_to_end();
                    print!("{}", report(&s));
                }
                Err(e) => {
                    eprintln!("import refused: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("roundtrip") => {
            for path in &args[1..] {
                let src = std::fs::read_to_string(path).unwrap();
                let mut whole = Session::start(&src);
                whole.run_to_end();
                let want = report(&whole);
                let mut k = 0;
                let mut ok = 0;
                let mut fails = Vec::new();
                loop {
                    let mut s = Session::start(&src);
                    for _ in 0..k {
                        s.deliver_one();
                    }
                    if s.finished.is_some() {
                        break;
                    }
                    let got = export::export(&mut s)
                        .map_err(|e| format!("export: {e:?}"))
                        .and_then(|j| {
                            let text = serde_json::to_string(&j).unwrap();
                            let j: serde_json::Value = serde_json::from_str(&text).unwrap();
                            import::import(&j).map_err(|e| format!("import: {e}"))
                        })
                        .map(|mut r| {
                            r.run();
                            r.run_to_end();
                            report(&r)
                        });
                    match got {
                        Ok(g) if g == want => ok += 1,
                        Ok(g) => fails.push(format!("idle {k}: output differs:\n{g}")),
                        Err(e) => fails.push(format!("idle {k}: {e}")),
                    }
                    k += 1;
                }
                println!("{path}: {ok}/{k} idle points round-trip");
                for f in fails {
                    println!("  {f}");
                }
            }
        }
        Some("harvest") => sites::main(&args[1..]),
        Some("import-all") => sites::import_all(&args[1]),
        _ => {
            eprintln!("usage: portable run|export|import|roundtrip|sites ...");
            std::process::exit(2);
        }
    }
}
