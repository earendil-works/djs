//! The builtin registry is current for this build (D2371, D3644): every builtin and
//! intrinsic has an entry, and every entry resolves.

use djs_core::vm::VmOptions;
use djs_core::vm::registry::Registry;

#[test]
fn registry_is_current() {
    let reg = Registry::parse(include_str!("../../../tests/builtins/registry.tsv"))
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
