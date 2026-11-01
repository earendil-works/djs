//! A prototype *logical* snapshot encoder: the object graph as records, with no engine
//! layout in it. Cells that exist in a reference world (the base realm, or the image
//! right after pi's prelude, which a restore can recompute) are named by path instead of
//! stored; changed ones get a patch record (their full new content); new ones a record.
//! Strings are deduplicated by content; templates are (script, source span); bytecode,
//! inline caches, hash indexes, atoms and compiled regexps are left out (recomputable).
//!
//! Only the encoder exists (sizes); the format is sketched in the issue file.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::heap::cells::{
    AccessorPair, BigInt as BigIntCell, BoxCell, FuncTemplate, HostCall, Interp, Object, PropEntry,
    PropTable, Realm, Roots, Script, Symbol,
};
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass, Ref};
use djs_core::value::{Kind, Value};

use crate::analyze::{
    Matching, Paths, class_of, handles, kind_name, string_from_offset, value_array,
};

pub struct Known<'a> {
    pub m: &'a Matching,
    pub mutated: &'a HashSet<u32>,
    pub paths: &'a Paths,
}

#[derive(Default, Debug)]
pub struct Stats {
    pub total: usize,
    pub strings: usize,
    pub string_count: usize,
    pub paths: usize,
    pub path_count: usize,
    pub shapes: usize,
    pub records: BTreeMap<String, (usize, usize)>,
    pub patches: BTreeMap<String, (usize, usize)>,
    pub new_units: usize,
    pub patch_units: usize,
    pub bytes: Vec<u8>,
}

struct Enc<'a> {
    h: &'a Heap,
    known: Option<Known<'a>>,
    out: Vec<u8>,
    strings: HashMap<String, u32>,
    str_bytes: Vec<u8>,
    nodes: HashMap<u32, u32>,
    queue: VecDeque<u32>,
    visited: HashSet<u32>,
    shapes: HashMap<Vec<(u64, u8)>, u32>,
    shape_bytes: usize,
    known_refs: HashSet<u32>,
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

impl<'a> Enc<'a> {
    fn str_id(&mut self, s: String) -> u32 {
        if let Some(&i) = self.strings.get(&s) {
            return i;
        }
        let i = self.strings.len() as u32;
        varint(&mut self.str_bytes, s.len() as u64);
        self.str_bytes.extend_from_slice(s.as_bytes());
        self.strings.insert(s, i);
        i
    }

    /// The storage cells an object owns (property table, its index, accessor pairs,
    /// elements): they are part of the object's record, never shared.
    fn storage(&self, o: u32) -> Vec<u32> {
        let a = self.h.arena();
        let mut v = vec![];
        let t: Ref = a.get(o, Object::PROPS);
        if let Some(t) = t.get() {
            v.push(t);
            let idx: Ref = a.get(t, PropTable::INDEX);
            if let Some(i) = idx.get() {
                v.push(i);
            }
            let n: u32 = a.get(t, PropTable::COUNT);
            for i in 0..n {
                let e = PropTable::ENTRIES.at(t, i);
                let val: Value = a.get(e, PropEntry::VALUE);
                if let Some(p) = val.ref_offset()
                    && self.h.cell_kind(p) == CellKind::AccessorPair
                {
                    v.push(p);
                }
            }
        }
        let el: Ref = a.get(o, Object::ELEMENTS);
        if let Some(el) = el.get() {
            v.push(el);
        }
        v
    }

    fn is_known(&self, c: u32) -> bool {
        self.known
            .as_ref()
            .is_some_and(|k| k.m.h2r.contains_key(&c))
    }

    fn changed(&self, c: u32) -> bool {
        let Some(k) = &self.known else { return true };
        let cell_changed = |x: u32| !k.m.h2r.contains_key(&x) || k.mutated.contains(&x);
        match self.h.cell_kind(c) {
            CellKind::Object => cell_changed(c) || self.storage(c).into_iter().any(cell_changed),
            // Templates are immutable; a filled inline cache is not a change.
            CellKind::FuncTemplate | CellKind::Script => !k.m.h2r.contains_key(&c),
            _ => cell_changed(c),
        }
    }

