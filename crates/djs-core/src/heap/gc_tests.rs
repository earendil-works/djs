//! GC and handle table tests.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::value::{Kind, Value};

// ------------------------------------------------------------------ deep compare

/// A canonical text dump of everything reachable from `roots`, by walking the graph from
/// the handles. Objects and symbols get ids in visit order, so two heaps with the same
/// graph give the same dump regardless of where cells are.
fn dump(h: &Heap, roots: &[Handle]) -> String {
    struct D<'a> {
        h: &'a Heap,
        ids: BTreeMap<u64, usize>,
        queue: Vec<Value>,
        out: String,
    }
    impl D<'_> {
        fn val(&mut self, v: Value) -> String {
            match v.kind() {
                Kind::Object | Kind::Symbol => {
                    let next = self.ids.len();
                    let id = *self.ids.entry(v.to_bits()).or_insert_with(|| {
                        self.queue.push(v);
                        next
                    });
                    let p = if v.kind() == Kind::Object { "o" } else { "s" };
                    format!("{p}{id}")
                }
                Kind::String => format!("{:?}", self.h.str_to_string(v.as_string().unwrap())),
                Kind::Number => format!("n{:?}", v.as_number().unwrap()),
                _ => format!("{v:?}"),
            }
        }
        fn key(&mut self, k: Atom) -> String {
            match self.h.atom_symbol(k) {
                Some(s) => format!("[{}]", self.val(Value::symbol(s))),
                None => self.h.atom_name(k),
            }
        }
    }
    let mut d = D {
        h,
        ids: BTreeMap::new(),
        queue: Vec::new(),
        out: String::new(),
    };
    for &r in roots {
        let v = d.val(h.get_handle(r));
        d.out += &format!("root {} = {v}\n", r.index());
    }
    let mut i = 0;
    while i < d.queue.len() {
        let v = d.queue[i];
        i += 1;
        let line = if let Some(s) = v.as_symbol() {
            let desc = d.val(h.symbol_description(s));
            format!("{} symbol {desc}\n", d.val(v))
        } else {
            let o = v.as_object().unwrap();
            let proto = match h.get_prototype_of(o) {
                Some(p) => d.val(Value::object(p)),
                None => String::from("null"),
            };
            let mut line = format!(
                "{} {:?} proto={proto} ext={}",
                d.val(v),
                h.object_class(o),
                h.is_extensible(o)
            );
            for k in h.own_keys(o) {
                let ks = d.key(k);
                let p = h.get_own_property(o, k).unwrap();
                let f = p.flags().bits();
                let ps = match p {
                    Property::Data { value, .. } => d.val(value),
                    Property::Accessor { getter, setter, .. } => {
                        format!("get {} set {}", d.val(getter), d.val(setter))
                    }
                };
                line += &format!(" {ks}:{f}={ps}");
            }
            line + "\n"
        };
        d.out += &line;
    }
    d.out
}

// ------------------------------------------------------------------ random graphs

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }
}

/// Handles whose value satisfies `pred`.
fn pick(h: &Heap, rng: &mut Rng, hs: &[Handle], pred: impl Fn(Value) -> bool) -> Option<Handle> {
    let c: Vec<Handle> = hs
        .iter()
        .copied()
        .filter(|&x| pred(h.get_handle(x)))
        .collect();
    if c.is_empty() {
        None
    } else {
        Some(c[rng.below(c.len())])
    }
}

fn is_obj(v: Value) -> bool {
    v.as_object().is_some()
}

fn random_key(h: &mut Heap, rng: &mut Rng, hs: &[Handle]) -> Atom {
    match rng.below(4) {
        0 => Atom::int(rng.below(40) as u32).unwrap(),
        1 => {
            if let Some(s) = pick(h, rng, hs, |v| v.as_symbol().is_some()) {
                h.symbol_atom(h.get_handle(s).as_symbol().unwrap())
            } else {
                atoms::NAME
            }
        }
        _ => h.atom_str(&format!("k{}", rng.below(30))).unwrap(),
    }
}

