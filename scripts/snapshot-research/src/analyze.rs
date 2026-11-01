//! Image analysis: histograms, code/data partition, graph matching against a reference
//! image (what an old-generation delta would contain), and suspended activations.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use djs_core::bytecode::decode::decode;
use djs_core::bytecode::{Handler, HandlerTable, stack_depths};
use djs_core::heap::cells::{
    AccessorPair, Bytes, FuncTemplate, HANDLE_LIVE, HandleSlot, HandleTable, Object, PropEntry,
    PropTable, Realm, Roots, Script, U32Array, ValueArray,
};
use djs_core::heap::walk::Slot;
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass, Ref};
use djs_core::value::Value;

pub fn load(bytes: &[u8]) -> Heap {
    Heap::from_bytes_trusted(bytes.to_vec(), 1 << 30).expect("load")
}

pub fn kind_name(k: CellKind) -> String {
    format!("{k:?}")
}

pub fn class_of(h: &Heap, o: u32) -> ObjectClass {
    ObjectClass::from_u32(h.arena().get(o, Object::CLASS)).unwrap()
}

pub fn slots(h: &Heap, c: u32) -> Vec<Slot> {
    let mut v = Vec::new();
    h.trace_cell(c, |s| v.push(s));
    v
}

pub fn slot_pos(s: Slot) -> (u32, u32) {
    match s {
        Slot::Value(o) => (o, 8),
        Slot::Ref(o) => (o, 4),
    }
}

/// Bytes per cell kind and per object class.
pub fn histogram(h: &Heap) -> (BTreeMap<String, (u64, u64)>, BTreeMap<String, (u64, u64)>) {
    let mut kinds = BTreeMap::new();
    let mut classes = BTreeMap::new();
    for c in h.cells() {
        let e = kinds.entry(kind_name(c.kind)).or_insert((0, 0));
        e.0 += 1;
        e.1 += c.size as u64;
        if c.kind == CellKind::Object {
            let e = classes
                .entry(format!("{:?}", class_of(h, c.offset)))
                .or_insert((0, 0));
            e.0 += 1;
            e.1 += c.size as u64;
        }
    }
    (kinds, classes)
}

pub fn value_array(h: &Heap, a: u32) -> Vec<Value> {
    let n: u32 = h.arena().get(a, ValueArray::LEN);
    (0..n)
        .map(|i| h.arena().read::<Value>(ValueArray::VALUES.at(a, i)))
        .collect()
}

pub fn u32_array(h: &Heap, a: u32) -> Vec<u32> {
    if a == 0 {
        return vec![];
    }
    let n: u32 = h.arena().get(a, U32Array::LEN);
    (0..n)
        .map(|i| h.arena().read::<u32>(U32Array::DATA.at(a, i)))
        .collect()
}

pub fn bytes_of(h: &Heap, b: u32) -> Vec<u8> {
    if b == 0 {
        return vec![];
    }
    let n: u32 = h.arena().get(b, Bytes::LEN);
    h.arena().slice(Bytes::DATA.at(b, 0), n).to_vec()
}

/// The live handle values.
pub fn handles(h: &Heap) -> Vec<Value> {
    let t: Ref = h.arena().get(0, Roots::HANDLES);
    let Some(t) = t.get() else { return vec![] };
    let n: u32 = h.arena().get(t, HandleTable::LEN);
    let mut out = vec![];
    for i in 0..n {
        let s = HandleTable::SLOTS.at(t, i);
        if h.arena().get::<u32>(s, HandleSlot::NEXT) == HANDLE_LIVE {
            out.push(h.arena().get::<Value>(s, HandleSlot::VALUE));
        }
    }
    out
}

/// The file name of a template's script.
pub fn template_script_name(h: &Heap, t: u32) -> String {
    let s: Ref = h.arena().get(t, FuncTemplate::SCRIPT);
    let Some(s) = s.get() else { return "?".into() };
    let n: Ref = h.arena().get(s, Script::NAME);
    n.get()
        .map(|n| h.str_to_string(string_from_offset(n)))
        .unwrap_or("?".into())
}

/// A string reference for the string cell at `off` (Str has no public constructor).
pub fn string_from_offset(off: u32) -> djs_core::value::Str {
    let bits =
        djs_core::value::TAG_PREFIX | ((djs_core::value::tag::STRING as u64) << 32) | off as u64;
    Value::from_bits(bits).unwrap().as_string().unwrap()
}

