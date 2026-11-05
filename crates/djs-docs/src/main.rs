//! `djs-docs`: the guide in docs/guide (0103, D2100).
//!
//! ```text
//! djs-docs check               generated regions and diagrams fresh, examples' output
//!                              unchanged, links and references valid; renders every page
//! djs-docs regen [--examples]  rewrite the generated regions and diagrams in place;
//!                              --examples also rewrites example output (`gen:run`,
//!                              `gen:dump`)
//! djs-docs build [--out DIR] [--wasm FILE]
//!                              the static site (default target/docs-site), with live
//!                              examples on djs-wasm (default: the `wasm` profile build)
//! ```
//!
//! The guide's pages are listed in docs/guide/SUMMARY.md (mdBook's format).

mod highlight;
mod paths;
mod regions;
mod render;
mod run;
mod search;
mod site;
mod status;
mod svg;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: djs-docs check | regen [--examples] | build [--out DIR] [--wasm FILE]";

pub fn repo() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    p.canonicalize().unwrap_or(p)
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The guide after regeneration (in memory).
pub struct Guide {
    pub dir: PathBuf,
    pub summary: Vec<site::Entry>,
    /// Page -> regenerated source.
    pub pages: BTreeMap<String, String>,
    pub cx: regions::Ctx,
    /// Pages and diagrams whose checked-in text differs from the regenerated one.
    pub stale: Vec<String>,
    /// Repository files outside the guide with generated regions (README.md), by path
    /// from the repository root, after regeneration, and those that changed.
    pub extra: BTreeMap<String, String>,
    pub stale_extra: Vec<String>,
    /// Diagrams in gen/ that nothing generates any more.
    pub orphans: Vec<String>,
}

/// Repository files outside docs/guide whose generated regions djs-docs maintains.
const EXTRA: &[&str] = &["README.md"];