fn random_value(h: &mut Heap, rng: &mut Rng, hs: &[Handle]) -> Value {
    match rng.below(5) {
        0 => Value::int(rng.below(1000) as i32),
        1 => Value::number(rng.below(1000) as f64 / 7.0),
        2 => [Value::UNDEFINED, Value::NULL, Value::TRUE][rng.below(3)],
        _ => {
            if hs.is_empty() {
                Value::NULL
            } else {
                h.get_handle(hs[rng.below(hs.len())])
            }
        }
    }
}

/// Runs `ops` random mutations, holding everything through handles. With `gc_every`,
/// collects every that many ops. Returns the live handles.
fn build_random(h: &mut Heap, seed: u64, ops: usize, gc_every: Option<usize>) -> Vec<Handle> {
    let mut rng = Rng(seed | 1);
    let base = h.new_object(None).unwrap();
    let mut hs = alloc::vec![h.new_handle(Value::object(base)).unwrap()];
    for op in 0..ops {
        if let Some(n) = gc_every
            && op % n == n - 1
        {
            h.collect().unwrap();
        }
        match rng.below(14) {
            0 | 1 => {
                let proto = pick(h, &mut rng, &hs, is_obj)
                    .filter(|_| rng.chance(70))
                    .map(|p| h.get_handle(p).as_object().unwrap());
                let o = h.new_object(proto).unwrap();
                hs.push(h.new_handle(Value::object(o)).unwrap());
            }
            2 => {
                let a = h.new_array(None, rng.below(4) as u32).unwrap();
                hs.push(h.new_handle(Value::object(a)).unwrap());
            }
            3 => {
                let len = [1, 5, 40, 100][rng.below(4)];
                let t: String = (0..len)
                    .map(|i| {
                        if rng.chance(10) {
                            '€'
                        } else {
                            (b'a' + (i % 26) as u8) as char
                        }
                    })
                    .collect();
                let s = h.alloc_str(&t).unwrap();
                hs.push(h.new_handle(Value::string(s)).unwrap());
            }
            4 => {
                // Ropes, sometimes flattened.
                let a = pick(h, &mut rng, &hs, |v| v.as_string().is_some());
                let b = pick(h, &mut rng, &hs, |v| v.as_string().is_some());
                if let (Some(a), Some(b)) = (a, b) {
                    let sa = h.get_handle(a).as_string().unwrap();
                    let sb = h.get_handle(b).as_string().unwrap();
                    let r = h.concat(sa, sb).unwrap();
                    if rng.chance(30) {
                        h.flatten(r).unwrap();
                    }
                    hs.push(h.new_handle(Value::string(r)).unwrap());
                }
            }
            5..=7 => {
                if let Some(o) = pick(h, &mut rng, &hs, is_obj) {
                    let k = random_key(h, &mut rng, &hs);
                    let v = random_value(h, &mut rng, &hs);
                    let o = h.get_handle(o).as_object().unwrap();
                    let _ = h.create_data_property(o, k, v);
                }
            }
            8 => {
                if let Some(o) = pick(h, &mut rng, &hs, is_obj) {
                    let o = h.get_handle(o).as_object().unwrap();
                    let keys = h.own_keys(o);
                    if !keys.is_empty() {
                        let k = keys[rng.below(keys.len())];
                        h.delete_own_property(o, k);
                    }
                }
            }
            9 => {
                if hs.len() > 2 {
                    let i = 1 + rng.below(hs.len() - 1);
                    h.drop_handle(hs.swap_remove(i));
                }
            }
            10 => {
                if let Some(o) = pick(h, &mut rng, &hs, is_obj) {
                    let k = random_key(h, &mut rng, &hs);
                    let g = random_value(h, &mut rng, &hs);
                    let g = if is_obj(g) { g } else { Value::UNDEFINED };
                    let fl = PropFlags::from_bits(rng.below(8) as u8);
                    let o = h.get_handle(o).as_object().unwrap();
                    let d = if rng.chance(50) {
                        PropertyDescriptor::accessor(g, Value::UNDEFINED, fl)
                    } else {
                        PropertyDescriptor::data(g, fl)
                    };
                    let _ = h.define_own_property(o, k, &d);
                }
            }
            11 => {
                let desc = if rng.chance(50) {
                    Value::string(h.alloc_str(&format!("sym{op}")).unwrap())
                } else {
                    Value::UNDEFINED
                };
                let s = h.new_symbol(desc).unwrap();
                hs.push(h.new_handle(Value::symbol(s)).unwrap());
            }
            12 => {
                // Array elements: dense appends, holes, sparse.
                if let Some(a) = pick(h, &mut rng, &hs, |v| v.as_object().is_some()) {
                    let o = h.get_handle(a).as_object().unwrap();
                    if h.object_class(o) == ObjectClass::Array {
                        let v = random_value(h, &mut rng, &hs);
                        let i = if rng.chance(80) {
                            h.array_length(o)
                        } else {
                            rng.below(500) as u32
                        };
                        if !h.set_element_fast(o, i, v).unwrap() {
                            let _ = h.create_data_property(o, Atom::int(i).unwrap(), v);
                        }
                    }
                }
            }
            _ => {
                if let (Some(x), Some(y)) = (
                    pick(h, &mut rng, &hs, |_| true),
                    pick(h, &mut rng, &hs, |_| true),
                ) {
                    let v = h.get_handle(y);
                    h.set_handle(x, v);
                }
            }
        }
    }
    hs
}

