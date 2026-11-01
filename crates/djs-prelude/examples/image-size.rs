//! Size attribution of the base image (0043): bytes per cell kind, and per role for the
//! kinds that serve several (bytecode, line and handler tables, constant pools, script
//! sources, ...). `cargo run -p djs-prelude --example image-size`.

use std::collections::{BTreeMap, HashMap};

use djs_core::heap::cells::{FuncTemplate, Object, Script};
use djs_core::heap::{CellKind, Heap, ObjectClass, Ref};
use djs_core::vm::{Vm, VmOptions};

fn main() {
    let vm = djs_prelude::new_vm(VmOptions::default()).expect("base");
    let heap: &Heap = vm.heap();
    let a = heap.arena();
    // Roles of cells referenced from templates and scripts.
    let mut role: HashMap<u32, &'static str> = HashMap::new();
    for c in heap.cells() {
        match c.kind {
            CellKind::FuncTemplate => {
                for (f, name) in [
                    (FuncTemplate::CODE, "Bytes: bytecode"),
                    (FuncTemplate::HANDLERS, "Bytes: handler tables"),
                    (FuncTemplate::LINES, "Bytes: line tables"),
                    (FuncTemplate::CONSTS, "ValueArray: constant pools"),
                    (FuncTemplate::CHILDREN, "RefArray: child templates"),
                    (FuncTemplate::CAPTURE_DESC, "U32Array: capture descriptors"),
                    (FuncTemplate::NAMES, "U32Array: local names"),
                ] {
                    let r: Ref = a.get(c.offset, f);
                    if let Some(r) = r.get() {
                        role.insert(r, name);
                    }
                }
            }
            CellKind::Script => {
                let r: Ref = a.get(c.offset, Script::SOURCE);
                if let Some(r) = r.get() {
                    role.insert(r, "Str: script sources");
                }
            }
            CellKind::Object => {
                let props: Ref = a.get(c.offset, Object::PROPS);
                if let Some(r) = props.get() {
                    role.insert(r, "PropTable: object properties");
                }
            }
            _ => {}
        }
    }
    let mut by: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut total = 0u64;
    for c in heap.cells() {
        let key = match role.get(&c.offset) {
            Some(r) => r.to_string(),
            None if c.kind == CellKind::Object => {
                let class = ObjectClass::from_u32(a.get(c.offset, Object::CLASS));
                match class {
                    Some(ObjectClass::Function) => "Object: functions".into(),
                    Some(c) => format!("Object: {c:?}"),
                    None => "Object: ?".into(),
                }
            }
            None => format!("{:?}", c.kind),
        };
        let e = by.entry(key).or_default();
        e.0 += c.size as u64;
        e.1 += 1;
        total += c.size as u64;
    }
    let mut rows: Vec<_> = by.into_iter().collect();
    rows.sort_by(|x, y| y.1.0.cmp(&x.1.0));
    println!(
        "base image: {} bytes in the arena, {} in cells",
        vm.bytes().len(),
        total
    );
    for (k, (size, n)) in rows {
        println!(
            "{size:>9} {:5.1}% {n:>6}  {k}",
            100.0 * size as f64 / total as f64
        );
    }
    let _ = Vm::new; // (the image is djs_prelude::BASE)
    if std::env::args().any(|a| a == "--detail") {
        detail(heap);
    }
}

/// Detail: property table fill, value arrays, u32 arrays, strings, byte statistics.
#[allow(dead_code)]
pub fn detail(heap: &Heap) {
    use djs_core::heap::cells::{PropTable, U32Array, ValueArray};
    let a = heap.arena();
    let (mut cap_total, mut used_total, mut hashed) = (0u64, 0u64, 0);
    let mut by_count: BTreeMap<u32, u32> = BTreeMap::new();
    let mut big = Vec::new();
    for c in heap.cells() {
        match c.kind {
            CellKind::PropTable => {
                let count: u32 = a.get(c.offset, PropTable::COUNT);
                let cap = (c.size - 24) / 24;
                cap_total += cap as u64;
                used_total += count as u64;
                *by_count.entry(count).or_default() += 1;
                let idx: Ref = a.get(c.offset, PropTable::INDEX);
                if idx.get().is_some() {
                    hashed += 1;
                }
            }
            CellKind::ValueArray => {
                let len: u32 = a.get(c.offset, ValueArray::LEN);
                if c.size > 1000 {
                    big.push(format!("ValueArray size {} len {}", c.size, len));
                }
            }
            CellKind::U32Array => {
                let len: u32 = a.get(c.offset, U32Array::LEN);
                if c.size > 1000 {
                    big.push(format!("U32Array size {} len {}", c.size, len));
                }
            }
            _ => {}
        }
    }
    println!("\nprop tables: {cap_total} entry slots, {used_total} used, {hashed} hashed");
    println!("tables by entry count: {by_count:?}");
    for b in big {
        println!("{b}");
    }
    let bytes = heap.bytes();
    let zeros = bytes.iter().filter(|&&b| b == 0).count();
    let zero_words = bytes
        .chunks(8)
        .filter(|w| w.iter().all(|&b| b == 0))
        .count();
    println!(
        "\nbytes {}: zero bytes {} ({:.1}%), all-zero 8-byte words {}",
        bytes.len(),
        zeros,
        100.0 * zeros as f64 / bytes.len() as f64,
        zero_words
    );
}
