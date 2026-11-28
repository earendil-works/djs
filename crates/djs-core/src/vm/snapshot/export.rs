//! Export: a waiting VM as a logical graph (see the module docs of `snapshot`).

use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::schema::{self, ClassSpec, KindSpec, SlotTy};
use super::wire::Writer;
use super::*;
use crate::bytecode::template;
use crate::heap::cells::{Interp, Object, Realm, Roots, Script};
use crate::heap::layout::{FieldInfo, FieldTy};
use crate::heap::{Atom, CellKind, ConstSet, Heap, ObjectClass, Property, RecordSchema};
use crate::interp::realm::{Intrinsic, intrinsic};
use crate::interp::{RunState, state};
use crate::value::{Kind, Obj, Str, Value, tag};

/// One record type of the snapshot: a name and its fields' names and types.
#[derive(Clone)]
pub(super) struct TypeDef {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

/// A host value a snapshot names (D3720): a root by its name, an object its root (or a
/// member) holds under a key, or a host function by its identity.
#[derive(Clone, Copy)]
pub(super) enum HostEntry {
    /// The name (a string id).
    Root(u32),
    /// The parent entry and the key (a string id).
    Member(u32, u32),
    /// The identity (a string id).
    Function(u32),
}

/// Where a host value sits, before it is written: a root (its index in
/// `HostNames::values`) or a member (the parent cell and the key).
#[derive(Clone, Copy)]
enum HostSpot {
    Root(usize),
    Member(u32, Atom),
}

pub(super) struct Exporter<'a> {
    h: &'a Heap,
    /// A host's run (D3720): what it names.
    host: Option<&'a HostNames>,
    host_spots: BTreeMap<u32, HostSpot>,
    host_ids: BTreeMap<u32, u32>,
    host_fn_ids: BTreeMap<u32, u32>,
    pub host_entries: Vec<HostEntry>,
    base: &'a BaseInfo,
    base_end: u32,
    pub types: Vec<TypeDef>,
    type_ids: BTreeMap<String, u32>,
    /// Per cell kind or object class: its type id (`u32::MAX`: not made yet).
    pub strings: Vec<(bool, Vec<u8>)>,
    string_ids: BTreeMap<u32, u32>,
    atom_strings: BTreeMap<u32, u32>,
    pub bases: Vec<u32>,
    base_ids: BTreeMap<u32, u32>,
    pub intrinsics: Vec<u16>,
    intrinsic_ids: BTreeMap<u16, u32>,
    pub scripts: Vec<u32>,
    script_ids: BTreeMap<u32, u32>,
    pub functions: Vec<u32>,
    function_ids: BTreeMap<u32, u32>,
    /// Templates with a stored frame (a Generator's callee): their bytecode must match.
    pinned: alloc::collections::BTreeSet<u32>,
    record_ids: BTreeMap<u32, u32>,
    queue: VecDeque<u32>,
    pub records: Writer,
    pub record_count: u32,
    /// The patch records of the per-VM objects: global object, global lexical record,
    /// `Symbol.for` registry (their record ids are fixed: 0, 1, 2).
    patched: [u32; 3],
    pub refusals: Vec<String>,
    /// The path to the cell being written, for refusals.
    path: Vec<String>,
    /// The named-constant sets the types use (D3710), written with the flag sets.
    pub const_sets: Vec<&'static ConstSet>,
}

const PATCH_NAMES: [&str; 3] = ["Patch.Global", "Patch.GlobalLex", "Patch.SymbolRegistry"];