#[test]
fn random_graphs_survive_collection() {
    for seed in 1..=40u64 {
        let mut h = Heap::new().unwrap();
        let hs = build_random(&mut h, seed.wrapping_mul(0x9E37_79B9_7F4A_7C15), 400, None);
        h.verify().unwrap();
        let d0 = dump(&h, &hs);
        let stats = h.collect().unwrap();
        assert!(stats.after <= stats.before);
        h.verify().unwrap();
        assert_eq!(dump(&h, &hs), d0, "seed {seed}");
        // Collecting again is a fixpoint, byte for byte.
        let b1 = h.bytes().to_vec();
        let s2 = h.collect().unwrap();
        assert_eq!(s2.before, s2.after);
        assert_eq!(h.bytes(), &b1[..], "seed {seed}");
        // The image round trip still works, handles included.
        let mut r = Heap::from_bytes(b1, DEFAULT_LIMIT).unwrap();
        assert_eq!(dump(&r, &hs), d0);
        r.collect().unwrap();
        assert_eq!(dump(&r, &hs), d0);
    }
}

#[test]
fn collecting_during_construction_changes_nothing() {
    // The same mutation sequence with and without collections in between ends in the same
    // graph.
    for seed in 1..=20u64 {
        let seed = seed.wrapping_mul(0xD1B5_4A32_D192_ED03);
        let mut a = Heap::new().unwrap();
        let mut b = Heap::new().unwrap();
        let ha = build_random(&mut a, seed, 600, None);
        let hb = build_random(&mut b, seed, 600, Some(37));
        assert_eq!(ha, hb);
        b.verify().unwrap();
        assert_eq!(dump(&a, &ha), dump(&b, &hb), "seed {seed}");
        // Both collected: identical images.
        a.collect().unwrap();
        b.collect().unwrap();
        assert_eq!(a.bytes(), b.bytes(), "seed {seed}");
    }
}

// ------------------------------------------------------------------ focused tests

#[test]
fn garbage_is_freed() {
    let mut h = Heap::new().unwrap();
    let base = h.collect().unwrap().after;
    let keep = h.alloc_str("keep me").unwrap();
    let hk = h.new_handle(Value::string(keep)).unwrap();
    for i in 0..10_000 {
        let o = h.new_object(None).unwrap();
        let s = h.alloc_str(&format!("garbage {i}")).unwrap();
        h.create_data_property(o, atoms::VALUE, Value::string(s))
            .unwrap();
    }
    let st = h.collect().unwrap();
    assert!(st.before > 500_000, "{st:?}");
    assert!(st.after < base + 1024, "{st:?} base {base}");
    assert_eq!(h.arena().len(), st.after);
    let s = h.get_handle(hk).as_string().unwrap();
    assert!(h.str_eq_str(s, "keep me"));
    h.verify().unwrap();
}

