//! Inspecting a snapshot (D3760): what a run that waits holds, for people (`describe`,
//! `djs describe`, the docs site) and for tools (`Inspection::json`).
//!
//! The format describes itself, so the bytes give every record with its fields by name.
//! What they do not hold is debug information: the names of a frame's locals, captures
//! and parameters, and where a pc is in the source. The inspector compiles the stored
//! scripts again, in a heap of its own, as a load does, and finds each function by its
//! span (and its hash, when two share one).
//!
//! The model is the run as a tree from its open host calls: each call, the activation
//! that awaits it, and up the await chain (promises, `Promise.all` elements, `then`
//! callbacks) to the run's result; then what a loading build must agree on, and the bytes
//! per section. Record ids (`#14`) are positions in the snapshot's record list: stable
//! within one snapshot, not across snapshots.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

mod json;
mod view;

use super::Refusal;
use super::export::PROP_ACCESSOR;
use super::import::{F, HostEntry, P, Snap, Ty, V, fields, string_of};
use super::wire::Reader;
use crate::heap::cells::{FuncTemplate, U32Array};
use crate::heap::{Atom, Heap};
use crate::value::Value;
use view::Node;

/// How much of the data the text shows: nesting, items per container, string length,
/// and the open calls shown in full.
const DEPTH: usize = 3;
const BREADTH: usize = 8;
const STRING: usize = 48;
const CALLS: usize = 16;
/// The text's line width, for packing bindings.
const WIDTH: usize = 88;
/// The longest await chain followed (a guard: chains end at the run's result).
const CHAIN: usize = 64;

/// What the inspector knows of a function from compiling its script again.
#[derive(Default, Clone)]
struct FnDebug {
    /// The function's name; empty for an anonymous one.
    name: String,
    /// The script's top-level code.
    top: bool,
    locals: Vec<String>,
    captures: Vec<String>,
    /// The simple parameters' names, by index.
    params: Vec<String>,
    /// The line table (`bytecode::lookup_line`).
    lines: Vec<u8>,
    /// Line and column of the span's start.
    at: (u32, u32),
    /// Found in a compile of the stored source.
    known: bool,
}

/// A link of an await chain: what waits on a promise.
#[derive(Clone)]
enum Link {
    /// A suspended async function or generator: record `g`, its frame read.
    Activation(Box<Frame>),
    /// An element function of a promise combinator (`Promise.all` and the like).
    Element {
        record: u32,
        combinator: String,
        index: Option<i64>,
        values: Option<V>,
        remaining: Option<i64>,
        promise: Option<u32>,
        waiters: Vec<Link>,
    },
    /// Another reaction: a `then` callback, a resolving function, or nothing to call.
    Reaction {
        handler: V,
        promise: Option<u32>,
        waiters: Vec<Link>,
    },
    /// Nothing waits on the promise.
    End {
        promise: u32,
        result: bool,
        held: Option<String>,
    },
    /// The promise's waiters were shown already.
    Seen { promise: u32 },
}

/// A suspended activation.
#[derive(Clone)]
struct Frame {
    record: u32,
    /// The function's index in the snapshot's function list, when it is code of a script.
    function: Option<usize>,
    callee: V,
    pc: i64,
    pos: Option<(u32, u32)>,
    this: V,
    /// The arguments, named when the function's parameters are simple.
    args: Vec<(Option<String>, V)>,
    /// Locals by name (`<temp>` and other unnamed slots kept with their debug names).
    locals: Vec<(String, V)>,
    /// The closure's captured bindings by name: their boxes.
    captures: Vec<(String, V)>,
    /// The operand stack: what the expression it waits in holds so far.
    stack: Vec<V>,
    /// The frame's slots when its function is not known (no source to compile).
    slots: Vec<V>,
    promise: Option<u32>,
    waiters: Vec<Link>,
}

/// An open host call and what waits on it.
struct Call {
    id: u64,
    record: u32,
    function: String,
    args: Vec<V>,
    promise: Option<u32>,
    waiters: Vec<Link>,
}

