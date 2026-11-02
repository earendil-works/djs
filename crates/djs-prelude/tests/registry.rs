//! The builtin registry is current for this build (D2371): every builtin and intrinsic has
//! a live entry, every live entry resolves, and the base offset table agrees. The
//! append-only property (no renumbering, no dropped lines) is checked against main by
//! scripts/registry.py in the gate.

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::vm::VmOptions;
use djs_core::vm::registry::{EntryKind, Registry};
use djs_prelude::registry;

#[test]
fn registry_is_current() {
    let reg = Registry::parse(registry::TEXT)
        .unwrap_or_else(|e| panic!("tests/builtins/registry.tsv: {e}"));
    let vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let problems = reg.check(vm.heap());
    assert!(
        problems.is_empty(),
        "the builtin registry is out of date; run scripts/registry-update.sh \
         (and commit tests/builtins/registry.tsv):\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn base_offsets_agree_with_paths() {
    let reg = registry::registry();
    let vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let cells = reg.name_cells(vm.heap());
    let mut values = 0;
    for e in &reg.entries {
        let want = if e.kind == EntryKind::Value && e.removed.is_none() {
            cells.iter().find(|(_, id)| **id == e.id).map(|(c, _)| *c)
        } else {
            None
        };
        // An alias (a second path to a named cell) has no offset of its own.
        if let Some(c) = want {
            assert_eq!(registry::base_offset(e.id), Some(c), "{}", e.path);
            assert_eq!(registry::id_of_base_offset(c), Some(e.id), "{}", e.path);
            values += 1;
        }
        assert_eq!(registry::path(e.id).as_deref(), Some(e.path.as_str()));
    }
    assert!(values > 900, "{values}");
    for (i, info) in INTRINSICS.iter().enumerate() {
        let id = registry::id_of_intrinsic(i as u16).unwrap();
        assert_eq!(registry::path(id).as_deref(), Some(info.name));
    }
    let map = registry::id_of_path(EntryKind::Value, "Array.prototype.map").unwrap();
    assert!(registry::base_offset(map).is_some());
}