#[test]
fn handles() {
    let mut h = Heap::new().unwrap();
    let o = h.new_object(None).unwrap();
    let a = h.new_handle(Value::object(o)).unwrap();
    let b = h.new_handle(Value::int(1)).unwrap();
    assert_eq!(h.handle_count(), 2);
    assert_eq!(h.get_handle(b), Value::int(1));
    h.set_handle(b, Value::TRUE);
    assert_eq!(h.get_handle(b), Value::TRUE);
    h.drop_handle(a);
    assert_eq!(h.handle_count(), 1);
    // Freed slots are reused.
    let c = h.new_handle(Value::NULL).unwrap();
    assert_eq!(c, a);
    // Growth keeps every handle.
    let many: Vec<Handle> = (0..100)
        .map(|i| h.new_handle(Value::int(i)).unwrap())
        .collect();
    for (i, &x) in many.iter().enumerate() {
        assert_eq!(h.get_handle(x), Value::int(i as i32));
    }
    for &x in many.iter().step_by(3) {
        h.drop_handle(x);
    }
    h.collect().unwrap();
    for (i, &x) in many.iter().enumerate() {
        if i % 3 != 0 {
            assert_eq!(h.get_handle(x), Value::int(i as i32));
        }
    }
    assert_eq!(h.handle_count(), 2 + 66);
    h.verify().unwrap();
}

#[test]
#[should_panic(expected = "dropped")]
fn double_drop_panics() {
    let mut h = Heap::new().unwrap();
    let a = h.new_handle(Value::NULL).unwrap();
    h.drop_handle(a);
    h.drop_handle(a);
}

#[test]
fn handles_keep_objects_alive_and_follow_moves() {
    let mut h = Heap::new().unwrap();
    // Garbage first, so the live object moves down.
    for _ in 0..100 {
        h.new_object(None).unwrap();
    }
    let o = h.new_object(None).unwrap();
    let k = h.atom_str("answer").unwrap();
    h.create_data_property(o, k, Value::int(42)).unwrap();
    let ho = h.new_handle(Value::object(o)).unwrap();
    h.collect().unwrap();
    let o2 = h.get_handle(ho).as_object().unwrap();
    assert_ne!(o2, o);
    assert_eq!(
        h.get_own_property(o2, k).and_then(|p| p.value()),
        Some(Value::int(42))
    );
    // Without the handle the object is collected.
    h.drop_handle(ho);
    let before = h.arena().len();
    h.collect().unwrap();
    assert!(h.arena().len() < before);
}

#[test]
fn flattened_ropes_are_short_circuited() {
    let mut h = Heap::new().unwrap();
    let x = h.alloc_str(&"x".repeat(40)).unwrap();
    let y = h.alloc_str(&"€".repeat(40)).unwrap();
    let flat_rope = h.concat(x, y).unwrap();
    h.flatten(flat_rope).unwrap();
    let rope = h.concat(y, x).unwrap();
    let hf = h.new_handle(Value::string(flat_rope)).unwrap();
    let hr = h.new_handle(Value::string(rope)).unwrap();
    // A rope whose child is the flattened rope.
    let outer = h.concat(flat_rope, x).unwrap();
    let ho = h.new_handle(Value::string(outer)).unwrap();
    h.collect().unwrap();
    let f = h.get_handle(hf).as_string().unwrap();
    assert_eq!(h.cell_kind(f.0), CellKind::Str16);
    assert_eq!(h.str_to_string(f), "x".repeat(40) + &"€".repeat(40));
    let r = h.get_handle(hr).as_string().unwrap();
    assert_eq!(h.cell_kind(r.0), CellKind::Rope);
    assert_eq!(h.str_to_string(r), "€".repeat(40) + &"x".repeat(40));
    let o = h.get_handle(ho).as_string().unwrap();
    let left = h.arena().get(o.0, cells::Rope::LEFT).0;
    assert_eq!(left, f.0, "child short-circuited to the shared flat string");
    assert_eq!(h.str_len(o), 120);
    // No flattened rope survives.
    assert!(
        h.cells().all(|c| c.kind != CellKind::Rope
            || h.cell_flags(c.offset) & cells::flags::ROPE_FLATTENED == 0)
    );
    h.verify().unwrap();
}