    /// Encodes a reference to cell `c` and schedules it.
    fn cell_ref(&mut self, c: u32) {
        self.schedule(c);
        if self.is_known(c) {
            self.known_refs.insert(c);
            self.out.push(10);
            // path index: count of known refs, roughly
            varint(&mut self.out, self.known_refs.len() as u64);
        } else {
            let n = self.nodes.len() as u32;
            let id = *self.nodes.entry(c).or_insert(n);
            self.out.push(9);
            varint(&mut self.out, id as u64);
        }
    }

    fn schedule(&mut self, c: u32) {
        if self.visited.insert(c) {
            self.queue.push_back(c);
        }
    }

    fn value(&mut self, v: Value) {
        match v.kind() {
            Kind::Undefined => self.out.push(0),
            Kind::Null => self.out.push(1),
            Kind::Bool => self.out.push(if v.as_bool().unwrap() { 3 } else { 2 }),
            Kind::Hole => self.out.push(4),
            Kind::Uninitialized => self.out.push(5),
            Kind::Number => {
                let n = v.as_number().unwrap();
                if n.fract() == 0.0 && n.abs() < 1e15 && !(n == 0.0 && n.is_sign_negative()) {
                    self.out.push(6);
                    let i = n as i64;
                    varint(&mut self.out, ((i << 1) ^ (i >> 63)) as u64);
                } else {
                    self.out.push(7);
                    self.out.extend_from_slice(&n.to_le_bytes());
                }
            }
            Kind::String => {
                let s = self.h.str_to_string(v.as_string().unwrap());
                let id = self.str_id(s);
                self.out.push(8);
                varint(&mut self.out, id as u64);
            }
            Kind::BigInt => {
                let c = v.as_bigint().unwrap();
                let n: u32 = self.h.arena().get(c, BigIntCell::LEN);
                self.out.push(11);
                varint(&mut self.out, n as u64);
                self.out.extend(std::iter::repeat_n(0u8, n as usize * 4));
            }
            Kind::Object | Kind::Symbol | Kind::Internal | Kind::HostRef => {
                self.cell_ref(v.ref_offset().unwrap())
            }
        }
    }

    fn ref_field(&mut self, r: Ref) {
        match r.get() {
            Some(c) => self.cell_ref(c),
            None => self.out.push(1),
        }
    }

    fn key(&mut self, key: u32) -> u64 {
        let a = Atom::from_id(key);
        if let Some(sym) = self.h.atom_symbol(a) {
            // symbols by identity: schedule and use a node/path marker
            let c = sym.offset();
            self.schedule(c);
            (1u64 << 40) | c as u64
        } else {
            self.str_id(self.h.atom_name(a)) as u64
        }
    }

