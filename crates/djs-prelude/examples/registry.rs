//! Maintains the builtin registry (D2371, D3644): `cargo run -p djs-prelude --example
//! registry -- update` drops the entries of tests/builtins/registry.tsv that no longer
//! resolve and appends this build's new builtins and intrinsics; `check` lists what is out
//! of date. Prefer scripts/registry-update.sh.

use djs_core::vm::VmOptions;
use djs_core::vm::registry::Registry;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/builtins/registry.tsv"
);
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let vm = djs_prelude::new_vm(VmOptions::default()).expect("the base VM");
    let text = std::fs::read_to_string(FILE).unwrap_or_default();
    let mut reg = match Registry::parse(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("registry: {FILE}: {e}");
            std::process::exit(1);
        }
    };
    match args.first().map(String::as_str) {
        Some("update") => {
            let u = reg.update(vm.heap());
            for (kind, path) in &u.removed {
                println!("removed {kind:?} {path}");
            }
            for (kind, path) in &u.added {
                println!("added {kind:?} {path}");
            }
            std::fs::write(FILE, reg.to_text()).expect("cannot write the registry");
            println!(
                "registry: {} entries, {} added, {} removed",
                reg.entries.len(),
                u.added.len(),
                u.removed.len()
            );
        }
        Some("check") => {
            let problems = reg.check(vm.heap());
            for p in &problems {
                println!("{p}");
            }
            if !problems.is_empty() {
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: registry update | check");
            std::process::exit(2);
        }
    }
}