/// A snapshot, inspected: its parts and the run as a tree ([`inspect`]).
pub struct Inspection<'a> {
    snap: Snap<'a>,
    size: usize,
    /// The records' byte ranges, and the `Vm` record's.
    spans: Vec<(usize, usize)>,
    root_span: (usize, usize),
    functions: Vec<FnDebug>,
    /// Which records hold each record (`u32::MAX`: the `Vm` record).
    held_by: Vec<Vec<u32>>,
    calls: Vec<Call>,
    /// Suspended activations no open call leads to.
    others: Vec<Link>,
    /// The run in JavaScript terms: a node per open call, then one per activation no
    /// call leads to ([`view`]).
    view: Vec<Node>,
    others_view: Vec<Node>,
    /// A role sentence per record the tree explains (the promise a call returned, a
    /// frame), for the records' notes.
    roles: ById<String>,
}

/// A snapshot inspected: the model [`Inspection::text`] and [`Inspection::json`] render.
pub fn inspect(bytes: &[u8]) -> Result<Inspection<'_>, Refusal> {
    let snap = Snap::parse(bytes)?;
    let (spans, root_span) = record_spans(bytes, &snap);
    let functions = debug_info(&snap);
    let mut i = Inspection {
        size: bytes.len(),
        spans,
        root_span,
        functions,
        held_by: Vec::new(),
        calls: Vec::new(),
        others: Vec::new(),
        view: Vec::new(),
        others_view: Vec::new(),
        roles: ById::default(),
        snap,
    };
    i.held_by = i.holders();
    i.tree();
    i.build_view();
    Ok(i)
}

/// The snapshot described for people: [`inspect`]'s text.
pub fn describe(bytes: &[u8]) -> Result<String, Refusal> {
    inspect(bytes).map(|i| i.text())
}

/// The records' byte ranges: the records section read again, as `Snap::parse` read it.
fn record_spans(bytes: &[u8], s: &Snap<'_>) -> (Vec<(usize, usize)>, (usize, usize)) {
    let mark = |n: &str| s.marks.iter().find(|m| m.0 == n).map_or(0, |m| m.1);
    let mut r = Reader::new(bytes);
    r.at = mark("records");
    r.reset_refs();
    let mut out = Vec::with_capacity(s.records.len());
    if r.u().is_ok() {
        for _ in 0..s.records.len() {
            let from = r.at;
            let ok = r
                .u32()
                .ok()
                .and_then(|t| s.types.get(t as usize))
                .is_some_and(|ty| fields(&mut r, &ty.fields).is_ok());
            if !ok {
                break;
            }
            out.push((from, r.at));
        }
    }
    out.resize(s.records.len(), (0, 0));
    (out, (mark("vm"), mark("end")))
}

/// The debug information of every function of the snapshot, from compiling its scripts.
fn debug_info(s: &Snap<'_>) -> Vec<FnDebug> {
    let mut out = alloc::vec![FnDebug::default(); s.functions.len()];
    let Ok(mut heap) = Heap::new() else {
        return out;
    };
    let text = |i: u32| {
        s.strings
            .get(i as usize)
            .map_or_else(String::new, |&(two, d)| string_of(two, d))
    };
    for (k, &(name, params, bits, src)) in s.scripts.iter().enumerate() {
        let Ok(src) = src else { continue };
        let name = text(name);
        let params = text(params);
        let ps: Vec<&str> = if params.is_empty() {
            Vec::new()
        } else {
            params.split(',').collect()
        };
        let opts = crate::compiler::CompileOptions::from_bits(&name, &ps, bits);
        let Ok(root) = crate::compiler::compile(&mut heap, src, &opts) else {
            continue;
        };
        let h = &heap;
        // (Not a map: its generic code would be shared with the loader's, and the shipped
        // module's inlining would change though this code is not in it.)
        let mut spans: Vec<(u32, u32, u32)> = Vec::new();
        let mut stack = alloc::vec![root.0];
        while let Some(t) = stack.pop() {
            let d = crate::bytecode::template::debug_info(h, t);
            spans.push((d.src_start, d.src_end, t));
            for c in 0..crate::bytecode::template::child_count(h, t) {
                if let Some(c) = crate::bytecode::template::child(h, t, c).and_then(|r| r.get()) {
                    stack.push(c);
                }
            }
        }
        let starts = line_starts(src);
        for (f, &(script, start, end, pinned, hash, _)) in s.functions.iter().enumerate() {
            if script as usize != k {
                continue;
            }
            let ts: Vec<u32> = spans
                .iter()
                .filter(|x| (x.0, x.1) == (start, end))
                .map(|x| x.2)
                .collect();
            // Two templates of one span (a class's field initializers): the interface hash
            // tells them apart. (Not the bytecode hash: it names builtins through the
            // realm, which this heap has not.)
            let t = match ts.as_slice() {
                [] => continue,
                [t] => *t,
                _ => ts
                    .iter()
                    .copied()
                    .find(|&t| !pinned && super::hash::interface_hash(h, t) == hash)
                    .unwrap_or(ts[0]),
            };
            out[f] = template_debug(h, t, t == root.0, src, &starts, start);
        }
    }
    out
}

