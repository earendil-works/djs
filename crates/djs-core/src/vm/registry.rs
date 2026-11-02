//! The builtin registry (D2371, issue 0145): stable ids for the builtin objects, functions
//! and symbols of the realm and for the intrinsics, keyed by a spec-style path.
//!
//! The registry is a committed, append-only table (tests/builtins/registry.tsv): an id is
//! never renumbered or reused, and a builtin that goes away keeps its line as a tombstone.
//! It names builtins across djs builds, where offsets, intrinsic ids and the base image's
//! layout change: snapshot exports (docs/goldilocks-snapshots.md, tier 2) and tools such as
//! `djs heap` name a builtin by its id or path.
//!
//! **Paths.** A *value* entry is a path that leads from a root to an object or symbol of the
//! base realm, the way the spec writes intrinsics without the `%`s:
//! - roots: `globalThis`; a global's name (`Array`); a realm intrinsic that no global reaches
//!   (`%ArrayIteratorPrototype%`, `%TypedArray%`); a prelude function registered for an
//!   intrinsic (`%intrinsic:PromiseResolve%`);
//! - segments: `.key` for identifier keys, `["key"]` for others, `[Symbol.iterator]` for
//!   well-known symbol keys, `.[[Prototype]]`;
//! - accessor functions are prefixed `get ` or `set ` (`get Map.prototype.size`);
//! - `~N` is the value of a closure's capture N: the prelude's internal functions that
//!   builtins capture and no property reaches (`Array.prototype.map~2`). These paths change
//!   when the builtin's code changes; the old entry is then tombstoned and a new one added.
//!
//! An *intrinsic* entry names an entry of the intrinsics table (bytecode/intrinsics.rs) by
//! its name: the kinds of builtin closures (`PromiseResolveFunction`-like records hold an
//! intrinsic id) and the operands of `call_intrinsic`.
//!
//! Several registered paths may lead to the same object (a new alias); the object's id is
//! the lowest. New builtins are appended under their canonical path: the first one a
//! breadth-first walk from the roots reaches, with keys in sorted order.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::bytecode::intrinsics::INTRINSICS;
use crate::heap::cells::BoxCell;
use crate::heap::{Atom, CellKind, Heap, ObjectClass, Property};
use crate::interp::realm::{Intrinsic, installed_intrinsic, intrinsic};
use crate::value::{Obj, Value};

/// What an entry names.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum EntryKind {
    /// An object or symbol of the base realm, by path.
    Value,
    /// An entry of the intrinsics table, by name.
    Intrinsic,
}

impl EntryKind {
    fn as_str(self) -> &'static str {
        match self {
            EntryKind::Value => "value",
            EntryKind::Intrinsic => "intrinsic",
        }
    }
}

/// One line of the registry.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub id: u32,
    pub kind: EntryKind,
    pub path: String,
    /// A tombstone: why the builtin went away. Its id is never used again.
    pub removed: Option<String>,
}

/// The registry.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Registry {
    /// In id order: `entries[i].id == i`.
    pub entries: Vec<Entry>,
}

/// What [`Registry::update`] changed.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Update {
    pub added: Vec<u32>,
    pub removed: Vec<u32>,
}

/// The header of the registry file.
const HEADER: &str = "\
# The builtin registry (D2371): stable ids for builtins, append-only. Never edit by hand:
# `scripts/registry-update.sh` appends new builtins and tombstones removed ones. Columns:
# id, kind (value: an object or symbol by path; intrinsic: by name), path, and for a
# tombstone why it was removed. An id is never renumbered or reused (the gate checks it).
";