/// Code vs data: bytes of templates and their parts, by script name; script sources.
pub fn code_partition(h: &Heap) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    let mut owned: HashSet<u32> = HashSet::new();
    for c in h.cells() {
        if c.kind != CellKind::FuncTemplate {
            continue;
        }
        let t = c.offset;
        let name = template_script_name(h, t);
        let mut bytes = c.size as u64;
        for f in [
            FuncTemplate::CODE,
            FuncTemplate::CONSTS,
            FuncTemplate::CHILDREN,
            FuncTemplate::CAPTURE_DESC,
            FuncTemplate::HANDLERS,
            FuncTemplate::LINES,
            FuncTemplate::NAMES,
            FuncTemplate::IC,
        ] {
            let r: Ref = h.arena().get(t, f);
            if let Some(r) = r.get()
                && owned.insert(r)
            {
                bytes += h.cell_size(r) as u64;
            }
        }
        *out.entry(format!("code {name}")).or_insert(0) += bytes;
    }
    for c in h.cells() {
        if c.kind != CellKind::Script {
            continue;
        }
        let n: Ref = h.arena().get(c.offset, Script::NAME);
        let name = n
            .get()
            .map(|n| h.str_to_string(string_from_offset(n)))
            .unwrap_or_default();
        let s: Ref = h.arena().get(c.offset, Script::SOURCE);
        if let Some(s) = s.get() {
            *out.entry(format!("source {name}")).or_insert(0) += h.cell_size(s) as u64;
        }
    }
    out
}

// ---- Graph matching ------------------------------------------------------------------

pub struct Matching {
    pub h2r: HashMap<u32, u32>,
    pub r2h: HashMap<u32, u32>,
}

/// Matches the cells of `h` to those of the reference `r` by a parallel BFS from the
/// roots, following slots pairwise (the same slot position in cells of the same kind).
pub fn match_graphs(r: &Heap, h: &Heap) -> Matching {
    let mut m = Matching {
        h2r: HashMap::new(),
        r2h: HashMap::new(),
    };
    let mut q = VecDeque::new();
    m.h2r.insert(0, 0);
    m.r2h.insert(0, 0);
    q.push_back((0u32, 0u32));
    while let Some((rc, hc)) = q.pop_front() {
        let rs = slots(r, rc);
        let hs = slots(h, hc);
        for (a, b) in rs.iter().zip(hs.iter()) {
            let (Some(rt), Some(ht)) = (r.slot_target(*a), h.slot_target(*b)) else {
                continue;
            };
            if m.r2h.contains_key(&rt) || m.h2r.contains_key(&ht) {
                continue;
            }
            if r.cell_kind(rt) != h.cell_kind(ht) {
                continue;
            }
            m.r2h.insert(rt, ht);
            m.h2r.insert(ht, rt);
            q.push_back((rt, ht));
        }
    }
    m
}

/// Whether matched cells have the same content (refs compared through the matching),
/// and how many 8-byte words differ.
pub fn compare(r: &Heap, h: &Heap, m: &Matching, rc: u32, hc: u32) -> (bool, u32) {
    let size = r.cell_size(rc);
    if size != h.cell_size(hc) {
        let words = h.cell_size(hc).div_ceil(8);
        return (false, words);
    }
    let rb = r.arena().slice(rc, size);
    let hb = h.arena().slice(hc, size);
    let mut masked = vec![false; size as usize];
    let mut diff_words: HashSet<u32> = HashSet::new();
    let rs = slots(r, rc);
    let hs = slots(h, hc);
    for (a, b) in rs.iter().zip(hs.iter()) {
        let (pa, w) = slot_pos(*a);
        let rel = pa - rc;
        for i in 0..w {
            masked[(rel + i) as usize] = true;
        }
        let same = match (r.slot_target(*a), h.slot_target(*b)) {
            (Some(rt), Some(ht)) => m.h2r.get(&ht) == Some(&rt),
            (None, None) => {
                rb[rel as usize..(rel + w) as usize] == hb[rel as usize..(rel + w) as usize]
            }
            _ => false,
        };
        if !same {
            diff_words.insert(rel / 8);
        }
    }
    for i in 0..size as usize {
        if !masked[i] && rb[i] != hb[i] {
            diff_words.insert(i as u32 / 8);
        }
    }
    (diff_words.is_empty(), diff_words.len() as u32)
}

#[derive(Default, Debug, Clone)]
pub struct DeltaEstimate {
    pub new_cells: u64,
    pub new_bytes: u64,
    pub mutated_cells: u64,
    pub mutated_bytes: u64,
    pub changed_words: u64,
    pub dirty_pages: u64,
    pub ref_pages: u64,
    pub unchanged_bytes: u64,
    /// New bytes by kind.
    pub new_by_kind: BTreeMap<String, u64>,
    pub mutated_by_kind: BTreeMap<String, u64>,
}