/// Tests (tests.rs): an exporter that writes what another build would (D3712): the slots
/// and the constants' members in another order, or a field or a member by another name, or
/// a field less.
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum Skew {
    None,
    Reverse,
    RenameField(&'static str),
    RenameMember(&'static str),
    DropField(&'static str),
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
std::thread_local! {
    pub(super) static SKEW: core::cell::Cell<Skew> = const { core::cell::Cell::new(Skew::None) };
}

fn skew() -> Skew {
    #[cfg(test)]
    return SKEW.with(|s| s.get());
    #[cfg(not(test))]
    Skew::None
}

/// The order a snapshot writes slots `0..n` in (reversed by the `Reverse` skew).
fn slot_order(n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    if skew() == Skew::Reverse {
        v.reverse();
    }
    v
}

/// A field's name as written (the `RenameField` skew).
fn field_name(n: &str) -> String {
    match skew() {
        Skew::RenameField(r) if r == n => format!("{n}Renamed"),
        _ => n.to_string(),
    }
}

/// Whether a field is written (the `DropField` skew drops one).
fn field_written(n: &str) -> bool {
    !matches!(skew(), Skew::DropField(d) if d == n)
}

/// The index constant set `c`'s member `i` is written as (reversed by the `Reverse` skew).
fn member_index(c: &ConstSet, i: usize) -> usize {
    if skew() == Skew::Reverse {
        c.members.len() - 1 - i
    } else {
        i
    }
}

pub(super) fn refuse_unless_idle(h: &Heap) -> Result<(), String> {
    if !matches!(
        state::state(h),
        RunState::Idle | RunState::Done | RunState::Threw
    ) {
        return Err(format!("not idle: {:?}", state::state(h)));
    }
    let a = h.arena();
    let i = state::interp(h);
    let jobs: Ref = a.get(0, Roots::JOBS);
    if let Some(j) = jobs.get()
        && h.value_array_len(j) > a.get(i, Interp::JOB_HEAD)
    {
        return Err(String::from("not idle: jobs are queued"));
    }
    let inbox: Ref = a.get(i, Interp::INBOX);
    if let Some(b) = inbox.get()
        && h.value_array_len(b) > a.get(i, Interp::INBOX_HEAD)
    {
        return Err(String::from("not idle: host results wait in the inbox"));
    }
    if a.get(i, Interp::SP) != 0 {
        return Err(String::from("not idle: frames on the stack"));
    }
    if h.prefix_written() {
        return Err(String::from("the builtins were written"));
    }
    Ok(())
}

impl<'a> Exporter<'a> {
    pub fn new(h: &'a Heap, base: &'a BaseInfo, host: Option<&'a HostNames>) -> Exporter<'a> {
        let mut e = Exporter {
            h,
            host,
            host_spots: BTreeMap::new(),
            host_ids: BTreeMap::new(),
            host_fn_ids: BTreeMap::new(),
            host_entries: Vec::new(),
            base,
            base_end: h.base_end(),
            types: Vec::new(),
            type_ids: BTreeMap::new(),
            strings: Vec::new(),
            string_ids: BTreeMap::new(),
            atom_strings: BTreeMap::new(),
            bases: Vec::new(),
            base_ids: BTreeMap::new(),
            intrinsics: Vec::new(),
            intrinsic_ids: BTreeMap::new(),
            scripts: Vec::new(),
            script_ids: BTreeMap::new(),
            functions: Vec::new(),
            function_ids: BTreeMap::new(),
            pinned: alloc::collections::BTreeSet::new(),
            record_ids: BTreeMap::new(),
            queue: VecDeque::new(),
            records: Writer::default(),
            record_count: 0,
            patched: [0; 3],
            refusals: Vec::new(),
            path: Vec::new(),
            const_sets: Vec::new(),
        };
        // The per-VM objects come first, as patches.
        let objs = [
            intrinsic(h, Intrinsic::Global).as_object(),
            Some(crate::interp::realm::global_lex(h)),
            intrinsic(h, Intrinsic::SymbolRegistry).as_object(),
        ];
        for (i, o) in objs.into_iter().enumerate() {
            let o = o.map_or(0, |o| o.offset());
            e.patched[i] = o;
            e.record_ids.insert(o, i as u32);
            e.queue.push_back(o);
        }
        e.record_count = 3;
        if let Some(host) = host {
            e.host_spots(host);
        }
        e
    }

    /// Marks the host values (D3720): each root, and every object reachable from it
    /// through own data properties, as a member (a snapshot keeps the identity of every
    /// object of the host's data). A host function
    /// that is no member is named by its identity, and a Proxy's insides are not looked
    /// at.
    fn host_spots(&mut self, host: &HostNames) {
        let h = self.h;
        let mut todo = Vec::new();
        for (i, (_, v)) in host.values.iter().enumerate() {
            if let Some(o) = self.host_object(*v)
                && self.h.object_class(o) != ObjectClass::HostFunction
            {
                self.host_spots.insert(o.offset(), HostSpot::Root(i));
                todo.push(o);
            }
        }
        while let Some(o) = todo.pop() {
            if h.object_class(o) == ObjectClass::Proxy {
                continue;
            }
            for k in h.own_keys(o) {
                if h.atom_symbol(k).is_some() {
                    continue;
                }
                if let Some(Property::Data { value, .. }) = h.get_own_property(o, k)
                    && let Some(c) = self.host_object(value)
                {
                    self.host_spots
                        .insert(c.offset(), HostSpot::Member(o.offset(), k));
                    todo.push(c);
                }
            }
        }
    }

    /// `v` as an object a host value can be: above the builtins, not marked yet.
    fn host_object(&self, v: Value) -> Option<Obj> {
        let o = v.as_object()?;
        (o.offset() >= self.base_end && !self.host_spots.contains_key(&o.offset())).then_some(o)
    }

    /// The host entry of cell `off`, a host value or a host function, or `None` when it
    /// is neither (or the snapshot is no host's).
    fn host_entry(&mut self, off: u32) -> Option<u32> {
        let host = self.host?;
        if let Some(&i) = self.host_ids.get(&off) {
            return Some(i);
        }
        let entry = match self.host_spots.get(&off).copied() {
            Some(HostSpot::Root(i)) => HostEntry::Root(self.str_lit(&host.values[i].0)),
            Some(HostSpot::Member(p, k)) => {
                let parent = self.host_entry(p)?;
                let n = self.h.atom_name(k);
                HostEntry::Member(parent, self.str_lit(&n))
            }
            None => {
                let o = Obj(off);
                if self.h.cell_kind(off) != CellKind::Object
                    || self.h.object_class(o) != ObjectClass::HostFunction
                {
                    return None;
                }
                let id = self.h.object_slot(o, 0).as_int().unwrap_or(-1) as u32;
                let i = self.host_function(id);
                self.host_ids.insert(off, i);
                return Some(i);
            }
        };
        let i = self.host_entries.len() as u32;
        self.host_entries.push(entry);
        self.host_ids.insert(off, i);
        Some(i)
    }

    /// The host entry of host function `id`, by its identity.
    fn host_function(&mut self, id: u32) -> u32 {
        if let Some(&i) = self.host_fn_ids.get(&id) {
            return i;
        }
        let name = self
            .host
            .and_then(|h| h.functions.iter().find(|(i, _)| *i == id))
            .map(|(_, n)| n.clone());
        let n = match name {
            Some(n) => self.str_lit(&n),
            None => {
                self.refuse(format!(
                    "a host function (id {id}) the host has no name for"
                ));
                0
            }
        };
        let i = self.host_entries.len() as u32;
        self.host_entries.push(HostEntry::Function(n));
        self.host_fn_ids.insert(id, i);
        i
    }

    pub fn h(&self) -> &'a Heap {
        self.h
    }

    pub fn base_path(&self, off: u32) -> String {
        self.base.paths[&off].clone()
    }

    fn refuse(&mut self, why: String) {
        let at = self.path.join(" ");
        self.refusals.push(format!("{at}: {why}"));
    }

    pub fn type_id(&mut self, name: &str, fields: impl FnOnce() -> Vec<(String, String)>) -> u32 {
        if let Some(&t) = self.type_ids.get(name) {
            return t;
        }
        let id = self.types.len() as u32;
        self.types.push(TypeDef {
            name: name.to_string(),
            fields: fields(),
        });
        self.type_ids.insert(name.to_string(), id);
        id
    }

    // ---------------------------------------------------------------- values

    fn string_id(&mut self, s: Str) -> u32 {
        if let Some(&i) = self.string_ids.get(&s.offset()) {
            return i;
        }
        let id = self.push_string(s);
        self.string_ids.insert(s.offset(), id);
        id
    }

    fn push_string(&mut self, s: Str) -> u32 {
        let h = self.h;
        let two = h.str_is_two_byte(s);
        let mut data = Vec::with_capacity(h.str_len(s) as usize * if two { 2 } else { 1 });
        for c in h.str_chunks(s) {
            if two {
                for i in 0..c.len() {
                    data.extend_from_slice(&c.get(i).to_le_bytes());
                }
            } else if c.is_two_byte() {
                for i in 0..c.len() {
                    data.push(c.get(i) as u8);
                }
            } else {
                data.extend_from_slice(c.bytes());
            }
        }
        let id = self.strings.len() as u32;
        self.strings.push((two, data));
        id
    }

    pub fn str_lit(&mut self, s: &str) -> u32 {
        // Literal strings (names): latin1 when they fit, else UTF-16.
        let entry = if s.chars().all(|c| (c as u32) < 256) {
            (false, s.chars().map(|c| c as u8).collect())
        } else {
            let mut d = Vec::new();
            for u in s.encode_utf16() {
                d.extend_from_slice(&u.to_le_bytes());
            }
            (true, d)
        };
        // Once: a host value's name is often also the global's key (`console`). The
        // names are few, so a scan does.
        if let Some(i) = self.strings.iter().position(|e| *e == entry) {
            return i as u32;
        }
        let id = self.strings.len() as u32;
        self.strings.push(entry);
        id
    }

    /// A property key: a string (by atom, deduplicated) or a symbol.
    fn key(&mut self, w: &mut Writer, k: Atom) {
        if let Some(s) = self.h.atom_symbol(k) {
            self.value(w, Value::symbol(s));
            return;
        }
        let id = match self.atom_strings.get(&k.id()) {
            Some(&i) => i,
            None => {
                // Integer atoms (array indexes) have no string cell.
                let i = match self.h.atom_string_of(k) {
                    Some(s) => self.string_id(s),
                    None => {
                        let n = self.h.atom_name(k);
                        self.str_lit(&n)
                    }
                };
                self.atom_strings.insert(k.id(), i);
                i
            }
        };
        w.byte(V_STR);
        w.str_ref(id);
    }

    pub fn value(&mut self, w: &mut Writer, v: Value) {
        match v.kind() {
            Kind::Undefined => w.byte(V_UNDEF),
            Kind::Null => w.byte(V_NULL),
            Kind::Bool => w.byte(if v.as_bool() == Some(true) {
                V_TRUE
            } else {
                V_FALSE
            }),
            Kind::Number => match v.tag() {
                Some(tag::INT) => {
                    w.byte(V_INT);
                    w.i(i64::from(v.as_int().unwrap()));
                }
                _ => {
                    w.byte(V_F64);
                    w.u64_raw(v.to_bits());
                }
            },
            Kind::Hole => w.byte(V_HOLE),
            Kind::Uninitialized => w.byte(V_UNINIT),
            Kind::String => {
                let id = self.string_id(v.as_string().unwrap());
                w.byte(V_STR);
                w.str_ref(id);
            }
            Kind::HostRef => {
                self.refuse(String::from("a host ref"));
                w.byte(V_UNDEF);
            }
            Kind::Object | Kind::Symbol | Kind::BigInt | Kind::Internal => {
                let off = v.ref_offset().unwrap();
                self.cell(w, off, v.tag().unwrap());
            }
        }
    }

    /// A reference field (`Ref`): null or a cell, written like a value.
    fn reference(&mut self, w: &mut Writer, r: Ref) {
        match r.get() {
            None => w.byte(V_NULL),
            Some(off) => {
                let t = natural_tag(self.h.cell_kind(off));
                self.cell(w, off, t)
            }
        }
    }

    /// A reference to cell `off` as a value with tag `t`.
    fn cell(&mut self, w: &mut Writer, off: u32, t: u16) {
        let h = self.h;
        let kind = h.cell_kind(off);
        let internal = t == tag::INTERNAL && natural_tag(kind) != tag::INTERNAL;
        if t != tag::INTERNAL && t != natural_tag(kind) {
            self.refuse(format!("a value tagged {t} refers to a {kind:?}"));
        }
        if matches!(
            kind,
            CellKind::Str8 | CellKind::Str16 | CellKind::Rope | CellKind::Slice
        ) {
            if internal || t == tag::INTERNAL {
                self.refuse(String::from("an internal reference to a string"));
            }
            let id = self.string_id(Str(off));
            w.byte(V_STR);
            w.str_ref(id);
            return;
        }
        if off < self.base_end {
            if !self.base.paths.contains_key(&off) {
                // Unnamed builtin cells: a template of a builtin, an internal record.
                self.refuse(format!("an unnamed builtin cell (a {kind:?})"));
                w.byte(V_UNDEF);
                return;
            }
            let p = off;
            let id = match self.base_ids.get(&p) {
                Some(&i) => i,
                None => {
                    let i = self.bases.len() as u32;
                    self.bases.push(p);
                    self.base_ids.insert(p, i);
                    i
                }
            };
            w.byte(if internal { V_BASE_INTERNAL } else { V_BASE });
            w.u(u64::from(id));
            return;
        }
        if !internal
            && kind == CellKind::Object
            && let Some(i) = self.host_entry(off)
        {
            w.byte(V_HOST);
            w.u(u64::from(i));
            return;
        }
        match kind {
            CellKind::FuncTemplate => {
                let id = self.function_id(off);
                w.byte(V_FN);
                w.u(u64::from(id));
            }
            CellKind::Script => {
                let id = self.script_id(off);
                w.byte(V_SCRIPT);
                w.u(u64::from(id));
            }
            _ => {
                let id = match self.record_ids.get(&off) {
                    Some(&i) => i,
                    None => {
                        let i = self.record_count;
                        self.record_count += 1;
                        self.record_ids.insert(off, i);
                        self.queue.push_back(off);
                        i
                    }
                };
                w.byte(if internal { V_REC_INTERNAL } else { V_REC });
                w.rec_ref(id);
            }
        }
    }

    fn script_id(&mut self, s: u32) -> u32 {
        if let Some(&i) = self.script_ids.get(&s) {
            return i;
        }
        let a = self.h.arena();
        if a.get::<Ref>(s, Script::SOURCE).is_null() {
            self.refuse(String::from("a script without its source"));
        }
        let i = self.scripts.len() as u32;
        self.scripts.push(s);
        self.script_ids.insert(s, i);
        i
    }

    fn function_id(&mut self, t: u32) -> u32 {
        if let Some(&i) = self.function_ids.get(&t) {
            return i;
        }
        let d = template::debug_info(self.h, t);
        match d.script.get() {
            Some(s) => {
                self.script_id(s);
            }
            None => self.refuse(String::from("a function template without a script")),
        }
        let i = self.functions.len() as u32;
        self.functions.push(t);
        self.function_ids.insert(t, i);
        i
    }

    pub fn script_index(&self, s: u32) -> u32 {
        self.script_ids[&s]
    }

    fn intrinsic_id(&mut self, id: u16) -> u32 {
        if let Some(&i) = self.intrinsic_ids.get(&id) {
            return i;
        }
        let i = self.intrinsics.len() as u32;
        self.intrinsics.push(id);
        self.intrinsic_ids.insert(id, i);
        i
    }

    // ---------------------------------------------------------------- records

    /// Writes every record reachable so far, in id order.
    pub fn drain(&mut self) {
        while let Some(off) = self.queue.pop_front() {
            let mut w = core::mem::take(&mut self.records);
            if let Some(i) = self.patched.iter().position(|&p| p == off) {
                self.patch(&mut w, i, off);
            } else {
                self.record(&mut w, off);
            }
            self.records = w;
        }
    }

    fn record(&mut self, w: &mut Writer, off: u32) {
        let h = self.h;
        let kind = h.cell_kind(off);
        if kind == CellKind::Object {
            let o = Obj(off);
            let class = h.object_class(o);
            if schema::is_kinded(class) {
                // A record: its kind's schema names the slots (D3710).
                match crate::heap::record_schema(h.record_kind(o)) {
                    Some(r) if r.class == class => self.object(w, o, &Slots::record(r)),
                    _ => {
                        self.path.push(format!("{class:?}"));
                        self.refuse(format!("a {class:?} of no known record kind"));
                        self.path.pop();
                        let t = self.type_id("Refused", Vec::new);
                        w.u(u64::from(t));
                    }
                }
                return;
            }
            match schema::class_spec(class) {
                Some(spec) => self.object(w, o, &Slots::class(spec)),
                None => {
                    self.path.push(format!("{class:?}"));
                    self.refuse(format!("an object of class {class:?}"));
                    self.path.pop();
                    let t = self.type_id("Refused", Vec::new);
                    w.u(u64::from(t));
                }
            }
            return;
        }
        match schema::kind_spec(kind) {
            Some(spec) => self.generic(w, off, spec),
            None => {
                self.refuse(format!("a {kind:?} cell"));
                let t = self.type_id("Refused", Vec::new);
                w.u(u64::from(t));
            }
        }
    }

    /// The fields of generic kind `spec` that a snapshot writes, with their types.
    fn kind_fields(spec: &KindSpec, host: bool) -> Vec<(String, String)> {
        let l = spec.kind.layout();
        let mut out = Vec::new();
        for f in l.fields {
            if spec.skip.contains(&f.name) || spec.count == Some(f.name) {
                continue;
            }
            let ty = if host && is_host_call_fn(spec, f) {
                "hostfn"
            } else {
                field_ty(f.ty)
            };
            out.push((f.name.to_string(), ty.to_string()));
        }
        if !spec.flags.is_empty() {
            out.push((
                String::from("header"),
                flags_ty(spec.kind.layout().name, spec.flags),
            ));
        }
        if let Some(t) = &l.tail {
            let elem = if t.elem.fields.len() == 1 && t.elem.fields[0].name.is_empty() {
                field_ty(t.elem.fields[0].ty).to_string()
            } else {
                let parts: Vec<String> = t
                    .elem
                    .fields
                    .iter()
                    .map(|f| format!("{}:{}", f.name, field_ty(f.ty)))
                    .collect();
                format!("record({})", parts.join(","))
            };
            out.push((t.name.to_string(), format!("tail({elem})")));
        }
        out
    }

    fn generic(&mut self, w: &mut Writer, off: u32, spec: &KindSpec) {
        let h = self.h;
        let l = spec.kind.layout();
        let host = self.host.is_some();
        let t = self.type_id(l.name, || Self::kind_fields(spec, host));
        w.u(u64::from(t));
        self.path.push(l.name.to_string());
        let a = h.arena();
        for f in l.fields {
            if spec.skip.contains(&f.name) || spec.count == Some(f.name) {
                continue;
            }
            if host && is_host_call_fn(spec, f) {
                // An open call's function by identity (D3720).
                let i = self.host_function(a.read::<u32>(off + f.offset));
                w.u(u64::from(i));
                continue;
            }
            self.field(w, off + f.offset, f);
        }
        if !spec.flags.is_empty() {
            let fl = h.cell_flags(off);
            let mut bits = 0u64;
            let mut known = 0u16;
            for (i, (_, b)) in spec.flags.iter().enumerate() {
                known |= b;
                if fl & b != 0 {
                    bits |= 1 << i;
                }
            }
            if fl & !known != 0 {
                self.refuse(format!("unknown header flags {:#x}", fl & !known));
            }
            w.u(bits);
        }
        if let Some(tail) = &l.tail {
            let cap = l.tail_capacity(h.cell_size(off));
            let n = match spec.count {
                Some(c) => {
                    let f = l.fields.iter().find(|f| f.name == c).unwrap();
                    let n = read_uint(a, off + f.offset, f.ty) as u32;
                    n.min(cap)
                }
                None => cap,
            };
            w.u(u64::from(n));
            for i in 0..n {
                let at = off + tail.offset + i * tail.elem.size;
                for f in tail.elem.fields {
                    self.field(w, at + f.offset, f);
                }
            }
        }
        self.path.pop();
    }

    fn field(&mut self, w: &mut Writer, at: u32, f: &FieldInfo) {
        let a = self.h.arena();
        match f.ty {
            FieldTy::Value => {
                let v: Value = a.read(at);
                self.value(w, v);
            }
            FieldTy::Ref => {
                let r: Ref = a.read(at);
                self.reference(w, r);
            }
            FieldTy::I32 => w.i(i64::from(a.read::<i32>(at))),
            FieldTy::F64 => w.f64(a.read::<f64>(at)),
            t => w.u(read_uint(a, at, t)),
        }
    }

    fn object_fields(spec: &Slots) -> Vec<(String, String)> {
        let mut f = alloc::vec![
            (String::from("proto"), String::from("ref")),
            (
                String::from("header"),
                flags_ty("Object", schema::OBJECT_FLAGS)
            ),
            (String::from("props"), String::from("props")),
        ];
        if spec.class == ObjectClass::Function {
            f.push((String::from("template"), String::from("function")));
        } else {
            f.push((String::from("elements"), String::from("values")));
        }
        for i in slot_order(spec.slots.len()) {
            let (n, t) = spec.slots[i];
            if field_written(n) {
                f.push((field_name(n), schema::slot_ty_name(t)));
            }
        }
        if let Some(r) = spec.rest {
            f.push((r.to_string(), String::from("values")));
        }
        f
    }

    fn object(&mut self, w: &mut Writer, o: Obj, spec: &Slots) {
        let h = self.h;
        let name = spec.type_name();
        for (_, t) in spec.slots {
            if let SlotTy::Enum(c) | SlotTy::Bits(c) = t
                && !self.const_sets.iter().any(|s| s.name == c.name)
            {
                self.const_sets.push(c);
            }
        }
        let t = self.type_id(&name, || Self::object_fields(spec));
        w.u(u64::from(t));
        self.path.push(name);
        let a = h.arena();
        let proto: Ref = a.get(o.offset(), Object::PROTO);
        self.reference(w, proto);
        self.object_flags(w, o);
        self.props(w, o);
        let el = h.snap_elements(o);
        if spec.class == ObjectClass::Function {
            if h.cell_kind(el) != CellKind::FuncTemplate {
                self.refuse(String::from("a closure without a template"));
            }
            self.cell(w, el, tag::INTERNAL);
        } else if el == 0 {
            w.u(0);
        } else {
            let n = h.value_array_len(el);
            w.u(u64::from(n));
            for i in 0..n {
                self.value(w, h.value_array_get(el, i));
            }
        }
        if spec.class == ObjectClass::Generator
            && let Some(frame) = h.object_slot(o, 1).as_internal()
            && h.value_array_len(frame) > 0
            && let Some(callee) = h.value_array_get(frame, 0).as_object()
            && h.object_class(callee) == ObjectClass::Function
        {
            self.pinned.insert(h.closure_template(callee));
        }
        let count = h.object_slot_count(o);
        let fixed = spec.slots.len() as u32;
        if count < fixed || (spec.rest.is_none() && count != fixed) {
            self.refuse(format!("{count} slots, the schema has {fixed}"));
        }
        for i in slot_order(spec.slots.len()) {
            let (n, ty) = &spec.slots[i];
            if !field_written(n) {
                continue;
            }
            let v = if (i as u32) < count {
                h.object_slot(o, i as u32)
            } else {
                Value::UNDEFINED
            };
            self.path.push(n.to_string());
            match ty {
                SlotTy::Value => self.value(w, v),
                SlotTy::Intrinsic => match v.as_int() {
                    Some(id) => {
                        let i = self.intrinsic_id(id as u16);
                        w.u(u64::from(i));
                    }
                    None => {
                        self.refuse(String::from("not an intrinsic id"));
                        w.u(0);
                    }
                },
                SlotTy::Buckets => {
                    let n = v.as_internal().map_or(0, |ix| h.u32_array_len(ix));
                    w.u(u64::from(n));
                }
                SlotTy::Program => {}
                SlotTy::Enum(c) | SlotTy::Bits(c) => self.constant(w, v, c),
            }
            self.path.pop();
        }
        if spec.rest.is_some() {
            let n = count.saturating_sub(fixed);
            w.u(u64::from(n));
            for i in 0..n {
                self.value(w, h.object_slot(o, fixed + i));
            }
        }
        self.path.pop();
    }

    /// A field of named constants (D3710): 0 and the value for anything but an int32 (a
    /// field not set yet), else 1 + the member's index in the set (an enum) or 1 + the
    /// members' bits by index (flags).
    fn constant(&mut self, w: &mut Writer, v: Value, c: &'static ConstSet) {
        let Some(n) = v.as_int() else {
            w.u(0);
            self.value(w, v);
            return;
        };
        if c.flags {
            let mut bits = 0u64;
            let mut known = 0i32;
            for (i, (_, b)) in c.members.iter().enumerate() {
                known |= b;
                if n & b != 0 {
                    bits |= 1 << member_index(c, i);
                }
            }
            if n & !known != 0 {
                self.refuse(format!("{n:#x} has bits that are no {}", c.name));
            }
            w.u(bits + 1);
        } else {
            match c.members.iter().position(|(_, m)| *m == n) {
                Some(i) => w.u(member_index(c, i) as u64 + 1),
                None => {
                    self.refuse(format!("{n} is no {} member", c.name));
                    w.u(0);
                    self.value(w, v);
                }
            }
        }
    }

    fn object_flags(&mut self, w: &mut Writer, o: Obj) {
        let fl = self.h.snap_obj_flags(o);
        let mut bits = 0u64;
        for (i, (_, b)) in schema::OBJECT_FLAGS.iter().enumerate() {
            if fl & b != 0 {
                bits |= 1 << i;
            }
        }
        w.u(bits);
    }

    fn props(&mut self, w: &mut Writer, o: Obj) {
        let h = self.h;
        let n = h.snap_prop_slots(o);
        let live: Vec<(Atom, Property)> = (0..n).filter_map(|i| h.snap_prop(o, i)).collect();
        w.u(live.len() as u64);
        for (k, p) in live {
            self.prop(w, k, &p);
        }
    }

    fn prop(&mut self, w: &mut Writer, k: Atom, p: &Property) {
        self.key(w, k);
        match *p {
            Property::Data { value, flags } => {
                w.u(u64::from(flags.bits()));
                self.value(w, value);
            }
            Property::Accessor {
                getter,
                setter,
                flags,
            } => {
                w.u(u64::from(flags.bits()) | PROP_ACCESSOR);
                self.value(w, getter);
                self.value(w, setter);
            }
        }
    }

    /// A per-VM object as a patch over the fresh VM's: the keys it no longer has, then
    /// the properties that differ, then every property after the first one out of the
    /// fresh order.
    fn patch(&mut self, w: &mut Writer, which: usize, off: u32) {
        let t = self.type_id(PATCH_NAMES[which], || {
            alloc::vec![
                (String::from("proto"), String::from("ref")),
                (
                    String::from("header"),
                    flags_ty("Object", schema::OBJECT_FLAGS)
                ),
                (String::from("deleted"), String::from("values")),
                (String::from("props"), String::from("props")),
                (String::from("elements"), String::from("values")),
            ]
        });
        w.u(u64::from(t));
        self.path.push(PATCH_NAMES[which].to_string());
        let h = self.h;
        let o = Obj(off);
        if off == 0 {
            w.byte(V_NULL);
            w.u(0);
            w.u(0);
            w.u(0);
            w.u(0);
            self.path.pop();
            return;
        }
        let proto: Ref = h.arena().get(off, Object::PROTO);
        self.reference(w, proto);
        self.object_flags(w, o);
        let fresh = &self.base.fresh[which];
        let n = h.snap_prop_slots(o);
        let cur: Vec<(Atom, Property)> = (0..n).filter_map(|i| h.snap_prop(o, i)).collect();
        // The fresh keys kept in order (a prefix-subsequence), the rest re-added.
        let mut kept = Vec::new();
        let mut pos = 0;
        let mut split = cur.len();
        for (i, (k, _)) in cur.iter().enumerate() {
            match fresh[pos..].iter().position(|f| f.0 == k.id()) {
                Some(j) => {
                    pos += j + 1;
                    kept.push(k.id());
                }
                None => {
                    split = i;
                    break;
                }
            }
        }
        let deleted: Vec<u32> = fresh
            .iter()
            .map(|f| f.0)
            .filter(|k| !kept.contains(k))
            .collect();
        w.u(deleted.len() as u64);
        for k in deleted {
            self.key(w, Atom::from_id(k));
        }
        let mut out = Vec::new();
        for (i, (k, p)) in cur.iter().enumerate() {
            if i < split {
                let f = fresh.iter().find(|f| f.0 == k.id()).unwrap();
                if self.same_prop(p, &f.1) {
                    continue;
                }
            }
            out.push((*k, *p));
        }
        w.u(out.len() as u64);
        for (k, p) in out {
            self.prop(w, k, &p);
        }
        let el = h.snap_elements(o);
        let n = if el == 0 { 0 } else { h.value_array_len(el) };
        w.u(u64::from(n));
        for i in 0..n {
            self.value(w, h.value_array_get(el, i));
        }
        self.path.pop();
    }

    /// Whether a per-VM object's property is as in the fresh VM: the same attributes and
    /// the same builtin, primitive or per-VM object.
    fn same_prop(&self, p: &Property, f: &Property) -> bool {
        let same = |a: Value, b: Value| {
            a == b || {
                let g = intrinsic(self.h, Intrinsic::Global);
                a == g && b == self.base.fresh_global
            }
        };
        match (*p, *f) {
            (
                Property::Data { value, flags },
                Property::Data {
                    value: v2,
                    flags: f2,
                },
            ) => flags == f2 && same(value, v2),
            (
                Property::Accessor {
                    getter,
                    setter,
                    flags,
                },
                Property::Accessor {
                    getter: g2,
                    setter: s2,
                    flags: f2,
                },
            ) => flags == f2 && same(getter, g2) && same(setter, s2),
            _ => false,
        }
    }

    // ---------------------------------------------------------------- the VM root

    /// The VM's own state besides the graph: the interpreter's, the host calls and
    /// handles, the realm's per-VM values.
    pub fn root(&mut self, w: &mut Writer) {
        let host = self.host;
        let t = self.type_id("Vm", || {
            let handles: &[(&str, &str)] = if host.is_some() {
                &[("named", "values"), ("hostData", "value")]
            } else {
                &[("handles", "ref")]
            };
            [
                ("nextCallId", "uint"),
                ("state", "uint"),
                ("outcome", "uint"),
                ("result", "value"),
                ("throw", "value"),
                ("rejections", "ref"),
                ("reported", "uint"),
                ("hostCalls", "ref"),
            ]
            .iter()
            .chain(handles)
            .chain(&[
                ("global", "value"),
                ("globalLex", "value"),
                ("symbolRegistry", "value"),
                ("random", "values"),
                ("clock", "value"),
                ("timeZoneOffset", "value"),
                ("globalLexEpoch", "value"),
                ("keptObjects", "value"),
                ("registries", "value"),
                ("modules", "value"),
                ("frozenBuiltins", "uint"),
                ("weakRefsStrong", "uint"),
            ])
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
        });
        w.u(u64::from(t));
        self.path.push(String::from("Vm"));
        let h = self.h;
        let a = h.arena();
        let i = state::interp(h);
        w.u(u64::from(a.get(i, Interp::NEXT_CALL_ID)));
        w.u(match state::state(h) {
            RunState::Done => 1,
            RunState::Threw => 2,
            _ => 0,
        });
        w.u(match state::outcome(h) {
            RunState::Done => 1,
            RunState::Threw => 2,
            _ => 0,
        });
        self.value(w, a.get(i, Interp::RESULT));
        self.value(w, a.get(i, Interp::THROW));
        self.reference(w, a.get(i, Interp::REJECTIONS));
        w.u(u64::from(a.get(i, Interp::REPORTED)));
        self.reference(w, a.get(0, Roots::HOST_CALLS));
        match host {
            // The handles the host holds for the run, by name; not its handle table.
            Some(host) => {
                w.u(2 * host.handles.len() as u64);
                for (n, v) in &host.handles {
                    let s = self.str_lit(n);
                    w.byte(V_STR);
                    w.str_ref(s);
                    self.value(w, *v);
                }
                let s = self.str_lit(&host.data);
                w.byte(V_STR);
                w.str_ref(s);
            }
            None => self.reference(w, a.get(0, Roots::HANDLES)),
        }
        for k in 0..3 {
            w.byte(V_REC);
            w.rec_ref(k);
        }
        let rs = intrinsic(h, Intrinsic::RandomState).as_internal().unwrap();
        let n = h.value_array_len(rs);
        w.u(u64::from(n));
        for k in 0..n {
            self.value(w, h.value_array_get(rs, k));
        }
        for i in [
            Intrinsic::Clock,
            Intrinsic::TimeZoneOffset,
            Intrinsic::GlobalLexEpoch,
            Intrinsic::KeptObjects,
            Intrinsic::Registries,
            Intrinsic::Modules,
        ] {
            let v = intrinsic(h, i);
            self.value(w, v);
        }
        w.u(u64::from(h.frozen_builtins()));
        w.u(u64::from(h.weak_refs_strong()));
        self.path.pop();
        // Realm intrinsics above the prefix that the snapshot does not know.
        let r = crate::interp::realm::realm(h);
        let count = (h.cell_size(r) - Realm::INTRINSICS.offset) / 8;
        for k in 0..count {
            let v: Value = a.read(Realm::INTRINSICS.at(r, k));
            if v.ref_offset().is_some_and(|o| o >= self.base_end)
                && !REALM_PER_VM.contains(&k)
                && !REALM_CACHES.contains(&k)
            {
                self.refuse(format!("realm intrinsic {k} is per-VM state"));
            }
        }
    }

    pub fn function_entry(&mut self, w: &mut Writer, t: u32) {
        let h = self.h;
        let d = template::debug_info(h, t);
        let s = d.script.get().map_or(0, |s| self.script_index(s));
        w.u(u64::from(s));
        w.u(u64::from(d.src_start));
        w.u(u64::from(d.src_end));
        // A function with a stored frame needs its bytecode; the others only their
        // closures' interface.
        if self.pinned.contains(&t) {
            w.byte(1);
            w.u64_raw(super::hash::template_hash(h, t));
        } else {
            w.byte(0);
            w.u64_raw(super::hash::interface_hash(h, t));
        }
        // The template objects its tagged templates made (state of the run), by site.
        let consts = template::consts(h, t);
        let mut sites: Vec<(u32, Value)> = Vec::new();
        for insn in crate::bytecode::decode::insns(template::code(h, t)).flatten() {
            if insn.op == crate::bytecode::Op::GetTemplateObject {
                let i = insn.imm(0);
                if let Some(v) = consts.get(i as usize).filter(|v| v.as_object().is_some())
                    && !sites.iter().any(|s| s.0 == i)
                {
                    sites.push((i, *v));
                }
            }
        }
        w.u(sites.len() as u64);
        for (i, v) in sites {
            w.u(u64::from(i));
            self.value(w, v);
        }
    }
}

pub(super) const V_UNDEF: u8 = 0;
pub(super) const V_NULL: u8 = 1;
pub(super) const V_FALSE: u8 = 2;
pub(super) const V_TRUE: u8 = 3;
pub(super) const V_INT: u8 = 4;
pub(super) const V_F64: u8 = 5;
pub(super) const V_HOLE: u8 = 6;
pub(super) const V_UNINIT: u8 = 7;
pub(super) const V_STR: u8 = 8;
pub(super) const V_REC: u8 = 9;
pub(super) const V_REC_INTERNAL: u8 = 10;
pub(super) const V_BASE: u8 = 11;
pub(super) const V_BASE_INTERNAL: u8 = 12;
pub(super) const V_FN: u8 = 13;
pub(super) const V_SCRIPT: u8 = 14;
/// A host value or function (D3720): its host entry.
pub(super) const V_HOST: u8 = 15;

pub(super) const PROP_ACCESSOR: u64 = 8;

/// The realm intrinsics the `Vm` record carries, and the caches it drops.
pub(super) const REALM_PER_VM: &[u32] = &[
    Intrinsic::Global as u32,
    Intrinsic::SymbolRegistry as u32,
    Intrinsic::RandomState as u32,
    Intrinsic::Clock as u32,
    Intrinsic::TimeZoneOffset as u32,
    Intrinsic::GlobalLexEpoch as u32,
    Intrinsic::KeptObjects as u32,
    Intrinsic::Registries as u32,
    Intrinsic::Modules as u32,
];
pub(super) const REALM_CACHES: &[u32] = &[
    // Per VM, but the base's functions: the importer has its own.
    Intrinsic::PreludeFunctions as u32,
    Intrinsic::RegExpCache as u32,
    Intrinsic::UnitStrings as u32,
];

/// A host call's function id, which a host's snapshot writes by identity (D3720).
pub(super) fn is_host_call_fn(spec: &KindSpec, f: &FieldInfo) -> bool {
    spec.kind == CellKind::HostCall && f.name == "FN"
}

pub(super) fn natural_tag(k: CellKind) -> u16 {
    match k {
        CellKind::Object => tag::OBJECT,
        CellKind::Symbol => tag::SYMBOL,
        CellKind::BigInt => tag::BIGINT,
        CellKind::Str8 | CellKind::Str16 | CellKind::Rope | CellKind::Slice => tag::STRING,
        _ => tag::INTERNAL,
    }
}

pub(super) fn field_ty(t: FieldTy) -> &'static str {
    match t {
        FieldTy::Value => "value",
        FieldTy::Ref => "ref",
        FieldTy::I32 => "int",
        FieldTy::F64 => "f64",
        _ => "uint",
    }
}

/// A header flags field's type: its flag set by name (`FLAGSETS` has the names).
pub(super) fn flags_ty(set: &str, _f: &[(&str, u16)]) -> String {
    format!("flags:{set}")
}

/// The flag sets of the header flags fields and the named-constant sets the types use
/// (D3710), by member name in index order: written once per snapshot.
pub(super) fn flagsets(consts: &[&'static ConstSet]) -> Vec<(&'static str, Vec<&'static str>)> {
    let names = |f: &'static [(&'static str, u16)]| f.iter().map(|(n, _)| *n).collect();
    let mut v: Vec<(&'static str, Vec<&'static str>)> =
        alloc::vec![("Object", names(schema::OBJECT_FLAGS))];
    for k in schema::KINDS {
        if !k.flags.is_empty() {
            v.push((k.kind.layout().name, names(k.flags)));
        }
    }
    for c in consts {
        let mut names: Vec<&'static str> = c.members.iter().map(|(n, _)| *n).collect();
        if skew() == Skew::Reverse {
            names.reverse();
        }
        if let Skew::RenameMember(m) = skew() {
            for n in &mut names {
                if *n == m {
                    *n = "renamed";
                }
            }
        }
        v.push((c.name, names));
    }
    v
}

/// The slots of an object as a snapshot writes them: its class's, or for a record (or an
/// iterator helper) its kind's schema (D3710).
#[derive(Clone, Copy)]
pub(super) struct Slots {
    pub class: ObjectClass,
    pub class_name: &'static str,
    /// The record kind's schema name.
    pub record: Option<&'static str>,
    pub slots: &'static [(&'static str, SlotTy)],
    pub rest: Option<&'static str>,
}