impl Registry {
    /// Parses the registry file. Ids must be 0, 1, 2, ... in order, and no two live entries
    /// of a kind may share a path.
    pub fn parse(text: &str) -> Result<Registry, String> {
        let mut entries = Vec::new();
        let mut live = BTreeSet::new();
        for (n, line) in text.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if !(3..=4).contains(&cols.len()) {
                return Err(format!("line {}: expected 3 or 4 columns", n + 1));
            }
            let id: u32 = cols[0]
                .parse()
                .map_err(|_| format!("line {}: bad id {:?}", n + 1, cols[0]))?;
            if id as usize != entries.len() {
                return Err(format!(
                    "line {}: id {id} out of sequence (expected {})",
                    n + 1,
                    entries.len()
                ));
            }
            let kind = match cols[1] {
                "value" => EntryKind::Value,
                "intrinsic" => EntryKind::Intrinsic,
                k => return Err(format!("line {}: unknown kind {k:?}", n + 1)),
            };
            let removed = match cols.get(3) {
                None => None,
                Some(r) => Some(
                    r.strip_prefix("removed: ")
                        .ok_or_else(|| format!("line {}: expected `removed: ...`", n + 1))?
                        .to_string(),
                ),
            };
            if removed.is_none() && !live.insert((kind, cols[2].to_string())) {
                return Err(format!("line {}: {} registered twice", n + 1, cols[2]));
            }
            entries.push(Entry {
                id,
                kind,
                path: cols[2].to_string(),
                removed,
            });
        }
        Ok(Registry { entries })
    }

    /// The file text.
    pub fn to_text(&self) -> String {
        let mut out = String::from(HEADER);
        for e in &self.entries {
            out.push_str(&format!("{}\t{}\t{}", e.id, e.kind.as_str(), e.path));
            if let Some(r) = &e.removed {
                out.push_str("\tremoved: ");
                out.push_str(r);
            }
            out.push('\n');
        }
        out
    }

    /// The live entry with this kind and path.
    pub fn id_of(&self, kind: EntryKind, path: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.kind == kind && e.path == path && e.removed.is_none())
            .map(|e| e.id)
    }

    pub fn entry(&self, id: u32) -> Option<&Entry> {
        self.entries.get(id as usize)
    }

    /// Registry ids of the intrinsics table, by intrinsic id (`None`: unregistered).
    pub fn intrinsic_ids(&self) -> Vec<Option<u32>> {
        INTRINSICS
            .iter()
            .map(|i| self.id_of(EntryKind::Intrinsic, i.name))
            .collect()
    }

    /// The ids of the builtin objects and symbols of `heap`, by cell offset: every live
    /// value entry resolved on this heap, the lowest id for an object with several paths.
    pub fn name_cells(&self, heap: &Heap) -> BTreeMap<u32, u32> {
        let mut r = Resolver::new(heap);
        let mut out = BTreeMap::new();
        for e in &self.entries {
            if e.kind == EntryKind::Value
                && e.removed.is_none()
                && let Some(cell) = r.resolve(&e.path).and_then(cell_of)
            {
                out.entry(cell).or_insert(e.id);
            }
        }
        out
    }

    /// What is wrong with the registry on this realm: live entries that no longer resolve,
    /// builtins without an entry, intrinsics without an entry. Empty when it is current.
    pub fn check(&self, heap: &Heap) -> Vec<String> {
        let mut problems = Vec::new();
        let mut r = Resolver::new(heap);
        for e in self.entries.iter().filter(|e| e.removed.is_none()) {
            let ok = match e.kind {
                EntryKind::Value => r.resolve(&e.path).and_then(cell_of).is_some(),
                EntryKind::Intrinsic => INTRINSICS.iter().any(|i| i.name == e.path),
            };
            if !ok {
                problems.push(format!(
                    "{} {} (id {}) is gone",
                    e.kind.as_str(),
                    e.path,
                    e.id
                ));
            }
        }
        let named = self.name_cells(heap);
        for (cell, path) in canonical_names(heap) {
            if !named.contains_key(&cell) {
                problems.push(format!("value {path} has no entry"));
            }
        }
        for i in INTRINSICS {
            if self.id_of(EntryKind::Intrinsic, i.name).is_none() {
                problems.push(format!("intrinsic {} has no entry", i.name));
            }
        }
        problems
    }

    /// Brings the registry up to date with this realm: tombstones live entries that no
    /// longer resolve (with `reason`), then appends the builtins and intrinsics that have
    /// none. Never renumbers.
    pub fn update(&mut self, heap: &Heap, reason: &str) -> Update {
        let mut u = Update::default();
        let mut r = Resolver::new(heap);
        for e in self.entries.iter_mut().filter(|e| e.removed.is_none()) {
            let ok = match e.kind {
                EntryKind::Value => r.resolve(&e.path).and_then(cell_of).is_some(),
                EntryKind::Intrinsic => INTRINSICS.iter().any(|i| i.name == e.path),
            };
            if !ok {
                e.removed = Some(reason.to_string());
                u.removed.push(e.id);
            }
        }
        let named = self.name_cells(heap);
        for (cell, path) in canonical_names(heap) {
            if !named.contains_key(&cell) {
                u.added.push(self.push(EntryKind::Value, path));
            }
        }
        for i in INTRINSICS {
            if self.id_of(EntryKind::Intrinsic, i.name).is_none() {
                u.added
                    .push(self.push(EntryKind::Intrinsic, i.name.to_string()));
            }
        }
        u
    }

    fn push(&mut self, kind: EntryKind, path: String) -> u32 {
        let id = self.entries.len() as u32;
        self.entries.push(Entry {
            id,
            kind,
            path,
            removed: None,
        });
        id
    }
}