pub fn delta_estimate(r: &Heap, h: &Heap, m: &Matching) -> (DeltaEstimate, HashSet<u32>) {
    let mut d = DeltaEstimate::default();
    let mut pages = HashSet::new();
    let mut mutated = HashSet::new();
    for c in h.cells() {
        match m.h2r.get(&c.offset) {
            None => {
                d.new_cells += 1;
                d.new_bytes += c.size as u64;
                *d.new_by_kind.entry(kind_name(c.kind)).or_insert(0) += c.size as u64;
            }
            Some(&rc) => {
                let (same, words) = compare(r, h, m, rc, c.offset);
                if same {
                    d.unchanged_bytes += c.size as u64;
                } else {
                    mutated.insert(c.offset);
                    d.mutated_cells += 1;
                    d.mutated_bytes += c.size as u64;
                    d.changed_words += words as u64;
                    *d.mutated_by_kind.entry(kind_name(c.kind)).or_insert(0) += c.size as u64;
                    let size = r.cell_size(rc);
                    for p in rc / 4096..=(rc + size - 1) / 4096 {
                        pages.insert(p);
                    }
                }
            }
        }
    }
    d.dirty_pages = pages.len() as u64;
    d.ref_pages = (r.arena().len() as u64).div_ceil(4096);
    (d, mutated)
}

// ---- Paths: version-independent names of reference cells -------------------------------

/// A BFS tree over the reference heap with labelled edges: property names, `^` for the
/// prototype, `~name` for captured bindings, `%k` for realm intrinsics, `$k` for handles.
pub struct Paths {
    pub parent: HashMap<u32, (u32, String)>,
}

pub fn labelled_edges(h: &Heap, c: u32) -> Vec<(String, u32)> {
    let a = h.arena();
    let mut out = vec![];
    match h.cell_kind(c) {
        CellKind::Object => {
            let proto: Ref = a.get(c, Object::PROTO);
            if let Some(p) = proto.get() {
                out.push(("^".into(), p));
            }
            let t: Ref = a.get(c, Object::PROPS);
            if let Some(t) = t.get() {
                let n: u32 = a.get(t, PropTable::COUNT);
                for i in 0..n {
                    let e = PropTable::ENTRIES.at(t, i);
                    let key: u32 = a.get(e, PropEntry::KEY);
                    if key == 0 {
                        continue;
                    }
                    let name = h.atom_name(Atom::from_id(key));
                    let v: Value = a.get(e, PropEntry::VALUE);
                    if let Some(t) = v.ref_offset() {
                        if h.cell_kind(t) == CellKind::AccessorPair {
                            for (s, f) in
                                [("get", AccessorPair::GETTER), ("set", AccessorPair::SETTER)]
                            {
                                let g: Value = a.get(t, f);
                                if let Some(g) = g.ref_offset() {
                                    out.push((format!(".{name}/{s}"), g));
                                }
                            }
                        } else {
                            out.push((format!(".{name}"), t));
                        }
                    }
                }
            }
            let el: Ref = a.get(c, Object::ELEMENTS);
            if let Some(el) = el.get() {
                for (i, v) in value_array(h, el).into_iter().enumerate() {
                    if let Some(t) = v.ref_offset() {
                        out.push((format!("[{i}]"), t));
                    }
                }
            }
            let nslots = (h.cell_size(c) - Object::LAYOUT.min_size) / 8;
            let class = class_of(h, c);
            let template = (class == ObjectClass::Function)
                .then(|| a.read::<Value>(Object::SLOTS.at(c, 0)).ref_offset())
                .flatten();
            for i in 0..nslots {
                let v = a.read::<Value>(Object::SLOTS.at(c, i));
                let Some(t) = v.ref_offset() else { continue };
                let label = match (template, i) {
                    (Some(_), 0) => "@fn".to_string(),
                    (Some(tpl), i) => {
                        let locals: u16 = a.get(tpl, FuncTemplate::LOCALS);
                        let names: Ref = a.get(tpl, FuncTemplate::NAMES);
                        let names = u32_array(h, names.0);
                        names
                            .get(locals as usize + i as usize - 1)
                            .map(|&n| format!("~{}", h.atom_name(Atom::from_id(n))))
                            .unwrap_or(format!("~{}", i - 1))
                    }
                    _ => format!("#{i}"),
                };
                out.push((label, t));
            }
        }
        _ => {
            for (i, s) in slots(h, c).into_iter().enumerate() {
                if let Some(t) = h.slot_target(s) {
                    out.push((format!("#{i}"), t));
                }
            }
        }
    }
    out
}