#[test]
fn atoms_and_symbols_survive() {
    let mut h = Heap::new().unwrap();
    let ids: Vec<Atom> = (0..500)
        .map(|i| h.atom_str(&format!("atom{i}")).unwrap())
        .collect();
    let d = h.alloc_str("desc").unwrap();
    let sym = h.new_symbol(Value::string(d)).unwrap();
    let ks = h.symbol_atom(sym);
    // Garbage between atoms.
    for _ in 0..1000 {
        h.alloc_str("junk junk junk").unwrap();
    }
    h.collect().unwrap();
    for (i, &a) in ids.iter().enumerate() {
        assert_eq!(h.atom_str(&format!("atom{i}")).unwrap(), a);
        assert_eq!(h.atom_name(a), format!("atom{i}"));
    }
    assert_eq!(h.atom_str("length").unwrap(), atoms::LENGTH);
    // The symbol is kept by the atom table (atoms are not collected yet).
    let sym2 = h.atom_symbol(ks).unwrap();
    assert_eq!(h.symbol_atom(sym2), ks);
    let d2 = h.symbol_description(sym2).as_string().unwrap();
    assert!(h.str_eq_str(d2, "desc"));
    let it = h.atom_symbol(atoms::SYM_ITERATOR).unwrap();
    assert_eq!(h.atom_name(h.symbol_atom(it)), "[Symbol.iterator]");
    // New atoms still intern correctly after the move.
    let n = h.atom_str("fresh").unwrap();
    assert_eq!(h.atom_str("fresh").unwrap(), n);
    h.verify().unwrap();
}

#[test]
fn gc_pending_follows_the_threshold() {
    let mut h = Heap::new().unwrap();
    assert!(!h.gc_pending());
    let t = h.gc_threshold();
    assert!(t >= h.arena().len() + gc::MIN_GROWTH.min(1 << 20) / 2);
    while h.arena().len() < t {
        assert!(!h.gc_pending());
        h.alloc_str("some garbage string").unwrap();
    }
    assert!(h.gc_pending());
    h.collect().unwrap();
    assert!(!h.gc_pending());
    assert_eq!(
        h.gc_threshold(),
        gc::next_threshold(h.arena().len(), h.arena().limit())
    );
}

#[test]
fn failed_collection_leaves_heap_intact() {
    let mut h = Heap::new().unwrap();
    let hs = build_random(&mut h, 77, 300, None);
    let bytes = h.bytes().to_vec();
    let d = dump(&h, &hs);
    h.fail_next_gc_reserve = true;
    assert_eq!(h.collect(), Err(HeapError::OutOfMemory));
    assert_eq!(h.bytes(), &bytes[..]);
    assert_eq!(dump(&h, &hs), d);
    h.collect().unwrap();
    assert_eq!(dump(&h, &hs), d);
}

#[test]
fn collection_under_a_tight_cap() {
    // Fill a small heap to its cap with mostly garbage; collecting makes room again.
    let mut h = Heap::with_limit(256 * 1024).unwrap();
    let keep = h.new_object(None).unwrap();
    let hk = h.new_handle(Value::object(keep)).unwrap();
    let mut rounds = 0;
    for i in 0..20_000u32 {
        let r = (|| -> Result<(), HeapError> {
            let s = h.alloc_str(&format!("garbage string number {i}"))?;
            if i % 100 == 0 {
                let o = h.get_handle(hk).as_object().unwrap();
                let k = h.atom_from_index(i / 100)?;
                h.create_data_property(o, k, Value::string(s))?;
            }
            Ok(())
        })();
        if r.is_err() || h.gc_pending() {
            h.collect().unwrap();
            rounds += 1;
            if r.is_err() {
                // Retry once after collecting; it must fit now.
                let s = h.alloc_str(&format!("garbage string number {i}")).unwrap();
                if i % 100 == 0 {
                    let o = h.get_handle(hk).as_object().unwrap();
                    let k = h.atom_from_index(i / 100).unwrap();
                    h.create_data_property(o, k, Value::string(s)).unwrap();
                }
            }
        }
    }
    assert!(rounds > 3, "{rounds}");
    let o = h.get_handle(hk).as_object().unwrap();
    assert_eq!(h.own_keys(o).len(), 200);
    h.verify().unwrap();
}

#[test]
fn empty_heap_collects() {
    let mut h = Heap::new().unwrap();
    let b = h.bytes().to_vec();
    let st = h.collect().unwrap();
    assert!(st.after <= st.before);
    h.verify().unwrap();
    let again = h.bytes().to_vec();
    h.collect().unwrap();
    assert_eq!(h.bytes(), &again[..]);
    let _ = b;
}

