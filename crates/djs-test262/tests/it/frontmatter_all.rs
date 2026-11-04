//! Parses the frontmatter of every test in the test262 checkout. Ignored by default
//! because it needs the checkout (scripts/fetch-test262.sh):
//!
//!     cargo test -p djs-test262 --test it frontmatter_all -- --ignored
//!
//! With `DJS_DUMP_META=<file>` it also writes one tab-separated line per test
//! (path, description, includes, flags, features, negative), for comparing against
//! a real YAML parser.

use std::fmt::Write;

use djs_test262::config::Config;
use djs_test262::suite::{self, Suite};

#[test]
#[ignore]
fn every_frontmatter_parses() {
    let root = suite::locate_test262().unwrap();
    let suite = Suite::open(&root).unwrap();
    let config = Config::parse("").unwrap();
    let paths = suite.discover(&[]).unwrap();
    assert!(paths.len() > 50_000, "only {} tests found", paths.len());
    let tests = suite.load(&config, paths, 8);
    let errors: Vec<String> = tests
        .iter()
        .filter_map(|t| t.error.as_ref().map(|e| format!("{}: {e}", t.path)))
        .collect();
    assert!(
        errors.is_empty(),
        "{} errors:\n{}",
        errors.len(),
        errors.join("\n")
    );

    if let Some(out) = std::env::var_os("DJS_DUMP_META") {
        let mut s = String::new();
        for t in &tests {
            let m = &t.meta;
            let neg = m
                .negative
                .as_ref()
                .map(|n| format!("{}:{}", n.phase, n.error_type))
                .unwrap_or_default();
            writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}",
                t.path,
                m.description.replace(['\t', '\n'], " "),
                m.includes.join(","),
                m.flags.join(","),
                m.features.join(","),
                neg
            )
            .unwrap();
        }
        std::fs::write(out, s).unwrap();
    }
}