/// The heap cell a builtin value is: an object's or a symbol's offset.
fn cell_of(v: Value) -> Option<u32> {
    v.as_object()
        .map(Obj::offset)
        .or_else(|| v.as_symbol().map(|s| s.offset()))
}

/// The realm intrinsics that serve as roots, with their names: the ones that hold builtin
/// objects. `Proto` reads `Prototype`, as in the spec.
pub const REALM_ROOTS: &[(Intrinsic, &str)] = &[
    (Intrinsic::ObjectProto, "ObjectPrototype"),
    (Intrinsic::FunctionProto, "FunctionPrototype"),
    (Intrinsic::ArrayProto, "ArrayPrototype"),
    (Intrinsic::ErrorProto, "ErrorPrototype"),
    (Intrinsic::TypeErrorProto, "TypeErrorPrototype"),
    (Intrinsic::RangeErrorProto, "RangeErrorPrototype"),
    (Intrinsic::ReferenceErrorProto, "ReferenceErrorPrototype"),
    (Intrinsic::SyntaxErrorProto, "SyntaxErrorPrototype"),
    (Intrinsic::InternalErrorProto, "InternalErrorPrototype"),
    (Intrinsic::OutOfMemoryError, "OutOfMemoryError"),
    (Intrinsic::EvalErrorProto, "EvalErrorPrototype"),
    (Intrinsic::UriErrorProto, "URIErrorPrototype"),
    (Intrinsic::AggregateErrorProto, "AggregateErrorPrototype"),
    (Intrinsic::BooleanProto, "BooleanPrototype"),
    (Intrinsic::NumberProto, "NumberPrototype"),
    (Intrinsic::StringProto, "StringPrototype"),
    (Intrinsic::SymbolProto, "SymbolPrototype"),
    (Intrinsic::SymbolRegistry, "SymbolRegistry"),
    (Intrinsic::IteratorProto, "IteratorPrototype"),
    (Intrinsic::ArrayIteratorProto, "ArrayIteratorPrototype"),
    (Intrinsic::StringIteratorProto, "StringIteratorPrototype"),
    (Intrinsic::MapProto, "MapPrototype"),
    (Intrinsic::SetProto, "SetPrototype"),
    (Intrinsic::WeakMapProto, "WeakMapPrototype"),
    (Intrinsic::WeakSetProto, "WeakSetPrototype"),
    (Intrinsic::MapIteratorProto, "MapIteratorPrototype"),
    (Intrinsic::SetIteratorProto, "SetIteratorPrototype"),
    (Intrinsic::PromiseProto, "PromisePrototype"),
    (Intrinsic::DateProto, "DatePrototype"),
    (Intrinsic::GeneratorProto, "GeneratorPrototype"),
    (Intrinsic::ArrayBufferProto, "ArrayBufferPrototype"),
    (Intrinsic::TypedArrayCtor, "TypedArray"),
    (Intrinsic::TypedArrayProto, "TypedArrayPrototype"),
    (Intrinsic::Int8ArrayProto, "Int8ArrayPrototype"),
    (Intrinsic::Uint8ArrayProto, "Uint8ArrayPrototype"),
    (
        Intrinsic::Uint8ClampedArrayProto,
        "Uint8ClampedArrayPrototype",
    ),
    (Intrinsic::Int16ArrayProto, "Int16ArrayPrototype"),
    (Intrinsic::Uint16ArrayProto, "Uint16ArrayPrototype"),
    (Intrinsic::Int32ArrayProto, "Int32ArrayPrototype"),
    (Intrinsic::Uint32ArrayProto, "Uint32ArrayPrototype"),
    (Intrinsic::Float16ArrayProto, "Float16ArrayPrototype"),
    (Intrinsic::Float32ArrayProto, "Float32ArrayPrototype"),
    (Intrinsic::Float64ArrayProto, "Float64ArrayPrototype"),
    (Intrinsic::BigInt64ArrayProto, "BigInt64ArrayPrototype"),
    (Intrinsic::BigUint64ArrayProto, "BigUint64ArrayPrototype"),
    (Intrinsic::DataViewProto, "DataViewPrototype"),
    (
        Intrinsic::GeneratorFunctionProto,
        "GeneratorFunctionPrototype",
    ),
    (Intrinsic::AsyncFunctionProto, "AsyncFunctionPrototype"),
    (
        Intrinsic::AsyncGeneratorFunctionProto,
        "AsyncGeneratorFunctionPrototype",
    ),
    (Intrinsic::AsyncIteratorProto, "AsyncIteratorPrototype"),
    (Intrinsic::AsyncGeneratorProto, "AsyncGeneratorPrototype"),
    (
        Intrinsic::AsyncFromSyncIteratorProto,
        "AsyncFromSyncIteratorPrototype",
    ),
    (Intrinsic::BigIntProto, "BigIntPrototype"),
    (Intrinsic::RegExpProto, "RegExpPrototype"),
    (
        Intrinsic::RegExpStringIteratorProto,
        "RegExpStringIteratorPrototype",
    ),
    (Intrinsic::IteratorHelperProto, "IteratorHelperPrototype"),
    (
        Intrinsic::WrapForValidIteratorProto,
        "WrapForValidIteratorPrototype",
    ),
    (Intrinsic::DisposableStackProto, "DisposableStackPrototype"),
    (
        Intrinsic::AsyncDisposableStackProto,
        "AsyncDisposableStackPrototype",
    ),
    (Intrinsic::SuppressedErrorProto, "SuppressedErrorPrototype"),
    (Intrinsic::WeakRefProto, "WeakRefPrototype"),
    (
        Intrinsic::FinalizationRegistryProto,
        "FinalizationRegistryPrototype",
    ),
    (Intrinsic::CallSiteProto, "CallSitePrototype"),
];

