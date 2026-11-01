//! The object graph of an image, built with djs-core's own walker (`Heap::cells`,
//! `Heap::trace_cell`: the layout table decides which fields are references) and named
//! with its layout table (`CellKind::layout`): liveness, the dominator tree (retained
//! sizes) and a readable path from the roots to every live cell.

use std::collections::VecDeque;

use djs_core::heap::cells::{
    AccessorPair, EphemeronArray, FuncTemplate, HandleTable, Interp, Object, PropEntry, PropTable,
    PropValues, Realm, Roots, Script, Shape, U32Array, ValueArray,
};
use djs_core::heap::layout::{CellLayout, Ref};
use djs_core::heap::walk::{CellInfo, Slot};
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass};
use djs_core::value::{Str, Value, tag};

pub const NONE: u32 = u32::MAX;

pub struct Graph<'h> {
    pub heap: &'h Heap,
    /// Every cell, in address order.
    pub cells: Vec<CellInfo>,
    /// Successors along strong references (and ephemeron values whose key is live).
    pub succ: Vec<Vec<u32>>,
    /// Reachable from the roots, as a collection would find it.
    pub live: Vec<bool>,
    /// Immediate dominator (cell index), `NONE` for dead cells; the roots cell is its own.
    pub idom: Vec<u32>,
    /// Bytes a cell keeps alive alone: itself and everything it dominates.
    pub retained: Vec<u64>,
    /// Reverse postorder of the live cells (dominators first).
    pub rpo: Vec<u32>,
    /// Breadth-first path tree: the parent and the label of the edge from it.
    parent: Vec<(u32, String)>,
}

/// A string reference to the string cell at `off` (`Str` has no public constructor).
pub fn str_at(off: u32) -> Str {
    let bits = djs_core::value::TAG_PREFIX | (tag::STRING as u64) << 32 | off as u64;
    Value::from_bits(bits)
        .and_then(Value::as_string)
        .expect("a string offset")
}

/// Whether the optional field at `offset` of a cell of `size` bytes exists (templates of
/// privileged code are shorter, `min 56` in the layout).
pub fn has_field(size: u32, offset: u32) -> bool {
    offset < size
}

