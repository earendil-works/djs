//! Maintains the builtin registry (D2371): `cargo run -p djs-prelude --example registry --
//! update [REASON]` appends this build's new builtins and intrinsics to
//! tests/builtins/registry.tsv and tombstones the entries that no longer resolve (with
//! REASON, default "removed"); `check` lists what is out of date. Prefer
//! scripts/registry-update.sh.

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
            let reason = args.get(1).map_or("removed", String::as_str);
            let u = reg.update(vm.heap(), reason);
            for id in &u.added {
                let e = &reg.entries[*id as usize];
                println!("added {} {:?} {}", e.id, e.kind, e.path);
            }
            for id in &u.removed {
                let e = &reg.entries[*id as usize];
                println!("tombstoned {} {:?} {}", e.id, e.kind, e.path);
            }
            std::fs::write(FILE, reg.to_text()).expect("cannot write the registry");
            println!(
                "registry: {} entries, {} added, {} tombstoned",
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
            eprintln!("usage: registry update [REASON] | check");
            std::process::exit(2);
        }
    }
}