/// Every builtin object and symbol a walk from the roots reaches, each with its canonical
/// path (the first path breadth-first, keys in sorted order), in walk order.
pub fn canonical_names(heap: &Heap) -> Vec<(u32, String)> {
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut out: Vec<(Value, String)> = Vec::new();
    let global = intrinsic(heap, Intrinsic::Global);
    if let Some(c) = cell_of(global) {
        seen.insert(c);
        out.push((global, String::from("globalThis")));
    }
    // Passes, so that a builtin gets the most readable path it has: what the globals
    // reach by properties (breadth-first), then the realm roots (`%TypedArray%` rather than
    // `Int8Array.[[Prototype]]`), then the prelude functions registered for intrinsics,
    // and last what only `[[Prototype]]` links reach from any of them.
    let mut globals = Vec::new();
    if let Some(g) = global.as_object() {
        for (seg, v, _) in edges(heap, g, false) {
            // A global's path is its name; keys that aren't identifiers keep `globalThis`.
            let p = child_path("globalThis", &seg);
            let p = match p.strip_prefix("globalThis.") {
                Some(name) => name.to_string(),
                None => p,
            };
            globals.push((v, p));
        }
    }
    walk(heap, globals, false, &mut seen, &mut out);
    let realm = REALM_ROOTS
        .iter()
        .map(|(i, name)| (intrinsic(heap, *i), format!("%{name}%")))
        .collect();
    walk(heap, realm, false, &mut seen, &mut out);
    let installed = INTRINSICS
        .iter()
        .enumerate()
        .map(|(id, info)| {
            (
                installed_intrinsic(heap, id as u16),
                format!("%intrinsic:{}%", info.name),
            )
        })
        .collect();
    walk(heap, installed, false, &mut seen, &mut out);
    let named = out.clone();
    walk(heap, named, true, &mut seen, &mut out);
    // Last, the prelude's internal functions that builtins capture (`~N`: the value of
    // capture N), which no property reaches.
    let named = out.clone();
    walk_captures(heap, named, &mut seen, &mut out);
    out.into_iter()
        .filter_map(|(v, p)| Some((cell_of(v)?, p)))
        .collect()
}