fn template_debug(
    h: &Heap,
    t: u32,
    top: bool,
    src: &[u8],
    starts: &[usize],
    start: u32,
) -> FnDebug {
    let a = h.arena();
    let d = crate::bytecode::template::debug_info(h, t);
    let name = Atom::from_id(a.get(t, FuncTemplate::NAME));
    let name = if name.is_null() {
        String::new()
    } else {
        h.atom_name(name)
    };
    let names: Vec<String> = match d.names.get() {
        Some(n) => (0..a.get(n, U32Array::LEN))
            .map(|i| {
                let id: u32 = a.read(U32Array::DATA.at(n, i));
                if id == Atom::NULL.id() {
                    String::new()
                } else {
                    h.atom_name(Atom::from_id(id))
                }
            })
            .collect(),
        None => Vec::new(),
    };
    let locals = (a.get(t, FuncTemplate::LOCALS) as usize).min(names.len());
    let caps = (a.get(t, FuncTemplate::CAPTURES) as usize).min(names.len() - locals);
    FnDebug {
        name,
        top,
        locals: names[..locals].to_vec(),
        captures: names[locals..locals + caps].to_vec(),
        params: names[locals + caps..].to_vec(),
        lines: crate::bytecode::template::bytes_of(h, d.lines).to_vec(),
        at: line_col(src, starts, start as usize),
        known: true,
    }
}

fn line_starts(src: &[u8]) -> Vec<usize> {
    let mut v = alloc::vec![0];
    v.extend(
        src.iter()
            .enumerate()
            .filter(|(_, c)| **c == b'\n')
            .map(|(i, _)| i + 1),
    );
    v
}

fn line_col(src: &[u8], starts: &[usize], pos: usize) -> (u32, u32) {
    let line = starts.partition_point(|&s| s <= pos).max(1);
    let from = starts[line - 1];
    let col = core::str::from_utf8(&src[from..pos.min(src.len()).max(from)])
        .map_or(pos - from, |s| s.chars().count());
    (line as u32, col as u32 + 1)
}

/// A set of record ids. (Not a `BTreeSet`: its generic code would be shared with the
/// loader's, and the shipped module's inlining would change though this code is not in
/// it.)
#[derive(Default)]
pub(super) struct Marks(Vec<bool>);

impl Marks {
    /// Marks `r`; whether it was not marked yet.
    pub fn insert(&mut self, r: u32) -> bool {
        let r = r as usize;
        if self.0.len() <= r {
            self.0.resize(r + 1, false);
        }
        !core::mem::replace(&mut self.0[r], true)
    }

    pub fn contains(&self, r: u32) -> bool {
        self.0.get(r as usize).copied().unwrap_or(false)
    }
}

/// Something per record id (a map from ids, kept as a list for the reason of [`Marks`]).
pub(super) struct ById<T>(Vec<Option<T>>);

impl<T> Default for ById<T> {
    fn default() -> Self {
        ById(Vec::new())
    }
}

impl<T> ById<T> {
    pub fn get(&self, r: u32) -> Option<&T> {
        self.0.get(r as usize).and_then(|x| x.as_ref())
    }

