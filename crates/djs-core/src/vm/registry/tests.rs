use super::*;
use crate::heap::CellKind;
use crate::vm::{Vm, VmOptions};

/// A VM like the base image: the prelude compiled, then collected and laid out.
fn vm() -> Vm {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    vm.prepare_base_image().unwrap();
    vm
}

#[test]
fn paths_split() {
    assert_eq!(
        split_root("Array.prototype.map"),
        ("Array", ".prototype.map")
    );
    assert_eq!(split_root("%TypedArray%.from"), ("%TypedArray%", ".from"));
    assert_eq!(split_root("%intrinsic:A%"), ("%intrinsic:A%", ""));
    assert_eq!(
        split_segment("[Symbol.iterator].x"),
        Some(("[Symbol.iterator]", ".x"))
    );
    assert_eq!(split_segment("[\"a.b\"].c"), Some(("[\"a.b\"]", ".c")));
    assert_eq!(
        split_segment(".[[Prototype]].y"),
        Some((".[[Prototype]]", ".y"))
    );
}

#[test]
fn canonical_names_resolve_to_their_cells() {
    let vm = vm();
    let heap = vm.heap();
    let names = canonical_names(heap);
    assert!(names.len() > 500, "{}", names.len());
    let mut r = Resolver::new(heap);
    for (cell, path) in &names {
        assert_eq!(r.resolve(path).and_then(cell_of), Some(*cell), "{path}");
    }
    let paths: Vec<&str> = names.iter().map(|(_, p)| p.as_str()).collect();
    // An alias resolves to the object of the canonical path.
    assert_eq!(
        r.resolve("Array.prototype[Symbol.iterator]"),
        r.resolve("Array.prototype.values")
    );
    for p in [
        "globalThis",
        "Array",
        "Array.prototype",
        "Array.prototype.map",
        "Array.prototype.values",
        "String.prototype[Symbol.iterator]",
        "get Map.prototype.size",
        "Symbol.iterator",
        "%ArrayIteratorPrototype%",
        "%TypedArray%",
    ] {
        assert!(paths.contains(&p), "{p} missing");
    }
}

#[test]
fn update_drops_the_gone_and_appends_the_new() {
    let vm = vm();
    let heap = vm.heap();
    let mut reg = Registry::default();
    let u = reg.update(heap);
    assert!(u.removed.is_empty());
    assert_eq!(u.added.len(), reg.entries.len());
    assert!(reg.check(heap).is_empty(), "{:?}", reg.check(heap));
    // A second update changes nothing.
    let before = reg.clone();
    assert_eq!(reg.update(heap), Update::default());
    assert_eq!(reg, before);
    // An entry that no longer resolves is dropped, and the ones after it move up.
    let map = reg.id_of(EntryKind::Value, "Array.prototype.map").unwrap();
    reg.entries.insert(
        0,
        Entry {
            id: 0,
            kind: EntryKind::Value,
            path: "Array.prototype.noSuchMethod".into(),
        },
    );
    for (i, e) in reg.entries.iter_mut().enumerate() {
        e.id = i as u32;
    }
    let problems = reg.check(heap);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let u = reg.update(heap);
    assert_eq!(
        u.removed,
        [(EntryKind::Value, "Array.prototype.noSuchMethod".into())]
    );
    assert!(u.added.is_empty());
    assert_eq!(reg, before);
    assert_eq!(
        reg.id_of(EntryKind::Value, "Array.prototype.map"),
        Some(map)
    );
    // The text round-trips.
    let text = reg.to_text();
    assert_eq!(Registry::parse(&text).unwrap(), reg);
}

#[test]
fn ids_name_cells_and_intrinsics() {
    let vm = vm();
    let heap = vm.heap();
    let mut reg = Registry::default();
    reg.update(heap);
    let map = reg.id_of(EntryKind::Value, "Array.prototype.map").unwrap();
    let cells = reg.name_cells(heap);
    let cell = cells
        .iter()
        .find(|(_, id)| **id == map)
        .map(|(c, _)| *c)
        .unwrap();
    let mut r = Resolver::new(heap);
    assert_eq!(
        r.resolve("Array.prototype.map").and_then(cell_of),
        Some(cell)
    );
    // An alias gets no entry of its own: the object keeps its id.
    assert!(reg.id_of(EntryKind::Value, "globalThis.Array").is_none());
    let ids = reg.intrinsic_ids();
    assert_eq!(ids.len(), crate::bytecode::intrinsics::INTRINSICS.len());
    assert!(ids.iter().all(Option::is_some));
}

#[test]
fn parse_rejects_gaps_and_duplicates() {
    assert!(Registry::parse("0\tvalue\tArray\n2\tvalue\tObject\n").is_err());
    assert!(Registry::parse("0\tvalue\tArray\n1\tvalue\tArray\n").is_err());
    assert!(Registry::parse("0\tvalue\tArray\tremoved: x\n").is_err());
    assert!(Registry::parse("0\tthing\tArray\n").is_err());
}

/// Every object and symbol of the base realm has a name, except the realm's own state
/// (the global lexical record, the registries), which no builtin exposes and which is
/// per VM rather than a builtin.
#[test]
fn unnamed_base_cells_are_realm_state() {
    let vm = vm();
    let heap = vm.heap();
    let named: BTreeSet<u32> = canonical_names(heap).into_iter().map(|(c, _)| c).collect();
    for c in heap.cells() {
        if !matches!(c.kind, CellKind::Object | CellKind::Symbol) || named.contains(&c.offset) {
            continue;
        }
        let from: Vec<CellKind> = heap
            .cells()
            .filter(|r| heap.cell_refs(r.offset).contains(&c.offset))
            .map(|r| r.kind)
            .collect();
        assert_eq!(from, [CellKind::Realm], "unnamed cell {}", c.offset);
    }
}