/// Breadth-first from `roots`, adding the cells not `seen` yet to `out` with their paths.
/// A root that was seen before is still walked from (the last pass starts from everything
/// named so far).
fn walk(
    heap: &Heap,
    roots: Vec<(Value, String)>,
    proto: bool,
    seen: &mut BTreeSet<u32>,
    out: &mut Vec<(Value, String)>,
) {
    let mut queue: VecDeque<(Value, String, bool, bool)> =
        roots.into_iter().map(|(v, p)| (v, p, true, true)).collect();
    while let Some((v, path, expand, root)) = queue.pop_front() {
        let Some(c) = cell_of(v) else { continue };
        if seen.insert(c) {
            out.push((v, path.clone()));
        } else if !root {
            continue;
        }
        let Some(o) = v.as_object().filter(|_| expand) else {
            continue;
        };
        for (seg, child, child_expand) in edges(heap, o, proto) {
            queue.push_back((child, child_path(&path, &seg), child_expand, false));
        }
    }
}

/// Breadth-first over captures (and the properties of what they lead to).
fn walk_captures(
    heap: &Heap,
    roots: Vec<(Value, String)>,
    seen: &mut BTreeSet<u32>,
    out: &mut Vec<(Value, String)>,
) {
    let mut queue: VecDeque<(Value, String, bool)> =
        roots.into_iter().map(|(v, p)| (v, p, true)).collect();
    while let Some((v, path, root)) = queue.pop_front() {
        let Some(c) = cell_of(v) else { continue };
        if seen.insert(c) {
            out.push((v, path.clone()));
        } else if !root {
            continue;
        }
        let Some(o) = v.as_object() else { continue };
        for (i, child) in captures(heap, o) {
            queue.push_back((child, format!("{path}~{i}"), false));
        }
        // An accessor's path names the accessor function; captures may follow it, other
        // segments may not.
        if path.starts_with("get ") || path.starts_with("set ") {
            continue;
        }
        for (seg, child, expand) in edges(heap, o, true) {
            if expand {
                queue.push_back((child, child_path(&path, &seg), false));
            }
        }
    }
}