    /// Emits the record of unit `c` (new or patched).
    fn record(&mut self, c: u32) -> String {
        let h = self.h;
        let a = h.arena();
        let kind = h.cell_kind(c);
        self.out.push(kind as u8);
        match kind {
            CellKind::Object => {
                let class = class_of(h, c);
                varint(&mut self.out, class as u64);
                self.out.push(h.cell_flags(c) as u8);
                let proto: Ref = a.get(c, Object::PROTO);
                self.ref_field(proto);
                // properties: a shape (keys and flags, shared) and the values
                let mut shape = vec![];
                let mut vals: Vec<(Value, Option<Value>)> = vec![];
                let t: Ref = a.get(c, Object::PROPS);
                if let Some(t) = t.get() {
                    let n: u32 = a.get(t, PropTable::COUNT);
                    for i in 0..n {
                        let e = PropTable::ENTRIES.at(t, i);
                        let k: u32 = a.get(e, PropEntry::KEY);
                        if k == 0 {
                            continue;
                        }
                        let f: u32 = a.get(e, PropEntry::FLAGS);
                        let kid = self.key(k);
                        shape.push((kid, f as u8));
                        let v: Value = a.get(e, PropEntry::VALUE);
                        match v.ref_offset() {
                            Some(p) if h.cell_kind(p) == CellKind::AccessorPair => vals.push((
                                a.get(p, AccessorPair::GETTER),
                                Some(a.get(p, AccessorPair::SETTER)),
                            )),
                            _ => vals.push((v, None)),
                        }
                    }
                }
                let n = self.shapes.len() as u32;
                let before = self.out.len();
                let sid = match self.shapes.get(&shape) {
                    Some(&s) => s,
                    None => {
                        self.shapes.insert(shape.clone(), n);
                        // a new shape is written inline once
                        varint(&mut self.out, shape.len() as u64);
                        for (k, f) in &shape {
                            varint(&mut self.out, *k & 0xffff_ffff);
                            self.out.push(*f);
                        }
                        n
                    }
                };
                self.shape_bytes += self.out.len() - before;
                varint(&mut self.out, sid as u64);
                for (v, s) in vals {
                    self.value(v);
                    if let Some(s) = s {
                        self.value(s);
                    }
                }
                let el: Ref = a.get(c, Object::ELEMENTS);
                match el.get() {
                    Some(el) => {
                        let vs = value_array(h, el);
                        varint(&mut self.out, vs.len() as u64);
                        for v in vs {
                            self.value(v);
                        }
                    }
                    None => self.out.push(0),
                }
                let nslots = (h.cell_size(c) - Object::LAYOUT.min_size) / 8;
                varint(&mut self.out, nslots as u64);
                for i in 0..nslots {
                    let v = a.read::<Value>(Object::SLOTS.at(c, i));
                    match (class, i) {
                        // the intrinsic by name, not by id
                        (ObjectClass::BuiltinClosure, 0) => {
                            let id = v.as_int().unwrap() as usize;
                            let s = self.str_id(INTRINSICS[id].name.to_string());
                            self.out.push(8);
                            varint(&mut self.out, s as u64);
                        }
                        // recomputable: hash indexes, compiled regexps
                        (
                            ObjectClass::Map
                            | ObjectClass::Set
                            | ObjectClass::WeakMap
                            | ObjectClass::WeakSet,
                            1,
                        )
                        | (ObjectClass::RegExp, 0) => self.out.push(0),
                        _ => self.value(v),
                    }
                }
                format!("{class:?}")
            }
            CellKind::ValueArray | CellKind::EphemeronArray => {
                let vs = value_array(h, c);
                varint(&mut self.out, vs.len() as u64);
                for v in vs {
                    self.value(v);
                }
                kind_name(kind)
            }
            CellKind::BoxCell => {
                self.value(a.get(c, BoxCell::VALUE));
                kind_name(kind)
            }
            CellKind::FuncTemplate => {
                // function identity: script and source span; the code is recompiled
                let s: Ref = a.get(c, FuncTemplate::SCRIPT);
                self.ref_field(s);
                varint(
                    &mut self.out,
                    a.get::<u32>(c, FuncTemplate::SRC_START) as u64,
                );
                varint(&mut self.out, a.get::<u32>(c, FuncTemplate::SRC_END) as u64);
                kind_name(kind)
            }
            CellKind::Script => {
                let n: Ref = a.get(c, Script::NAME);
                let s: Ref = a.get(c, Script::SOURCE);
                for r in [n, s] {
                    match r.get() {
                        Some(r) => self.value(Value::string(string_from_offset(r))),
                        None => self.out.push(1),
                    }
                }
                kind_name(kind)
            }
            CellKind::Symbol => {
                self.value(a.get(c, Symbol::DESCRIPTION));
                kind_name(kind)
            }
            CellKind::HostCall => {
                varint(&mut self.out, a.get::<u32>(c, HostCall::ID) as u64);
                varint(&mut self.out, a.get::<u32>(c, HostCall::FN) as u64);
                self.ref_field(a.get(c, HostCall::ARGS));
                self.value(a.get(c, HostCall::THIS));
                self.value(a.get(c, HostCall::PROMISE));
                kind_name(kind)
            }
            CellKind::Realm => {
                let n = (h.cell_size(c) - Realm::LAYOUT.min_size) / 8;
                for i in 0..n {
                    self.value(a.read::<Value>(Realm::INTRINSICS.at(c, i)));
                }
                kind_name(kind)
            }
            _ => {
                // anything else: the raw cell (should be rare)
                let size = h.cell_size(c);
                varint(&mut self.out, size as u64);
                self.out.extend_from_slice(a.slice(c, size));
                kind_name(kind)
            }
        }
    }
}