pub fn paths(h: &Heap) -> Paths {
    let mut parent: HashMap<u32, (u32, String)> = HashMap::new();
    let mut q = VecDeque::new();
    let realm = h.realm().as_internal().unwrap();
    let seed =
        |t: u32, label: String, q: &mut VecDeque<u32>, parent: &mut HashMap<u32, (u32, String)>| {
            if !parent.contains_key(&t) {
                parent.insert(t, (u32::MAX, label));
                q.push_back(t);
            }
        };
    // The global object first (intrinsic 0), then the realm's other intrinsics, then
    // handles: the names a reader would use.
    let a = h.arena();
    let n_intr = (h.cell_size(realm) - Realm::LAYOUT.min_size) / 8;
    for i in 0..n_intr {
        let v = a.read::<Value>(Realm::INTRINSICS.at(realm, i));
        if let Some(t) = v.ref_offset() {
            seed(
                t,
                if i == 0 {
                    "global".into()
                } else {
                    format!("%{i}")
                },
                &mut q,
                &mut parent,
            );
        }
    }
    let lex: Ref = a.get(realm, Realm::GLOBAL_LEX);
    if let Some(t) = lex.get() {
        seed(t, "lex".into(), &mut q, &mut parent);
    }
    for (i, v) in handles(h).into_iter().enumerate() {
        if let Some(t) = v.ref_offset() {
            seed(t, format!("${i}"), &mut q, &mut parent);
        }
    }
    while let Some(c) = q.pop_front() {
        if h.cell_kind(c) == CellKind::AtomTable {
            continue;
        }
        for (label, t) in labelled_edges(h, c) {
            if let std::collections::hash_map::Entry::Vacant(e) = parent.entry(t) {
                e.insert((c, label));
                q.push_back(t);
            }
        }
    }
    Paths { parent }
}

impl Paths {
    /// Bytes of a path trie holding `cells` and their ancestors: per node a varint
    /// parent index and the label.
    pub fn trie_bytes(&self, cells: &HashSet<u32>) -> (u64, u64) {
        let mut nodes: HashSet<u32> = HashSet::new();
        let mut bytes = 0u64;
        for &c in cells {
            let mut cur = c;
            while nodes.insert(cur) {
                let Some((p, l)) = self.parent.get(&cur) else {
                    break;
                };
                bytes += 2 + l.len() as u64;
                if *p == u32::MAX {
                    break;
                }
                cur = *p;
            }
        }
        (nodes.len() as u64, bytes)
    }
}

// ---- Suspended activations -------------------------------------------------------------

/// Printed with `{:?}` in the report.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Activation {
    pub script: String,
    pub function: String,
    pub kind: i32,
    pub state: i32,
    pub op: String,
    pub frame_len: u32,
    pub args: u32,
    pub locals: u32,
    pub operands: u32,
    pub operand_kinds: Vec<String>,
}

pub fn activations(h: &Heap) -> Vec<Activation> {
    let a = h.arena();
    let mut out = vec![];
    for c in h.cells() {
        if c.kind != CellKind::Object || class_of(h, c.offset) != ObjectClass::Generator {
            continue;
        }
        let slot = |i: u32| a.read::<Value>(Object::SLOTS.at(c.offset, i));
        let frame = slot(1);
        let Some(fr) = frame.ref_offset() else {
            continue;
        };
        let vals = value_array(h, fr);
        let pc = slot(2).as_int().unwrap_or(-1) as u32;
        let hp = slot(3).as_int().unwrap_or(0) as u32;
        let callee = vals[0];
        let f = callee.as_object().unwrap().offset();
        let tpl = a
            .read::<Value>(Object::SLOTS.at(f, 0))
            .ref_offset()
            .unwrap();
        let locals: u16 = a.get(tpl, FuncTemplate::LOCALS);
        let code: Ref = a.get(tpl, FuncTemplate::CODE);
        let code = bytes_of(h, code.0);
        let mut op = "?".to_string();
        let mut p = 0u32;
        while (p as usize) < code.len() {
            let i = decode(&code, p).unwrap();
            if i.next_pc() == pc {
                op = i.info().name.to_string();
                break;
            }
            p = i.next_pc();
        }
        let name: u32 = a.get(tpl, FuncTemplate::NAME);
        let operands = vals.len() as u32 - hp - 5 - locals as u32;
        let operand_kinds = vals[(hp + 5 + locals as u32) as usize..]
            .iter()
            .map(|v| describe(h, *v))
            .collect();
        out.push(Activation {
            script: template_script_name(h, tpl),
            function: h.atom_name(Atom::from_id(name)),
            kind: slot(5).as_int().unwrap_or(-1),
            state: slot(0).as_int().unwrap_or(-1),
            op,
            frame_len: vals.len() as u32,
            args: hp - 2,
            locals: locals as u32,
            operands,
            operand_kinds,
        });
    }
    out
}