/// Rough collection throughput. Run with
/// `cargo test -p djs-core --release gc_throughput -- --ignored --nocapture`.
#[test]
#[ignore]
fn gc_throughput() {
    let mut h = Heap::new().unwrap();
    // A codemode-like live set: arrays of similar small objects with string values.
    let root = h.new_array(None, 0).unwrap();
    let hr = h.new_handle(Value::object(root)).unwrap();
    let keys: Vec<Atom> = ["id", "name", "path", "size", "tags"]
        .iter()
        .map(|k| h.atom_str(k).unwrap())
        .collect();
    for i in 0..200_000u32 {
        let o = h.new_object(None).unwrap();
        for (j, &k) in keys.iter().enumerate() {
            let v = if j % 2 == 0 {
                Value::int(i as i32)
            } else {
                Value::string(h.alloc_str(&format!("value {i} {j}")).unwrap())
            };
            h.create_data_property(o, k, v).unwrap();
        }
        let root = h.get_handle(hr).as_object().unwrap();
        h.set_element_fast(root, i, Value::object(o)).unwrap();
    }
    report("records", &mut h);

    // Pointer dense: a linked list of small objects, plus ropes.
    let mut h = Heap::new().unwrap();
    let k = h.atom_str("next").unwrap();
    let head = h.new_handle(Value::NULL).unwrap();
    let piece = h.alloc_str(&"x".repeat(40)).unwrap();
    let hp = h.new_handle(Value::string(piece)).unwrap();
    for _ in 0..500_000 {
        let o = h.new_object(None).unwrap();
        let next = h.get_handle(head);
        h.create_data_property(o, k, next).unwrap();
        let p = h.get_handle(hp).as_string().unwrap();
        let r = h.concat(p, p).unwrap();
        h.create_data_property(o, atoms::VALUE, Value::string(r))
            .unwrap();
        h.set_handle(head, Value::object(o));
    }
    report("linked list", &mut h);
}

fn report(name: &str, h: &mut Heap) {
    extern crate std;
    use std::time::Instant;
    h.collect().unwrap();
    let live = h.arena().len() as f64 / (1 << 20) as f64;
    let cells = h.cells().count();
    let n = 10;
    let t = Instant::now();
    for _ in 0..n {
        h.collect().unwrap();
    }
    let per = t.elapsed().as_secs_f64() / n as f64;
    std::println!(
        "{name}: live {live:.1} MiB in {cells} cells, {:.2} ms per collection, {:.2} ms per MiB",
        per * 1e3,
        per * 1e3 / live
    );
}

#[test]
fn internal_refs_are_traced_and_verified() {
    let mut h = Heap::new().unwrap();
    for _ in 0..100 {
        h.alloc_str("garbage before the internal cell").unwrap();
    }
    // An internal cell (a ValueArray holding a string) in an object slot.
    let arr = h.alloc_value_array(2).unwrap();
    let s = h.alloc_str("inside").unwrap();
    h.value_array_set(arr, 0, Value::string(s));
    h.arena_mut().set(arr, cells::ValueArray::LEN, 1);
    let o = h.alloc_object(ObjectClass::Ordinary, None, 1).unwrap();
    h.set_object_slot(o, 0, Value::internal(arr));
    let ho = h.new_handle(Value::object(o)).unwrap();
    h.verify().unwrap();
    h.collect().unwrap();
    h.verify().unwrap();
    let o = h.get_handle(ho).as_object().unwrap();
    let arr2 = h.object_slot(o, 0).as_internal().unwrap();
    assert_ne!(arr2, arr, "moved");
    assert_eq!(h.cell_kind(arr2), CellKind::ValueArray);
    let s = h.value_array_get(arr2, 0).as_string().unwrap();
    assert!(h.str_eq_str(s, "inside"));

    // An internal reference to a JS cell is rejected.
    let p = h.new_object(None).unwrap();
    h.set_object_slot(o, 0, Value::internal(p.offset()));
    assert!(matches!(h.verify(), Err(walk::VerifyError::BadRef { .. })));
}