impl Slots {
    pub fn class(c: &'static ClassSpec) -> Slots {
        Slots {
            class: c.class,
            class_name: c.name,
            record: None,
            slots: c.slots,
            rest: c.rest,
        }
    }

    pub fn record(r: &'static RecordSchema) -> Slots {
        Slots {
            class: r.class,
            class_name: r.class.slot_table().name,
            record: Some(r.name),
            slots: r.fields,
            rest: r.rest,
        }
    }

    /// What has these slots, for refusals: `Promise objects`, `Record ElementRecord`.
    pub fn what(&self) -> String {
        match self.record {
            Some(r) => format!("{} {r}", self.class_name),
            None => format!("{} objects", self.class_name),
        }
    }

    /// `Object.Promise`, `Object.Record.ElementRecord`.
    pub fn type_name(&self) -> String {
        match self.record {
            Some(r) => format!("Object.{}.{r}", self.class_name),
            None => format!("Object.{}", self.class_name),
        }
    }
}

fn read_uint(a: &crate::heap::Arena, at: u32, t: FieldTy) -> u64 {
    match t {
        FieldTy::U8 => u64::from(a.read::<u8>(at)),
        FieldTy::U16 => u64::from(a.read::<u16>(at)),
        FieldTy::U32 => u64::from(a.read::<u32>(at)),
        FieldTy::U64 => a.read::<u64>(at),
        _ => 0,
    }
}