pub fn describe(h: &Heap, v: Value) -> String {
    use djs_core::value::Kind;
    match v.kind() {
        Kind::Object => format!("{:?}", class_of(h, v.as_object().unwrap().offset())),
        Kind::Internal => kind_name(h.cell_kind(v.as_internal().unwrap())),
        k => format!("{k:?}"),
    }
}

/// Static: for every await/yield in the templates of `script`, the operand depth below
/// the awaited value.
pub fn await_depths(h: &Heap, script: &str) -> Vec<(String, u32)> {
    let a = h.arena();
    let mut out = vec![];
    for c in h.cells() {
        if c.kind != CellKind::FuncTemplate || template_script_name(h, c.offset) != script {
            continue;
        }
        let code: Ref = a.get(c.offset, FuncTemplate::CODE);
        let code = bytes_of(h, code.0);
        let hb: Ref = a.get(c.offset, FuncTemplate::HANDLERS);
        let hbytes = bytes_of(h, hb.0);
        let table = HandlerTable::new(&hbytes).unwrap();
        let handlers: Vec<Handler> = table.iter().collect();
        let Ok(depths) = stack_depths(&code, &handlers) else {
            continue;
        };
        let mut p = 0u32;
        while (p as usize) < code.len() {
            let i = decode(&code, p).unwrap();
            let name = i.info().name;
            if name == "await" || name == "yield" || name == "yield_star" {
                let d = depths[p as usize];
                out.push((name.to_string(), d.saturating_sub(1)));
            }
            p = i.next_pc();
        }
    }
    out
}

/// Builtin-internal state at a quiescent point: builtin closures by intrinsic name,
/// records, promises by state, and what the pending reactions are.
pub fn internals(h: &Heap) -> BTreeMap<String, u32> {
    use djs_core::bytecode::intrinsics::INTRINSICS;
    let a = h.arena();
    let mut out = BTreeMap::new();
    for c in h.cells() {
        if c.kind != CellKind::Object {
            continue;
        }
        let class = class_of(h, c.offset);
        let slot = |i: u32| a.read::<Value>(Object::SLOTS.at(c.offset, i));
        let key = match class {
            ObjectClass::BuiltinClosure => {
                let id = slot(0).as_int().unwrap() as usize;
                format!("BuiltinClosure {}", INTRINSICS[id].name)
            }
            ObjectClass::Record => format!("Record ({} slots)", (c.size - 24) / 8),
            ObjectClass::Promise => {
                let st = slot(0).as_int().unwrap_or(-1);
                let reactions = slot(2)
                    .ref_offset()
                    .map(|r| value_array(h, r).len() / 3)
                    .unwrap_or(0);
                format!("Promise state {st} reactions {reactions}")
            }
            ObjectClass::Generator => format!(
                "Generator kind {} state {}",
                slot(5).as_int().unwrap_or(-1),
                slot(0).as_int().unwrap_or(-1)
            ),
            ObjectClass::ArrayIterator
            | ObjectClass::CollectionIterator
            | ObjectClass::IteratorHelper
            | ObjectClass::RegExpStringIterator => format!("{class:?}"),
            _ => continue,
        };
        *out.entry(key).or_insert(0) += 1;
    }
    out
}

/// Local and capture names of the templates of `script` (hidden bindings show as `<..>`).
pub fn binding_names(h: &Heap, script: &str) -> Vec<String> {
    let a = h.arena();
    let mut out = vec![];
    for c in h.cells() {
        if c.kind != CellKind::FuncTemplate || template_script_name(h, c.offset) != script {
            continue;
        }
        let name: u32 = a.get(c.offset, FuncTemplate::NAME);
        let locals: u16 = a.get(c.offset, FuncTemplate::LOCALS);
        let names: Ref = a.get(c.offset, FuncTemplate::NAMES);
        let names: Vec<String> = u32_array(h, names.0)
            .into_iter()
            .map(|n| h.atom_name(Atom::from_id(n)))
            .collect();
        out.push(format!(
            "{} [locals {locals}: {}]",
            h.atom_name(Atom::from_id(name)),
            names.join(", ")
        ));
    }
    out
}