/// Encodes the image `h`, relative to `known` (or to nothing).
pub fn encode(h: &Heap, known: Option<Known<'_>>) -> Stats {
    let mut e = Enc {
        h,
        known,
        out: vec![],
        strings: HashMap::new(),
        str_bytes: vec![],
        nodes: HashMap::new(),
        queue: VecDeque::new(),
        visited: HashSet::new(),
        shapes: HashMap::new(),
        shape_bytes: 0,
        known_refs: HashSet::new(),
    };
    let a = h.arena();
    // Roots: the handles, the realm, the interpreter's queues and tables. The atom table,
    // the (empty) value stack and the handle table itself are rebuilt on restore.
    let hs = handles(h);
    varint(&mut e.out, hs.len() as u64);
    for v in hs {
        e.value(v);
    }
    e.cell_ref(h.realm().as_internal().unwrap());
    for f in [Roots::JOBS, Roots::HOST_CALLS] {
        let r: Ref = a.get(0, f);
        e.ref_field(r);
    }
    let i: Ref = a.get(0, Roots::INTERP);
    let i = i.get().unwrap();
    assert_eq!(a.get::<u32>(i, Interp::SP), 0, "not quiescent");
    for f in [Interp::RESULT, Interp::THROW, Interp::FUEL_DEBT] {
        e.value(a.get(i, f));
    }
    for f in [Interp::REJECTIONS, Interp::INBOX] {
        e.ref_field(a.get(i, f));
    }
    for f in [
        Interp::STATE,
        Interp::NEXT_CALL_ID,
        Interp::OUTCOME,
        Interp::JOB_HEAD,
        Interp::REPORTED,
        Interp::INBOX_HEAD,
    ] {
        varint(&mut e.out, a.get::<u32>(i, f) as u64);
    }
    let mut stats = Stats::default();
    // Walk every unit reachable; emit the new and the changed ones.
    while let Some(c) = e.queue.pop_front() {
        let kind = h.cell_kind(c);
        if matches!(
            kind,
            CellKind::Str8
                | CellKind::Str16
                | CellKind::Rope
                | CellKind::BigInt
                | CellKind::U32Array
                | CellKind::Bytes
                | CellKind::RefArray
        ) {
            continue;
        }
        let changed = e.changed(c);
        let known = e.is_known(c);
        if changed {
            let start = e.out.len();
            let label = e.record(c);
            let n = e.out.len() - start;
            let map = if known {
                &mut stats.patches
            } else {
                &mut stats.records
            };
            let s = map.entry(label).or_insert((0, 0));
            s.0 += 1;
            s.1 += n;
            if known {
                stats.patch_units += 1;
            } else {
                stats.new_units += 1;
            }
        } else {
            // unchanged and known: nothing to write, but changed units may hang below it
            walk_children(&mut e, c);
        }
    }
    stats.strings = e.str_bytes.len();
    stats.string_count = e.strings.len();
    stats.shapes = e.shape_bytes;
    if let Some(k) = &e.known {
        let (n, b) = k.paths.trie_bytes(&e.known_refs);
        stats.path_count = n as usize;
        stats.paths = b as usize;
    }
    stats.total = e.out.len() + e.str_bytes.len() + stats.paths;
    let mut bytes = e.str_bytes.clone();
    bytes.extend_from_slice(&e.out);
    stats.bytes = bytes;
    stats
}

/// Schedules what an unchanged unit references (without writing anything).
fn walk_children(e: &mut Enc<'_>, c: u32) {
    let h = e.h;
    let mut cells = vec![c];
    if h.cell_kind(c) == CellKind::Object {
        cells.extend(e.storage(c));
    }
    if h.cell_kind(c) == CellKind::FuncTemplate {
        let s: Ref = h.arena().get(c, FuncTemplate::SCRIPT);
        if let Some(s) = s.get() {
            e.schedule(s);
        }
        return;
    }
    for x in cells {
        for t in h.cell_refs(x) {
            let k = h.cell_kind(t);
            if matches!(
                k,
                CellKind::AtomTable | CellKind::PropTable | CellKind::AccessorPair
            ) {
                continue;
            }
            if x != c
                && matches!(k, CellKind::ValueArray | CellKind::U32Array)
                && h.cell_kind(c) == CellKind::Object
            {
                // the elements array or the table's index: part of the object
                let el: Ref = h.arena().get(c, Object::ELEMENTS);
                if el.get() == Some(t) {
                    continue;
                }
            }
            e.schedule(t);
        }
    }
}
