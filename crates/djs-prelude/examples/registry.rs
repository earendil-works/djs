//! Maintains the builtin registry (D2371): `cargo run -p djs-prelude --example registry --
//! update [REASON]` appends this build's new builtins and intrinsics to
//! tests/builtins/registry.tsv and tombstones the entries that no longer resolve (with
//! REASON, default "removed"); `check` lists what is out of date. Prefer
//! scripts/registry-update.sh.
//!
//! In the slim build (`--no-default-features --features raw,registry`, D2411), `update`
//! writes the ids that build doesn't have to tests/builtins/registry-slim.txt and leaves
//! registry.tsv alone (the full build maintains it); `check` checks both.

use djs_core::vm::VmOptions;
use djs_core::vm::registry::Registry;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/builtins/registry.tsv"
);
const SLIM_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/builtins/registry-slim.txt"
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
        Some("update") if !djs_core::FULL => {
            let old = std::fs::read_to_string(SLIM_FILE).unwrap_or_default();
            let header: String = old
                .lines()
                .take_while(|l| l.starts_with('#'))
                .map(|l| format!("{l}\n"))
                .collect();
            let mut out = header;
            let absent = reg.unresolved(vm.heap());
            for id in &absent {
                out.push_str(&format!("{id}\t{}\n", reg.entries[*id as usize].path));
            }
            std::fs::write(SLIM_FILE, out).expect("cannot write the slim list");
            println!("registry (slim): {} entries absent", absent.len());
        }
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
            let problems = reg.check_build(vm.heap(), &djs_prelude::registry::absent());
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