/// The captured values of a closure: (capture index, value).
fn captures(heap: &Heap, o: Obj) -> Vec<(u32, Value)> {
    if heap.object_class(o) != ObjectClass::Function {
        return Vec::new();
    }
    (1..heap.object_slot_count(o))
        .filter_map(|i| {
            let b = heap.object_slot(o, i).as_internal()?;
            (heap.cell_kind(b) == CellKind::BoxCell)
                .then(|| (i - 1, heap.arena().get(b, BoxCell::VALUE)))
        })
        .collect()
}

/// `path` followed by `seg`; an accessor segment's `get `/`set ` goes to the front.
fn child_path(path: &str, seg: &str) -> String {
    match seg
        .strip_prefix("get ")
        .or_else(|| seg.strip_prefix("set "))
    {
        Some(rest) => format!("{} {path}{rest}", &seg[..3]),
        None => format!("{path}{seg}"),
    }
}

/// The outgoing edges of `o` that lead to heap values: (segment, value, whether to walk on
/// from it). Accessor segments carry their `get `/`set ` prefix; their functions are not
/// walked (a path can only name them as a whole).
fn edges(heap: &Heap, o: Obj, proto: bool) -> Vec<(String, Value, bool)> {
    let mut out: Vec<(String, Value, bool)> = Vec::new();
    for k in heap.own_keys(o) {
        let Some(seg) = segment(heap, k) else {
            continue;
        };
        match heap.get_own_property(o, k) {
            Some(Property::Data { value, .. }) => out.push((seg, value, true)),
            Some(Property::Accessor { getter, setter, .. }) => {
                out.push((format!("get {seg}"), getter, false));
                out.push((format!("set {seg}"), setter, false));
            }
            None => {}
        }
    }
    out.retain(|(_, v, _)| cell_of(*v).is_some());
    out.sort_by(|a, b| a.0.cmp(&b.0));
    if proto && let Some(p) = heap.get_prototype_of(o) {
        out.push((String::from(".[[Prototype]]"), Value::object(p), true));
    }
    out
}

/// A key as a path segment: `.name`, `["odd key"]`, `[Symbol.iterator]`; `None` for keys
/// a path cannot name (symbols without a `Symbol.` description).
fn segment(heap: &Heap, k: Atom) -> Option<String> {
    if let Some(s) = heap.atom_symbol(k) {
        let d = heap.symbol_description(s).as_string()?;
        let d = heap.str_to_string(d);
        return d.starts_with("Symbol.").then(|| format!("[{d}]"));
    }
    let name = heap.atom_name(k);
    let ident = !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    Some(if ident {
        format!(".{name}")
    } else {
        format!("[{}]", quote(&name))
    })
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Resolves paths on one heap, caching each object's segments.
struct Resolver<'h> {
    heap: &'h Heap,
    roots: BTreeMap<String, Value>,
    cache: BTreeMap<u32, Vec<(String, Value)>>,
}