fn load(examples: bool) -> Guide {
    let root = repo();
    let dir = root.join("docs/guide");
    let src = regions::Sources {
        ops_rs: read(&root.join("crates/djs-core/src/bytecode/ops.rs")),
        intrinsics_dts: read(&root.join("builtins/intrinsics.d.ts")),
    };
    let mut cx = regions::Ctx::new(&src, examples);
    let summary = site::summary(&read(&dir.join("SUMMARY.md")));
    let mut pages = BTreeMap::new();
    let mut stale = Vec::new();
    for e in &summary {
        let Some(p) = &e.path else { continue };
        let path = dir.join(p);
        let Ok(old) = std::fs::read_to_string(&path) else {
            cx.errors.push(format!("SUMMARY.md: {p} does not exist"));
            continue;
        };
        let new = regions::regen(p, &old, &mut cx);
        if new != old {
            stale.push(p.clone());
        }
        pages.insert(p.clone(), new);
    }
    // Files outside the guide that use generated regions (status numbers, D2101). They
    // are not rendered: their links are relative to the repository root.
    let mut extra = BTreeMap::new();
    let mut stale_extra = Vec::new();
    for p in EXTRA {
        let old = read(&root.join(p));
        let new = regions::regen(p, &old, &mut cx);
        if new != old {
            stale_extra.push(p.to_string());
        }
        extra.insert(p.to_string(), new);
    }
    // Hand-drawn diagrams keep their drawing; their stylesheet is the generated one's
    // (svg::restyle, D2200), so an Elanor update reaches them through regen too.
    if let Ok(rd) = std::fs::read_dir(dir.join("img")) {
        let mut names: Vec<String> = rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".svg"))
            .collect();
        names.sort();
        for n in names {
            let old = read(&dir.join("img").join(&n));
            match svg::restyle(n.trim_end_matches(".svg"), &old) {
                Ok(new) => {
                    cx.files.insert(format!("img/{n}"), new);
                }
                Err(e) => cx.errors.push(e),
            }
        }
    }
    for (f, svg) in &cx.files {
        if std::fs::read_to_string(dir.join(f)).ok().as_deref() != Some(svg.as_str()) {
            stale.push(f.clone());
        }
    }
    let mut orphans = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir.join("gen")) {
        for e in rd.flatten() {
            let f = format!("gen/{}", e.file_name().to_string_lossy());
            if !cx.files.contains_key(&f) {
                orphans.push(f);
            }
        }
    }
    orphans.sort();
    Guide {
        dir,
        summary,
        pages,
        extra,
        stale_extra,
        cx,
        stale,
        orphans,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let flag = |name: &str| args.iter().any(|a| a == name);
    let opt = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    match cmd.as_str() {
        "check" => {
            let mut g = load(false);
            // Render every page too: missing diagrams, broken links, the writing rules.
            let _ = site::render_all(&mut g);
            let mut bad = false;
            for s in &g.stale {
                eprintln!("djs-docs: stale: docs/guide/{s}");
                bad = true;
            }
            for s in &g.stale_extra {
                eprintln!("djs-docs: stale: {s}");
                bad = true;
            }
            for s in &g.orphans {
                eprintln!("djs-docs: no longer generated: docs/guide/{s}");
                bad = true;
            }
            for w in &g.cx.warnings {
                eprintln!("djs-docs: warning: {w}");
            }
            if bad {
                eprintln!("  run `cargo run -p djs-docs -- regen` (scripts/regen.sh does it too)");
            }
            for p in &g.cx.changed_examples {
                eprintln!("djs-docs: an example's output changed: docs/guide/{p}");
                bad = true;
            }
            if !g.cx.changed_examples.is_empty() {
                eprintln!(
                    "  if the new output is right, accept it with `cargo run -p djs-docs -- regen --examples`\n  \
                     and check that the page's text still says the right thing"
                );
            }
            for e in &g.cx.errors {
                eprintln!("djs-docs: {e}");
                bad = true;
            }
            if bad {
                return ExitCode::FAILURE;
            }
            println!(
                "djs-docs: {} pages, {} diagrams: ok",
                g.pages.len(),
                g.cx.files.len()
            );
        }
        "regen" => {
            let g = load(flag("--examples"));
            for p in &g.stale {
                let text = g
                    .pages
                    .get(p)
                    .or_else(|| g.cx.files.get(p))
                    .expect("generated");
                std::fs::create_dir_all(g.dir.join(p).parent().expect("parent")).expect("mkdir");
                std::fs::write(g.dir.join(p), text).expect("write");
                println!("djs-docs: rewrote docs/guide/{p}");
            }
            for p in &g.stale_extra {
                std::fs::write(repo().join(p), &g.extra[p]).expect("write");
                println!("djs-docs: rewrote {p}");
            }
            for w in &g.cx.warnings {
                eprintln!("djs-docs: warning: {w}");
            }
            for o in &g.orphans {
                std::fs::remove_file(g.dir.join(o)).expect("remove");
                println!("djs-docs: removed docs/guide/{o}");
            }
            for p in &g.cx.changed_examples {
                println!(
                    "djs-docs: kept the old example output in docs/guide/{p} (accept with --examples)"
                );
            }
            if !g.cx.errors.is_empty() {
                for e in &g.cx.errors {
                    eprintln!("djs-docs: {e}");
                }
                return ExitCode::FAILURE;
            }
        }
        "build" => {
            let out = opt("--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| repo().join("target/docs-site"));
            // The docs build of djs-wasm (live bytecode listings) if there is one, else the
            // shipping module (the Bytecode tab then keeps the build-time listings).
            let wasm = opt("--wasm").map(PathBuf::from).unwrap_or_else(|| {
                let docs =
                    repo().join("target/docs-wasm/wasm32-unknown-unknown/wasm/djs_wasm.wasm");
                if docs.exists() {
                    docs
                } else {
                    repo().join("target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")
                }
            });
            let mut g = load(false);
            if let Err(e) = site::build(&mut g, &out, &wasm) {
                eprintln!("djs-docs: {e}");
                return ExitCode::FAILURE;
            }
            for e in &g.cx.errors {
                eprintln!("djs-docs: {e}");
            }
            if !g.cx.errors.is_empty() {
                return ExitCode::FAILURE;
            }
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// Pages known to the guide (for link checks).
pub fn page_set(g: &Guide) -> BTreeSet<String> {
    g.pages.keys().cloned().collect()
}