impl<'h> Graph<'h> {
    pub fn new(heap: &'h Heap) -> Graph<'h> {
        let cells: Vec<CellInfo> = heap.cells().collect();
        let mut g = Graph {
            heap,
            succ: vec![Vec::new(); cells.len()],
            live: vec![false; cells.len()],
            idom: vec![NONE; cells.len()],
            retained: vec![0; cells.len()],
            rpo: Vec::new(),
            parent: vec![(NONE, String::new()); cells.len()],
            cells,
        };
        let pairs = g.strong_edges();
        g.mark(&pairs);
        g.dominators();
        g.paths();
        g
    }

    pub fn index(&self, off: u32) -> Option<u32> {
        self.cells
            .binary_search_by_key(&off, |c| c.offset)
            .ok()
            .map(|i| i as u32)
    }

    pub fn cell(&self, i: u32) -> CellInfo {
        self.cells[i as usize]
    }

    fn target(&self, slot: Slot) -> Option<u32> {
        self.heap.slot_target(slot).and_then(|t| self.index(t))
    }

    /// The value stack and the slots in use (a collection clears the rest, as
    /// `Vm::collect` does).
    pub fn stack(&self) -> Option<(u32, u32)> {
        let a = self.heap.arena();
        let s: Ref = a.get(0, Roots::STACK);
        let i: Ref = a.get(0, Roots::INTERP);
        Some((s.get()?, a.get(i.get()?, Interp::SP)))
    }

    /// Fills `succ` with the strong edges; returns the ephemeron pairs (table, key,
    /// value) to resolve against liveness.
    fn strong_edges(&mut self) -> Vec<(u32, Option<u32>, Option<u32>)> {
        let stack = self.stack();
        let mut pairs = Vec::new();
        for i in 0..self.cells.len() {
            let c = self.cells[i];
            if c.kind == CellKind::EphemeronArray {
                // Not traced: a pair lives only while its key does (D1382).
                let a = self.heap.arena();
                let n: u32 = a.get(c.offset, EphemeronArray::LEN) / 2;
                for p in 0..n {
                    let k = Slot::Value(EphemeronArray::VALUES.at(c.offset, 2 * p));
                    let v = Slot::Value(EphemeronArray::VALUES.at(c.offset, 2 * p + 1));
                    pairs.push((i as u32, self.target(k), self.target(v)));
                }
                continue;
            }
            let limit = match stack {
                Some((s, sp)) if s == c.offset => Some(ValueArray::VALUES.at(s, sp)),
                _ => None,
            };
            let mut out = Vec::new();
            self.heap.trace_cell(c.offset, |slot| {
                if let (Some(l), Slot::Value(at)) = (limit, slot)
                    && at >= l
                {
                    return;
                }
                if let Some(t) = self.target(slot) {
                    out.push(t);
                }
            });
            self.succ[i] = out;
        }
        pairs
    }

    fn mark(&mut self, pairs: &[(u32, Option<u32>, Option<u32>)]) {
        let mut work = vec![0u32];
        self.live[0] = true;
        let mut done = vec![false; pairs.len()];
        loop {
            while let Some(i) = work.pop() {
                for &t in &self.succ[i as usize] {
                    if !self.live[t as usize] {
                        self.live[t as usize] = true;
                        work.push(t);
                    }
                }
            }
            let mut progress = false;
            for (p, &(table, key, value)) in pairs.iter().enumerate() {
                if done[p]
                    || !self.live[table as usize]
                    || !key.is_some_and(|k| self.live[k as usize])
                {
                    continue;
                }
                done[p] = true;
                if let Some(v) = value {
                    self.succ[table as usize].push(v);
                    if !self.live[v as usize] {
                        self.live[v as usize] = true;
                        work.push(v);
                        progress = true;
                    }
                }
            }
            if !progress {
                break;
            }
        }
    }

    /// Cooper, Harvey and Kennedy's iterative dominator algorithm over the live graph.
    fn dominators(&mut self) {
        let n = self.cells.len();
        // Postorder by an iterative depth-first search.
        let mut po_num = vec![NONE; n];
        let mut order = Vec::new();
        let mut seen = vec![false; n];
        let mut stack: Vec<(u32, usize)> = vec![(0, 0)];
        seen[0] = true;
        while let Some(&mut (v, ref mut next)) = stack.last_mut() {
            if let Some(&w) = self.succ[v as usize].get(*next) {
                *next += 1;
                if !seen[w as usize] {
                    seen[w as usize] = true;
                    stack.push((w, 0));
                }
            } else {
                po_num[v as usize] = order.len() as u32;
                order.push(v);
                stack.pop();
            }
        }
        let rpo: Vec<u32> = order.iter().rev().copied().collect();
        let mut preds = vec![Vec::new(); n];
        for &v in &rpo {
            for &w in &self.succ[v as usize] {
                preds[w as usize].push(v);
            }
        }
        let mut idom = vec![NONE; n];
        idom[0] = 0;
        let intersect = |idom: &[u32], mut a: u32, mut b: u32| {
            while a != b {
                while po_num[a as usize] < po_num[b as usize] {
                    a = idom[a as usize];
                }
                while po_num[b as usize] < po_num[a as usize] {
                    b = idom[b as usize];
                }
            }
            a
        };
        loop {
            let mut changed = false;
            for &b in &rpo[1..] {
                let mut new = NONE;
                for &p in &preds[b as usize] {
                    if idom[p as usize] == NONE {
                        continue;
                    }
                    new = if new == NONE {
                        p
                    } else {
                        intersect(&idom, p, new)
                    };
                }
                if new != NONE && idom[b as usize] != new {
                    idom[b as usize] = new;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut retained: Vec<u64> = self.cells.iter().map(|c| c.size as u64).collect();
        for &b in rpo[1..].iter().rev() {
            let d = idom[b as usize];
            retained[d as usize] += retained[b as usize];
        }
        for (i, r) in retained.iter_mut().enumerate() {
            if idom[i] == NONE {
                *r = 0;
            }
        }
        self.idom = idom;
        self.retained = retained;
        self.rpo = rpo;
    }

    /// The breadth-first path tree from the names a reader knows: `globalThis`, the global
    /// lexical scope, the embedder's handles, then everything else from the roots cell.
    fn paths(&mut self) {
        let mut queue = VecDeque::new();
        let seed = |g: &mut Graph, i: Option<u32>, label: String, queue: &mut VecDeque<u32>| {
            if let Some(i) = i
                && g.parent[i as usize].0 == NONE
                && i != 0
            {
                g.parent[i as usize] = (i, label);
                queue.push_back(i);
            }
        };
        let heap = self.heap;
        let a = heap.arena();
        if let Some(realm) = heap.realm().as_internal() {
            let global = a.read::<Value>(Realm::INTRINSICS.at(realm, 0));
            seed(
                self,
                global.ref_offset().and_then(|o| self.index(o)),
                "globalThis".into(),
                &mut queue,
            );
            let lex: Ref = a.get(realm, Realm::GLOBAL_LEX);
            seed(
                self,
                lex.get().and_then(|o| self.index(o)),
                "<global let>".into(),
                &mut queue,
            );
        }
        let handles: Ref = a.get(0, Roots::HANDLES);
        if let Some(t) = handles.get() {
            let n: u32 = a.get(t, HandleTable::LEN);
            for h in 0..n {
                let v: Value = a.read(HandleTable::SLOTS.at(t, h));
                seed(
                    self,
                    v.ref_offset().and_then(|o| self.index(o)),
                    format!("handle[{}]", h + 1),
                    &mut queue,
                );
            }
        }
        self.parent[0] = (0, String::new());
        queue.push_back(0);
        while let Some(i) = queue.pop_front() {
            for (label, t) in self.labelled_edges(i) {
                if self.live[t as usize] && self.parent[t as usize].0 == NONE && t != 0 {
                    self.parent[t as usize] = (i, label);
                    queue.push_back(t);
                }
            }
        }
    }

    /// The path from the roots to cell `i`, e.g. `globalThis.cache[3].body`.
    pub fn path(&self, i: u32) -> String {
        let mut parts = Vec::new();
        let mut cur = i;
        for _ in 0..64 {
            let (p, label) = &self.parent[cur as usize];
            if *p == NONE {
                return if self.live[i as usize] {
                    "?".into()
                } else {
                    "(garbage)".into()
                };
            }
            parts.push(label.as_str());
            if *p == cur {
                break;
            }
            cur = *p;
        }
        if parts.len() == 64 {
            parts.push("…");
        }
        parts.reverse();
        let s = parts.concat();
        if s.is_empty() { "<roots>".into() } else { s }
    }

    // ---- Names -------------------------------------------------------------------------

    /// A property key as a path segment: `.name`, `["a b"]`, `[3]` or `[Symbol(x)]`.
    pub fn key_label(&self, key: u32) -> String {
        let atom = Atom::from_id(key);
        if let Some(n) = atom.as_int() {
            return format!("[{n}]");
        }
        let name = self.heap.atom_name(atom);
        if atom.is_symbol() {
            return format!(
                "[Symbol({})]",
                name.trim_start_matches('[').trim_end_matches(']')
            );
        }
        let ident = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            && !name.starts_with(|c: char| c.is_ascii_digit());
        if ident {
            format!(".{name}")
        } else {
            format!("[{name:?}]")
        }
    }

    /// The `i`th key of a shape (its descriptor array is `[used, key0, flags0, ...]`).
    fn shape_key(&self, shape: u32, i: u32) -> Option<u32> {
        let a = self.heap.arena();
        let descs: Ref = a.get(shape, Shape::DESCS);
        let d = descs.get()?;
        let n: u32 = a.get(d, U32Array::LEN);
        (1 + 2 * i < n).then(|| a.read::<u32>(U32Array::DATA.at(d, 1 + 2 * i)))
    }

    /// The keys of a shape, in order.
    pub fn shape_keys(&self, shape: u32) -> Vec<u32> {
        let count: u32 = self.heap.arena().get(shape, Shape::COUNT);
        (0..count)
            .filter_map(|i| self.shape_key(shape, i))
            .collect()
    }

    pub fn class(&self, off: u32) -> Option<ObjectClass> {
        ObjectClass::from_u32(self.heap.arena().get(off, Object::CLASS))
    }

    /// The kind of a cell, with the class for objects: `Object/Array`.
    pub fn kind_name(&self, i: u32) -> String {
        let c = self.cell(i);
        if c.kind == CellKind::Object {
            match self.class(c.offset) {
                Some(cl) => format!("Object/{cl:?}"),
                None => "Object/?".into(),
            }
        } else {
            c.kind.layout().name.to_string()
        }
    }

    /// The template of a JS closure object.
    pub fn closure_template(&self, obj: u32) -> Option<u32> {
        if self.class(obj) != Some(ObjectClass::Function) {
            return None;
        }
        let v: Value = self.heap.arena().read(Object::SLOTS.at(obj, 0));
        v.as_internal()
    }

    /// The recorded local and capture names of a template (absent for privileged code).
    pub fn template_names(&self, t: u32) -> Vec<String> {
        let a = self.heap.arena();
        if !has_field(self.heap.cell_size(t), FuncTemplate::NAMES.offset) {
            return Vec::new();
        }
        let names: Ref = a.get(t, FuncTemplate::NAMES);
        let Some(n) = names.get() else {
            return Vec::new();
        };
        let len: u32 = a.get(n, U32Array::LEN);
        (0..len)
            .map(|i| {
                self.heap
                    .atom_name(Atom::from_id(a.read::<u32>(U32Array::DATA.at(n, i))))
            })
            .collect()
    }

    pub fn template_name(&self, t: u32) -> String {
        let name: u32 = self.heap.arena().get(t, FuncTemplate::NAME);
        let atom = Atom::from_id(name);
        if atom.is_null() {
            "<anonymous>".into()
        } else {
            self.heap.atom_name(atom)
        }
    }

    /// The file name of a template's script; privileged (prelude) templates have none.
    pub fn template_script(&self, t: u32) -> String {
        let a = self.heap.arena();
        if !has_field(self.heap.cell_size(t), FuncTemplate::SCRIPT.offset) {
            return "<prelude>".into();
        }
        let s: Ref = a.get(t, FuncTemplate::SCRIPT);
        let Some(s) = s.get() else { return "?".into() };
        let name: Ref = a.get(s, Script::NAME);
        name.get()
            .map(|n| self.heap.str_to_string(str_at(n)))
            .unwrap_or("?".into())
    }

    /// `line:col` of `pc` in a template, from its line table.
    pub fn template_line(&self, t: u32, pc: u32) -> Option<(u32, u32)> {
        let a = self.heap.arena();
        if !has_field(self.heap.cell_size(t), FuncTemplate::LINES.offset) {
            return None;
        }
        let lines: Ref = a.get(t, FuncTemplate::LINES);
        let l = lines.get()?;
        let n: u32 = a.get(l, djs_core::heap::cells::Bytes::LEN);
        let bytes = a.slice(djs_core::heap::cells::Bytes::DATA.at(l, 0), n);
        djs_core::bytecode::lookup_line(bytes, pc)
    }

    /// A short printable form of a string cell: quoted, at most `max` characters.
    pub fn string_preview(&self, off: u32, max: usize) -> String {
        let s = self.heap.str_to_string(str_at(off));
        let mut out: String = s.chars().take(max).collect();
        if s.chars().count() > max {
            out.push('…');
        }
        format!("{out:?}")
    }

    /// The edges of cell `i` with path labels. Storage cells (property values, elements,
    /// boxes) get empty labels, so paths read like JavaScript: `globalThis.files[3].text`.
    fn labelled_edges(&self, i: u32) -> Vec<(String, u32)> {
        let c = self.cell(i);
        let a = self.heap.arena();
        let layout = c.kind.layout();
        let mut out = Vec::new();
        // A suspended activation: its frame's locals by name, before the frame itself.
        if c.kind == CellKind::Object && self.class(c.offset) == Some(ObjectClass::Generator) {
            out.extend(self.frame_edges(c.offset));
        }
        let stack = self.stack();
        self.heap.trace_cell(c.offset, |slot| {
            let Some(t) = self.target(slot) else { return };
            let off = match slot {
                Slot::Value(o) | Slot::Ref(o) => o,
            };
            if let Some((s, sp)) = stack
                && s == c.offset
                && off >= ValueArray::VALUES.at(s, sp)
            {
                return;
            }
            let label = self.slot_label(c, layout, off - c.offset);
            out.push((label, t));
        });
        let _ = a;
        out
    }

    fn frame_edges(&self, generator: u32) -> Vec<(String, u32)> {
        let a = self.heap.arena();
        let mut out = Vec::new();
        let frame: Value = a.read(Object::SLOTS.at(generator, 1));
        let Some(fr) = frame.ref_offset() else {
            return out;
        };
        let hp = a
            .read::<Value>(Object::SLOTS.at(generator, 3))
            .as_int()
            .unwrap_or(0) as u32;
        let len: u32 = a.get(fr, ValueArray::LEN);
        let callee: Value = a.read(ValueArray::VALUES.at(fr, 0));
        let template = callee
            .as_object()
            .and_then(|f| self.closure_template(f.offset()));
        let (locals, names) = match template {
            Some(t) => (
                a.get::<u16>(t, FuncTemplate::LOCALS) as u32,
                self.template_names(t),
            ),
            None => (0, Vec::new()),
        };
        for k in 0..len {
            let v: Value = a.read(ValueArray::VALUES.at(fr, k));
            let Some(t) = v.ref_offset().and_then(|o| self.index(o)) else {
                continue;
            };
            let label = if k == 0 {
                "<callee>".to_string()
            } else if k == 1 {
                "<this>".to_string()
            } else if k < hp {
                format!("<arg{}>", k - 2)
            } else if k < hp + 5 {
                continue;
            } else if k < hp + 5 + locals {
                let j = (k - hp - 5) as usize;
                match names.get(j) {
                    Some(n) if !n.is_empty() => format!("<frame>.{n}"),
                    _ => format!("<frame>.local{j}"),
                }
            } else {
                format!("<frame>.operand{}", k - hp - 5 - locals)
            };
            out.push((label, t));
        }
        out
    }

    /// The label of the reference at `rel` bytes into cell `c`.
    fn slot_label(&self, c: CellInfo, layout: &CellLayout, rel: u32) -> String {
        let a = self.heap.arena();
        let field = layout
            .fields
            .iter()
            .find(|f| f.offset == rel)
            .map(|f| f.name);
        let tail = layout.tail.as_ref().and_then(|t| {
            (rel >= t.offset).then(|| {
                (
                    (rel - t.offset) / t.elem.size,
                    (rel - t.offset) % t.elem.size,
                )
            })
        });
        let generic = || match (field, tail) {
            (Some(f), _) => format!("<{}>", f.to_lowercase()),
            (None, Some((i, _))) => format!("[{i}]"),
            _ => format!("<+{rel}>"),
        };
        match c.kind {
            CellKind::Object => match field {
                Some("PROTO") => "<proto>".into(),
                Some("PROPS") | Some("ELEMENTS") => String::new(),
                _ => match tail {
                    Some((i, _)) => self.object_slot_label(c.offset, i),
                    None => generic(),
                },
            },
            CellKind::PropValues => match tail {
                Some((i, _)) => {
                    let shape: Ref = a.get(c.offset, PropValues::SHAPE);
                    match shape.get().and_then(|s| self.shape_key(s, i)) {
                        Some(k) => self.key_label(k),
                        None => format!("[slot {i}]"),
                    }
                }
                None => generic(),
            },
            CellKind::PropTable => match tail {
                Some((i, _)) => {
                    let key: u32 = a.get(PropTable::ENTRIES.at(c.offset, i), PropEntry::KEY);
                    self.key_label(key)
                }
                None => generic(),
            },
            CellKind::AccessorPair => {
                if rel == AccessorPair::GETTER.offset {
                    "<get>".into()
                } else {
                    "<set>".into()
                }
            }
            CellKind::BoxCell => String::new(),
            CellKind::Realm => match tail {
                Some((i, _)) => format!("%intrinsic{i}"),
                None => generic(),
            },
            CellKind::HandleTable => match tail {
                Some((i, _)) => format!("handle[{}]", i + 1),
                None => generic(),
            },
            _ => generic(),
        }
    }

    fn object_slot_label(&self, o: u32, i: u32) -> String {
        use ObjectClass as C;
        let name = match (self.class(o), i) {
            (Some(C::Function), 0) => "<code>",
            (Some(C::Function), _) => {
                let t = self.closure_template(o);
                let names = t.map(|t| self.template_names(t)).unwrap_or_default();
                let locals = t
                    .map(|t| self.heap.arena().get::<u16>(t, FuncTemplate::LOCALS) as usize)
                    .unwrap_or(0);
                return match names.get(locals + i as usize - 1) {
                    Some(n) if !n.is_empty() => format!("~{n}"),
                    _ => format!("~capture{}", i - 1),
                };
            }
            (Some(C::BoundFunction), 0) => "<target>",
            (Some(C::BoundFunction), 1) => "<bound this>",
            (Some(C::BoundFunction), 2) => "<bound args>",
            (Some(C::Map | C::Set | C::WeakMap | C::WeakSet), 0) => "<entries>",
            (Some(C::Map | C::Set | C::WeakMap | C::WeakSet), 1) => "<index>",
            (Some(C::Promise), 1) => "<result>",
            (Some(C::Promise), 2) => "<reactions>",
            (Some(C::Generator), 1) => "<frame>",
            (Some(C::Generator), 4) => "<promise>",
            (Some(C::Generator), 6) => "<requests>",
            (Some(C::Proxy), 0) => "<target>",
            (Some(C::Proxy), 1) => "<handler>",
            (Some(C::BuiltinClosure), 1) => "<data>",
            (Some(C::RegExp), 0) => "<program>",
            (Some(C::RegExp), 1) => "<source>",
            _ => return format!("<slot{i}>"),
        };
        name.into()
    }
}