impl<'h> Resolver<'h> {
    fn new(heap: &'h Heap) -> Resolver<'h> {
        let mut roots = BTreeMap::new();
        let global = intrinsic(heap, Intrinsic::Global);
        roots.insert(String::from("globalThis"), global);
        for (i, name) in REALM_ROOTS {
            roots.insert(format!("%{name}%"), intrinsic(heap, *i));
        }
        for (id, info) in INTRINSICS.iter().enumerate() {
            let v = installed_intrinsic(heap, id as u16);
            if !v.is_undefined() {
                roots.insert(format!("%intrinsic:{}%", info.name), v);
            }
        }
        Resolver {
            heap,
            roots,
            cache: BTreeMap::new(),
        }
    }

    fn segments(&mut self, o: Obj) -> &Vec<(String, Value)> {
        let heap = self.heap;
        self.cache.entry(o.offset()).or_insert_with(|| {
            let mut out = Vec::new();
            for k in heap.own_keys(o) {
                let Some(seg) = segment(heap, k) else {
                    continue;
                };
                match heap.get_own_property(o, k) {
                    Some(Property::Data { value, .. }) => out.push((seg, value)),
                    Some(Property::Accessor { getter, setter, .. }) => {
                        out.push((format!("get {seg}"), getter));
                        out.push((format!("set {seg}"), setter));
                    }
                    None => {}
                }
            }
            out
        })
    }

    /// The value at `path`, or `None` if a step is missing.
    fn resolve(&mut self, path: &str) -> Option<Value> {
        let (accessor, path) = match path.get(..4) {
            Some("get ") | Some("set ") => (Some(&path[..3]), &path[4..]),
            _ => (None, path),
        };
        let (root, rest) = split_root(path);
        let mut segs = Vec::new();
        let mut rest_iter = rest;
        while !rest_iter.is_empty() {
            let (seg, tail) = split_segment(rest_iter)?;
            segs.push(seg);
            rest_iter = tail;
        }
        // The accessor prefix belongs to the last property segment (captures may follow).
        let accessor_at = segs
            .iter()
            .rposition(|s| !s.starts_with('~') && *s != ".[[Prototype]]");
        let mut cur = match self.roots.get(root) {
            Some(v) => *v,
            None => {
                let g = intrinsic(self.heap, Intrinsic::Global).as_object()?;
                let seg = format!(".{root}");
                self.segments(g).iter().find(|s| s.0 == seg)?.1
            }
        };
        for (n, seg) in segs.into_iter().enumerate() {
            let o = cur.as_object()?;
            if let Some(n) = seg.strip_prefix('~') {
                let n: u32 = n.parse().ok()?;
                cur = captures(self.heap, o).into_iter().find(|c| c.0 == n)?.1;
                continue;
            }
            if seg == ".[[Prototype]]" {
                cur = Value::object(self.heap.get_prototype_of(o)?);
                continue;
            }
            let want = match accessor {
                Some(a) if accessor_at == Some(n) => format!("{a} {seg}"),
                _ => seg.to_string(),
            };
            cur = self.segments(o).iter().find(|s| s.0 == want)?.1;
        }
        Some(cur)
    }
}

/// `"%Foo%.bar"` -> (`"%Foo%"`, `".bar"`); `"Array.prototype"` -> (`"Array"`, `".prototype"`).
fn split_root(path: &str) -> (&str, &str) {
    if let Some(rest) = path.strip_prefix('%')
        && let Some(end) = rest.find('%')
    {
        return path.split_at(end + 2);
    }
    let end = path.find(['.', '[', '~']).unwrap_or(path.len());
    path.split_at(end)
}

/// The first segment of `rest` and what follows it.
fn split_segment(rest: &str) -> Option<(&str, &str)> {
    if rest.starts_with(".[[Prototype]]") {
        return Some(rest.split_at(".[[Prototype]]".len()));
    }
    if let Some(r) = rest.strip_prefix('.') {
        let end = r.find(['.', '[', '~']).unwrap_or(r.len());
        return Some(rest.split_at(end + 1));
    }
    if rest.starts_with("[\"") {
        // A quoted key: up to the closing `"]`, skipping escapes.
        let b = rest.as_bytes();
        let mut i = 2;
        while i < b.len() {
            match b[i] {
                b'\\' => i += 2,
                b'"' if b.get(i + 1) == Some(&b']') => return Some(rest.split_at(i + 2)),
                _ => i += 1,
            }
        }
        return None;
    }
    if rest.starts_with('[') {
        let end = rest.find(']')?;
        return Some(rest.split_at(end + 1));
    }
    if let Some(r) = rest.strip_prefix('~') {
        let end = r.find(['.', '[', '~']).unwrap_or(r.len());
        return Some(rest.split_at(end + 1));
    }
    None
}

#[cfg(test)]
mod tests;