    pub fn insert(&mut self, r: u32, v: T) {
        let r = r as usize;
        if self.0.len() <= r {
            self.0.resize_with(r + 1, || None);
        }
        self.0[r] = Some(v);
    }

    /// Sets `r` unless it has a value.
    pub fn insert_new(&mut self, r: u32, v: impl FnOnce() -> T) {
        if self.get(r).is_none() {
            self.insert(r, v());
        }
    }
}

/// Record type names of the run's structure.
const PROMISE: &str = "Object.Promise";
const GENERATOR: &str = "Object.Generator";
const FUNCTION: &str = "Object.Function";
const BOX: &str = "BoxCell";

impl<'a> Inspection<'a> {
    // ---------------------------------------------------------------- reading

    fn text_of(&self, i: u32) -> String {
        self.snap
            .strings
            .get(i as usize)
            .map_or_else(String::new, |&(two, d)| string_of(two, d))
    }

    fn type_name(&self, r: u32) -> &str {
        self.snap
            .records
            .get(r as usize)
            .and_then(|(t, _)| self.snap.types.get(*t as usize))
            .map_or("", |t| t.name.as_str())
    }

    /// Field `name` of record `r`.
    fn field(&self, r: u32, name: &str) -> Option<&F> {
        let (t, fs) = self.snap.records.get(r as usize)?;
        let ty = self.snap.types.get(*t as usize)?;
        let k = ty.fields.iter().position(|(n, _)| n == name)?;
        fs.get(k)
    }

    fn root_field(&self, name: &str) -> Option<&F> {
        let ty = self.snap.types.get(self.snap.root.0 as usize)?;
        let k = ty.fields.iter().position(|(n, _)| n == name)?;
        self.snap.root.1.get(k)
    }

    fn value(&self, r: u32, name: &str) -> Option<V> {
        match self.field(r, name)? {
            F::V(v) => Some(*v),
            _ => None,
        }
    }

    fn int(&self, r: u32, name: &str) -> Option<i64> {
        match self.field(r, name)? {
            F::V(V::Imm(v)) => v.as_int().map(i64::from),
            F::U(u) => Some(*u as i64),
            F::I(i) => Some(*i),
            _ => None,
        }
    }

    /// The elements of a `ValueArray` record `v` refers to.
    fn list(&self, v: V) -> Vec<V> {
        match v {
            V::Rec(r, _) if self.type_name(r) == "ValueArray" => match self.field(r, "VALUES") {
                Some(F::Tail(t)) => t
                    .iter()
                    .filter_map(|e| match e.first() {
                        Some(F::V(v)) => Some(*v),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// A named constant's member name (an enum field).
    fn constant(&self, r: u32, name: &str) -> String {
        let (Some((t, _)), Some(F::Const(k))) =
            (self.snap.records.get(r as usize), self.field(r, name))
        else {
            return String::new();
        };
        let ty = &self.snap.types[*t as usize];
        match ty.fields.iter().find(|(n, _)| n == name) {
            Some((_, Ty::Enum(_, names))) => names.get(*k as usize).cloned().unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn rec(v: V) -> Option<u32> {
        match v {
            V::Rec(r, _) => Some(r),
            _ => None,
        }
    }

    /// The promise `v` is or a capability record holds.
    fn promise_of(&self, v: V) -> Option<u32> {
        let r = Self::rec(v)?;
        match self.type_name(r) {
            PROMISE => Some(r),
            t if t.starts_with("Object.Record.") => {
                self.value(r, "promise").and_then(|p| self.promise_of(p))
            }
            _ => None,
        }
    }

    fn host_name(&self, k: u32) -> String {
        match self.snap.host.get(k as usize) {
            Some(&HostEntry::Root(n)) | Some(&HostEntry::Function(n)) => self.text_of(n),
            Some(&HostEntry::Member(p, n)) if p < k => {
                format!("{}.{}", self.host_name(p), self.text_of(n))
            }
            _ => String::from("?"),
        }
    }

    /// The name of the host function object with id `id` the snapshot holds, if any.
    fn host_function_named(&self, id: u64) -> Option<String> {
        (0..self.snap.records.len() as u32).find_map(|r| {
            if self.type_name(r) != "Object.HostFunction"
                || self.int(r, "id").map(|x| x as u64) != Some(id)
            {
                return None;
            }
            match self.field(r, "props") {
                Some(F::Props(ps)) => ps
                    .iter()
                    .find(|p| matches!(p.key, V::Str(s) if self.text_of(s) == "name"))
                    .and_then(|p| match p.a {
                        V::Str(s) => Some(self.text_of(s)),
                        _ => None,
                    }),
                _ => None,
            }
        })
    }

    fn builtin(&self, i: u32) -> String {
        self.snap
            .bases
            .get(i as usize)
            .map_or_else(|| String::from("?"), |&s| self.text_of(s))
    }

    fn intrinsic(&self, i: u32) -> String {
        self.snap
            .intrinsics
            .get(i as usize)
            .map_or_else(|| String::from("?"), |&s| self.text_of(s))
    }

    fn script_name(&self, k: u32) -> String {
        self.snap
            .scripts
            .get(k as usize)
            .map_or_else(String::new, |s| self.text_of(s.0))
    }

    /// Function `f` as people call it: `fetchPage`, `the body`, `<anonymous at 3:9>`.
    fn fn_name(&self, f: usize) -> String {
        let d = &self.functions[f];
        let (script, start, end, ..) = self.snap.functions[f];
        if !d.name.is_empty() {
            d.name.clone()
        } else if d.top {
            String::from(
                match self.snap.scripts.get(script as usize).map(|s| s.2 & 3) {
                    Some(1) => "the body",
                    Some(3) => "the module",
                    _ => "the script",
                },
            )
        } else if d.known {
            format!("<anonymous at {}:{}>", d.at.0, d.at.1)
        } else {
            format!(
                "<function at {start}..{end} of {}>",
                self.script_name(script)
            )
        }
    }

    /// `waiting`, `done` or `threw`: a body's run is done when its promise settled, not
    /// when its call returned that promise.
    fn run_state(&self) -> &'static str {
        let result = match self.root_field("result") {
            Some(F::V(V::Rec(r, _))) if self.type_name(*r) == PROMISE => Some(*r),
            _ => None,
        };
        match (
            self.root_field("outcome"),
            result.map(|r| self.constant(r, "state")),
        ) {
            (_, Some(s)) if s == "pending" => "waiting",
            (_, Some(s)) if s == "rejected" => "threw",
            (Some(F::U(1)), _) => "done",
            (Some(F::U(2)), _) => "threw",
            _ if !self.calls.is_empty() => "waiting",
            (_, Some(_)) => "done",
            _ => "waiting",
        }
    }

    fn source(&self, script: u32) -> Option<&'a [u8]> {
        self.snap
            .scripts
            .get(script as usize)
            .and_then(|s| s.3.ok())
    }

    // ---------------------------------------------------------------- the tree

    fn holders(&self) -> Vec<Vec<u32>> {
        let mut by = alloc::vec![Vec::new(); self.snap.records.len()];
        let mut add = |from: u32, fs: &[F]| {
            let mut refs = Vec::new();
            walk_refs(fs, &mut |r| refs.push(r));
            refs.sort_unstable();
            refs.dedup();
            for r in refs {
                if let Some(v) = by.get_mut(r as usize) {
                    v.push(from);
                }
            }
        };
        for (k, (_, fs)) in self.snap.records.iter().enumerate() {
            add(k as u32, fs);
        }
        add(u32::MAX, &self.snap.root.1);
        by
    }

    fn tree(&mut self) {
        let mut seen = Marks::default();
        let mut shown = Marks::default();
        let mut calls = Vec::new();
        let list = self.root_field("hostCalls").and_then(|f| match f {
            F::V(v) => Some(*v),
            _ => None,
        });
        for c in list.map(|l| self.list(l)).unwrap_or_default() {
            let Some(r) = Self::rec(c).filter(|&r| self.type_name(r) == "HostCall") else {
                continue;
            };
            let function = match self.field(r, "FN") {
                Some(F::HostFn(k)) => self.host_name(*k),
                // A snapshot that is no host's: the host function by its id.
                Some(F::U(id)) => self
                    .host_function_named(*id)
                    .unwrap_or_else(|| format!("host function {id}")),
                _ => String::from("?"),
            };
            let args = self
                .value(r, "ARGS")
                .map(|a| self.list(a))
                .unwrap_or_default();
            let promise = self.value(r, "PROMISE").and_then(|p| self.promise_of(p));
            let waiters = promise
                .map(|p| self.waiters(p, &mut seen, &mut shown, 0))
                .unwrap_or_default();
            calls.push(Call {
                id: self.int(r, "ID").unwrap_or(0) as u64,
                record: r,
                function,
                args,
                promise,
                waiters,
            });
        }
        calls.sort_by_key(|c| c.id);
        // Activations no open call leads to: a generator stopped at a `yield`, an async
        // function awaiting a promise nothing settles.
        let mut others = Vec::new();
        for r in 0..self.snap.records.len() as u32 {
            if self.type_name(r) == GENERATOR
                && !shown.contains(r)
                && matches!(self.value(r, "frame"), Some(V::Rec(..)))
            {
                let f = self.frame(r, &mut seen, &mut shown, 0);
                others.push(Link::Activation(Box::new(f)));
            }
        }
        self.calls = calls;
        self.others = others;
    }

    /// What waits on promise `p`.
    fn waiters(&self, p: u32, seen: &mut Marks, shown: &mut Marks, depth: usize) -> Vec<Link> {
        if !seen.insert(p) || depth > CHAIN {
            return alloc::vec![Link::Seen { promise: p }];
        }
        let reactions = self
            .value(p, "reactions")
            .map(|r| self.list(r))
            .unwrap_or_default();
        let mut out = Vec::new();
        for t in reactions.chunks(3) {
            let (cap, on) = (t[0], t.get(1).copied().unwrap_or(V::Imm(Value::UNDEFINED)));
            let next = |s: &Self, seen: &mut Marks, shown: &mut Marks, q: Option<u32>| {
                q.map(|q| s.waiters(q, seen, shown, depth + 1))
                    .unwrap_or_default()
            };
            match Self::rec(on).map(|r| (r, self.type_name(r))) {
                Some((g, GENERATOR)) => out.push(Link::Activation(Box::new(
                    self.frame(g, seen, shown, depth),
                ))),
                Some((b, "Object.BuiltinClosure")) => {
                    let data = self.value(b, "data").unwrap_or(V::Imm(Value::UNDEFINED));
                    let kind = match self.field(b, "builtin") {
                        Some(F::Intr(i)) => self.intrinsic(*i),
                        _ => String::new(),
                    };
                    match Self::rec(data)
                        .filter(|&e| self.type_name(e) == "Object.Record.ElementRecord")
                    {
                        Some(e) => {
                            let promise =
                                self.value(e, "capability").and_then(|c| self.promise_of(c));
                            out.push(Link::Element {
                                record: b,
                                combinator: combinator(&kind),
                                index: self.int(e, "index"),
                                values: self.value(e, "values"),
                                remaining: self
                                    .value(e, "remaining")
                                    .and_then(Self::rec)
                                    .and_then(|c| self.int(c, "count")),
                                promise,
                                waiters: next(self, seen, shown, promise),
                            });
                        }
                        None => {
                            let promise = self.promise_of(data).or_else(|| self.promise_of(cap));
                            out.push(Link::Reaction {
                                handler: on,
                                promise,
                                waiters: next(self, seen, shown, promise),
                            });
                        }
                    }
                }
                _ => {
                    let promise = self.promise_of(cap);
                    out.push(Link::Reaction {
                        handler: on,
                        promise,
                        waiters: next(self, seen, shown, promise),
                    });
                }
            }
        }
        if out.is_empty() {
            let result = self
                .root_field("result")
                .is_some_and(|f| matches!(f, F::V(V::Rec(r, _)) if *r == p));
            let held = match self.root_field("named") {
                Some(F::Vs(named)) => named
                    .chunks(2)
                    .find(|kv| matches!(kv.get(1), Some(V::Rec(r, _)) if *r == p))
                    .and_then(|kv| match kv[0] {
                        V::Str(s) => Some(self.text_of(s)),
                        _ => None,
                    }),
                _ => None,
            };
            out.push(Link::End {
                promise: p,
                result,
                held,
            });
        }
        out
    }

    /// Activation `g`'s frame, read, and what waits on its promise.
    fn frame(&self, g: u32, seen: &mut Marks, shown: &mut Marks, depth: usize) -> Frame {
        shown.insert(g);
        let slots = self
            .value(g, "frame")
            .map(|f| self.list(f))
            .unwrap_or_default();
        let pc = self.int(g, "pc").unwrap_or(0);
        let hp = self.int(g, "hp").unwrap_or(0).max(2) as usize;
        let callee = slots.first().copied().unwrap_or(V::Imm(Value::UNDEFINED));
        let closure = Self::rec(callee).filter(|&c| self.type_name(c) == FUNCTION);
        let function = closure
            .and_then(|c| match self.field(c, "template") {
                Some(F::Fn(f)) => Some(*f as usize),
                _ => None,
            })
            .filter(|&f| f < self.functions.len());
        let d = function.map(|f| &self.functions[f]).filter(|d| d.known);
        let promise = self.value(g, "promise").and_then(|p| self.promise_of(p));
        let mut fr = Frame {
            record: g,
            function,
            callee,
            pc,
            pos: d.and_then(|d| {
                crate::bytecode::lookup_line(&d.lines, (pc as u32).saturating_sub(1))
            }),
            this: slots.get(1).copied().unwrap_or(V::Imm(Value::UNDEFINED)),
            args: Vec::new(),
            locals: Vec::new(),
            captures: Vec::new(),
            stack: Vec::new(),
            slots: Vec::new(),
            promise,
            waiters: Vec::new(),
        };
        match d {
            Some(d) => {
                let args = slots.get(2..hp.min(slots.len())).unwrap_or(&[]);
                fr.args = args
                    .iter()
                    .enumerate()
                    .map(|(k, v)| (d.params.get(k).filter(|n| !n.is_empty()).cloned(), *v))
                    .collect();
                let lp = hp + crate::interp::frame::HEADER as usize;
                let locals = slots.get(lp..).unwrap_or(&[]);
                let n = d.locals.len().min(locals.len());
                fr.locals = d
                    .locals
                    .iter()
                    .cloned()
                    .zip(locals[..n].iter().copied())
                    .collect();
                fr.stack = locals[n..].to_vec();
                let caps = closure.and_then(|c| match self.field(c, "captures") {
                    Some(F::Vs(v)) => Some(v.clone()),
                    _ => None,
                });
                fr.captures = d
                    .captures
                    .iter()
                    .cloned()
                    .zip(caps.unwrap_or_default())
                    .collect();
            }
            None => fr.slots = slots.get(1..).unwrap_or(&[]).to_vec(),
        }
        fr.waiters = promise
            .map(|p| self.waiters(p, seen, shown, depth + 1))
            .unwrap_or_default();
        fr
    }
}

/// The combinator an element function belongs to, by the builtin closure's kind.
fn combinator(kind: &str) -> String {
    String::from(match kind {
        "PromiseAllResolveElement" => "Promise.all",
        "PromiseAllSettledResolveElement" | "PromiseAllSettledRejectElement" => {
            "Promise.allSettled"
        }
        "PromiseAnyRejectElement" => "Promise.any",
        k => return k.to_string(),
    })
}

/// Every record field `fs` refers to.
fn walk_refs(fs: &[F], f: &mut impl FnMut(u32)) {
    let mut v = |x: &V| {
        if let V::Rec(r, _) = x {
            f(*r);
        }
    };
    for x in fs {
        match x {
            F::V(x) => v(x),
            F::Vs(xs) => xs.iter().for_each(&mut v),
            F::Props(ps) => {
                for p in ps {
                    v(&p.key);
                    v(&p.a);
                    v(&p.b);
                }
            }
            F::Tail(rows) => {
                for row in rows {
                    for x in row {
                        match x {
                            F::V(x) => v(x),
                            F::Vs(xs) => xs.iter().for_each(&mut v),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
pub(super) fn view_bytes(n: usize) -> String {
    view::bytes(n)
}
