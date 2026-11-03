//! Dumps (0167, docs/functions-as-code.md): a VM's own bindings written as a JavaScript
//! Script that recreates them when run, the semantic counterpart of an image.
//!
//! - Builtins are never written, only referenced: by their canonical path on the base
//!   image (`Array.prototype.map`; D2371's names), or through the loader's
//!   `$dump.builtin(path)` for the ones no expression reaches (`%ArrayIteratorPrototype%`).
//! - Functions are their source text. Closures that share captured variables are made by
//!   one factory per group, `$dump.s1 = (() => { let count = 2; return { inc, get }; })()`,
//!   so the text stays verbatim and the sharing stays shared. A captured value that
//!   closes a cycle is set after the data through the factory's `$set`.
//! - Data are literals or constructor calls; an object reached twice or on a cycle is
//!   made once as `$dump.nN`, others inline. Links that close cycles, non-default
//!   property attributes and non-extensible objects are patched at the end.
//! - Live things are refused with the path of the value: pending promises, suspended
//!   generators, proxies, weak collections, host functions the host does not re-bind by
//!   name, classes (not yet).
//!
//! The roots are top-level declarations of the Script (`let`/`const` for the global
//! lexical record, `var`/`function` for the global object), so loading a dump into a
//! VM makes the same names; `$dump` is a temporary global deleted at the end.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::Vm;
use crate::buf::PushOut;
use crate::bytecode::template::{self, func_flags};
use crate::heap::cells::{BoxCell, FuncInfo, FuncTemplate, Object as ObjectCell, U32Array};
use crate::heap::{Atom, Heap, ObjectClass, PropFlags, Property, Ref};
use crate::interp::realm::{Intrinsic, intrinsic};
use crate::value::{Kind, Obj, Value};

/// The first line of every dump: the format and its version. A dump is plain JavaScript
/// that only uses standard builtins and the helpers it defines itself, so any later djs
/// runs it; the version names the conventions tools may parse (`$dump`, the header).
pub const HEADER: &str = "// djs-dump 1";

/// What [`Vm::dump`] needs besides the VM.
pub struct DumpOptions<'a> {
    /// The base image the VM was made from: builtins are named by their paths on it.
    pub base: &'a [u8],
    /// Globals the host defines in every VM (`console`, `tools`): never dumped; values
    /// reached from them are referenced by name (`tools.read`).
    pub host: &'a [&'a str],
}

/// Why a dump was refused: one line per value, `path: reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpError(pub Vec<String>);

impl DumpError {
    pub fn message(&self) -> String {
        let mut s = String::from("cannot dump:");
        for r in &self.0 {
            s.push_str("\n  ");
            s.push_str(r);
        }
        s
    }
}

impl Vm {
    /// Dumps the global bindings `names` (all of the VM's own, if `None`: the global
    /// lexical record and the global object's properties the base does not have) and
    /// the user globals their functions read.
    pub fn dump(
        &self,
        opts: &DumpOptions<'_>,
        names: Option<&[&str]>,
    ) -> Result<String, DumpError> {
        let mut d = Dumper::new(self, opts)?;
        match names {
            Some(names) => {
                for &n in names {
                    if !d.add_global_root(n) {
                        d.refuse(n.to_string(), "no such global");
                    }
                }
            }
            None => {
                let all: Vec<String> = d.user_globals.keys().cloned().collect();
                for n in all {
                    d.add_global_root(&n);
                }
            }
        }
        d.finish()
    }

    /// Dumps explicit values as bindings `name = value` (a harness's selection). A value
    /// may be a captured variable's box (`Value::internal`), for bindings of a function
    /// scope: closures that capture it refer to it by name.
    pub fn dump_values(
        &self,
        opts: &DumpOptions<'_>,
        roots: &[(&str, Value)],
    ) -> Result<String, DumpError> {
        let mut d = Dumper::new(self, opts)?;
        for (name, v) in roots {
            d.add_root(name, Decl::Let, *v);
        }
        d.finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decl {
    Const,
    Let,
    Var,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    New,
    Busy,
    Done,
}

struct Root {
    name: String,
    decl: Decl,
    value: Value,
    state: State,
}

struct Node {
    v: Value,
    path: String,
    refs: u32,
    edges: Vec<usize>,
    cyclic: bool,
    /// The root whose value this is (its first).
    root: Option<usize>,
    state: State,
    loc: String,
    /// Closures and boxes: their scope group.
    group: usize,
}

struct Group {
    closures: Vec<usize>,
    boxes: Vec<usize>,
    state: State,
    loc: String,
}

struct Dumper<'a> {
    vm: &'a Vm,
    h: &'a Heap,
    /// Builtin cells (below `BASE_END`) by canonical path.
    builtins: BTreeMap<u32, String>,
    base_end: u32,
    host: BTreeMap<u32, String>,
    host_names: BTreeSet<String>,
    user_globals: BTreeMap<String, (Value, Decl)>,
    roots: Vec<Root>,
    root_names: BTreeMap<String, usize>,
    root_boxes: BTreeMap<u32, usize>,
    nodes: Vec<Node>,
    ids: BTreeMap<u64, usize>,
    groups: Vec<Group>,
    refusals: Vec<String>,
    /// Builtin properties the session changed: (holder expression, key, new property).
    builtin_patches: Vec<(String, Atom, Option<Property>)>,
    out: Vec<String>,
    patches: Vec<String>,
    finals: Vec<String>,
    helpers: BTreeSet<&'static str>,
    next: u32,
    depth: u32,
    visited: usize,
}

/// The realm objects `$dump.builtin` can reach without a global path, with the
/// expression that does.
const LOADER_ROOTS: &[(&str, &str)] = &[
    (
        "IteratorPrototype",
        "Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))",
    ),
    (
        "ArrayIteratorPrototype",
        "Object.getPrototypeOf([][Symbol.iterator]())",
    ),
    (
        "StringIteratorPrototype",
        "Object.getPrototypeOf(''[Symbol.iterator]())",
    ),
    (
        "MapIteratorPrototype",
        "Object.getPrototypeOf(new Map().entries())",
    ),
    (
        "SetIteratorPrototype",
        "Object.getPrototypeOf(new Set().values())",
    ),
    (
        "RegExpStringIteratorPrototype",
        "Object.getPrototypeOf(/./[Symbol.matchAll](''))",
    ),
    ("GeneratorFunction", "(function* () {}).constructor"),
    (
        "GeneratorPrototype",
        "Object.getPrototypeOf(function* () {}).prototype",
    ),
    ("AsyncFunction", "(async function () {}).constructor"),
    (
        "AsyncGeneratorFunction",
        "(async function* () {}).constructor",
    ),
    (
        "AsyncGeneratorPrototype",
        "Object.getPrototypeOf(async function* () {}).prototype",
    ),
    ("TypedArray", "Object.getPrototypeOf(Int8Array)"),
    (
        "TypedArrayPrototype",
        "Object.getPrototypeOf(Int8Array.prototype)",
    ),
    (
        "ThrowTypeError",
        "Object.getOwnPropertyDescriptor(Function.prototype, 'caller').get",
    ),
];

const HELPERS: &[(&str, &str)] = &[
    (
        "bytes",
        "$dump.bytes = (s) => {\n  const b = atob(s), u = new Uint8Array(b.length);\n  \
         for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);\n  return u.buffer;\n};",
    ),
    (
        "fn",
        "$dump.fn = (o) => {\n  const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]);\n  \
         return d.value || d.get || d.set;\n};",
    ),
    (
        "rejected",
        "$dump.rejected = (e) => {\n  const p = Promise.reject(e);\n  p.catch(() => {});\n  return p;\n};",
    ),
];

fn is_ident(s: &str) -> bool {
    const RESERVED: &[&str] = &[
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "new",
        "null",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        "let",
        "static",
        "implements",
        "interface",
        "package",
        "private",
        "protected",
        "public",
        "await",
    ];
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !RESERVED.contains(&s)
}

/// A JavaScript string literal for UTF-16 code units.
fn quote_units(units: impl Iterator<Item = u16>) -> String {
    let mut out = String::from("\"");
    for r in char::decode_utf16(units) {
        match r {
            Ok('"') => out.push_str("\\\""),
            Ok('\\') => out.push_str("\\\\"),
            Ok('\n') => out.push_str("\\n"),
            Ok('\r') => out.push_str("\\r"),
            Ok('\t') => out.push_str("\\t"),
            Ok(c @ ('\u{2028}' | '\u{2029}')) => out.push_str(&format!("\\u{:04x}", c as u32)),
            Ok(c) if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            Ok(c) => out.push_out(c),
            Err(e) => out.push_str(&format!("\\u{:04x}", e.unpaired_surrogate())),
        }
    }
    out.push_out('"');
    out
}

fn quote(s: &str) -> String {
    quote_units(s.encode_utf16())
}

fn number(n: f64) -> String {
    if n == 0.0 && n.is_sign_negative() {
        "-0".into()
    } else if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else {
        crate::num::to_string(n).as_str().into()
    }
}

/// `.key` or `["key"]`.
fn member(key: &str) -> String {
    if is_ident(key) {
        format!(".{key}")
    } else {
        format!("[{}]", quote(key))
    }
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push_out(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push_out('=');
            }
        }
    }
    out
}

/// Whether the source of a function gets its name from where it is defined (an arrow
/// or `function (...)`), rather than from its text.
fn anonymous_form(kind: u32, src: &str) -> bool {
    if kind == func_flags::KIND_ARROW {
        return true;
    }
    if kind != func_flags::KIND_NORMAL {
        return false;
    }
    let s = src.strip_prefix("async").map_or(src, str::trim_start);
    let Some(s) = s.strip_prefix("function") else {
        return false;
    };
    let s = s.trim_start();
    let s = s.strip_prefix('*').map_or(s, str::trim_start);
    s.starts_with('(')
}

/// The name a function declaration's text gives it (`function foo(`).
fn declared_name(kind: u32, src: &str) -> Option<&str> {
    if kind != func_flags::KIND_NORMAL {
        return None;
    }
    let s = src.strip_prefix("async").map_or(src, str::trim_start);
    let s = s.strip_prefix("function")?.trim_start();
    if s.starts_with('*') {
        return None;
    }
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))?;
    Some(&s[..end]).filter(|n| !n.is_empty())
}

impl<'a> Dumper<'a> {
    fn new(vm: &'a Vm, opts: &DumpOptions<'_>) -> Result<Dumper<'a>, DumpError> {
        let h = &vm.heap;
        let refuse = |m: &str| DumpError(alloc::vec![String::from(m)]);
        let base_end = h.base_end();
        if base_end == 0 || opts.base.len() < base_end as usize {
            return Err(refuse("the VM was not made from this base image"));
        }
        let base = Heap::from_bytes_trusted(opts.base.to_vec(), opts.base.len() * 2)
            .map_err(|_| refuse("the base image does not load"))?;
        let mut builtins = BTreeMap::new();
        for (cell, path) in super::registry::canonical_names(&base) {
            if cell < base_end {
                builtins.insert(cell, path);
            }
        }
        let mut d = Dumper {
            vm,
            h,
            builtins,
            base_end,
            host: BTreeMap::new(),
            host_names: opts.host.iter().map(|s| s.to_string()).collect(),
            user_globals: BTreeMap::new(),
            roots: Vec::new(),
            root_names: BTreeMap::new(),
            root_boxes: BTreeMap::new(),
            nodes: Vec::new(),
            ids: BTreeMap::new(),
            groups: Vec::new(),
            refusals: Vec::new(),
            builtin_patches: Vec::new(),
            out: Vec::new(),
            patches: Vec::new(),
            finals: Vec::new(),
            helpers: BTreeSet::new(),
            next: 0,
            depth: 0,
            visited: 0,
        };
        d.read_globals(&base);
        d.read_patches(&base);
        Ok(d)
    }

    fn refuse(&mut self, path: String, why: &str) {
        let line = format!("{path}: {why}");
        if !self.refusals.contains(&line) {
            self.refusals.push_out(line);
        }
    }

    /// The user's globals (what the base does not have), and the host's values by name.
    fn read_globals(&mut self, base: &Heap) {
        let h = self.h;
        let g = self.vm.global();
        let bg = intrinsic(base, Intrinsic::Global).as_object();
        let lex = self.vm.global_lexical();
        for k in h.own_keys(lex) {
            if let Some(Property::Data { value, flags }) = h.get_own_property(lex, k) {
                let decl = if flags.writable() {
                    Decl::Let
                } else {
                    Decl::Const
                };
                self.global_binding(h.atom_name(k), value, decl);
            }
        }
        for k in h.own_keys(g) {
            if h.atom_symbol(k).is_some() {
                continue;
            }
            let name = h.atom_name(k);
            let Some(p) = h.get_own_property(g, k) else {
                continue;
            };
            let same = bg.and_then(|bg| {
                let bk = base
                    .own_keys(bg)
                    .into_iter()
                    .find(|bk| base.atom_name(*bk) == name)?;
                Some(base.get_own_property(bg, bk)? == p)
            });
            if same == Some(true) {
                continue;
            }
            match p {
                Property::Data { value, .. } => self.global_binding(name, value, Decl::Var),
                Property::Accessor { .. } => {
                    if !self.host_names.contains(&name) {
                        self.refuse(name, "a global accessor");
                    }
                }
            }
        }
    }

    fn global_binding(&mut self, name: String, value: Value, decl: Decl) {
        if self.host_names.contains(&name) {
            self.host_values(value, name, 0);
        } else if name != "$dump" {
            self.user_globals.insert(name, (value, decl));
        }
    }

    fn host_values(&mut self, v: Value, expr: String, depth: u32) {
        let Some(o) = v.as_object() else { return };
        if o.offset() < self.base_end || self.host.contains_key(&o.offset()) {
            return;
        }
        self.host.insert(o.offset(), expr.clone());
        if depth >= 2 || self.h.object_class(o) == ObjectClass::Proxy {
            return;
        }
        for k in self.h.own_keys(o) {
            if self.h.atom_symbol(k).is_some() {
                continue;
            }
            if let Some(Property::Data { value, .. }) = self.h.get_own_property(o, k) {
                let e = format!("{expr}{}", member(&self.h.atom_name(k)));
                self.host_values(value, e, depth + 1);
            }
        }
    }

    /// Builtin properties the session changed (none with frozen builtins): compared
    /// with the base, object by object, when any word of the prefix differs.
    fn read_patches(&mut self, base: &Heap) {
        let h = self.h;
        if h.frozen_builtins() {
            return;
        }
        let start = h.cell_size(0) as usize;
        let end = self.base_end as usize;
        if h.bytes()[start..end] == base.bytes()[start..end] {
            return;
        }
        let cells: Vec<(u32, String)> =
            self.builtins.iter().map(|(c, p)| (*c, p.clone())).collect();
        for (cell, path) in cells {
            if base.cell_kind(cell) != crate::heap::CellKind::Object {
                continue;
            }
            let o = Obj(cell);
            if h.get_prototype_of(o).map(|p| p.offset())
                != base.get_prototype_of(o).map(|p| p.offset())
            {
                self.refuse(path.clone(), "a builtin whose prototype was changed");
            }
            if h.is_extensible(o) != base.is_extensible(o) {
                self.finals.push_out(format!(
                    "Object.preventExtensions({});",
                    self.builtin_expr_raw(&path)
                ));
            }
            let keys = h.own_keys(o);
            let bkeys = base.own_keys(o);
            let expr = self.builtin_expr_raw(&path);
            for k in &bkeys {
                let name = base.atom_name(*k);
                let now = keys.iter().find(|k2| {
                    h.atom_name(**k2) == name
                        && h.atom_symbol(**k2).is_none() == base.atom_symbol(*k).is_none()
                });
                match now {
                    None => self.builtin_patches.push_out((expr.clone(), *k, None)),
                    Some(k2) => {
                        let (Some(p), Some(bp)) =
                            (h.get_own_property(o, *k2), base.get_own_property(o, *k))
                        else {
                            continue;
                        };
                        if !same_property(h, &p, base, &bp) {
                            self.builtin_patches.push_out((expr.clone(), *k2, Some(p)));
                        }
                    }
                }
            }
            for k in keys {
                let name = h.atom_name(k);
                if !bkeys.iter().any(|bk| base.atom_name(*bk) == name) {
                    let p = h.get_own_property(o, k);
                    self.builtin_patches.push_out((expr.clone(), k, p));
                }
            }
        }
        let patches = core::mem::take(&mut self.builtin_patches);
        for (expr, k, p) in &patches {
            let path = format!("{expr}{}", self.key_member(*k));
            match p {
                Some(Property::Data { value, .. }) => {
                    self.note(*value, || path.clone());
                }
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.note(*getter, || format!("get {path}"));
                    self.note(*setter, || format!("set {path}"));
                }
                None => {}
            }
        }
        self.builtin_patches = patches;
    }

    fn builtin_expr_raw(&self, path: &str) -> String {
        if path.contains('%')
            || path.contains('~')
            || path.contains("[[")
            || path.starts_with("get ")
            || path.starts_with("set ")
        {
            format!("$dump.builtin({})", quote(path))
        } else {
            path.into()
        }
    }

    // ---------------------------------------------------------------- discovery

    fn add_global_root(&mut self, name: &str) -> bool {
        if self.root_names.contains_key(name) {
            return true;
        }
        let Some(&(v, decl)) = self.user_globals.get(name) else {
            return self.host_names.contains(name);
        };
        self.add_root(name, decl, v);
        true
    }

    fn add_root(&mut self, name: &str, decl: Decl, v: Value) {
        if self.root_names.contains_key(name) {
            return;
        }
        if name.starts_with("$dump") {
            self.refuse(name.into(), "the name is the dump's own");
            return;
        }
        let i = self.roots.len();
        let mut value = v;
        if let Some(b) = v.as_internal() {
            self.root_boxes.insert(b, i);
            value = self.h.arena().get(b, BoxCell::VALUE);
        }
        self.root_names.insert(name.into(), i);
        self.roots.push_out(Root {
            name: name.into(),
            decl,
            value,
            state: State::New,
        });
        if value.is_uninitialized() {
            self.refuse(name.into(), "a binding before its declaration ran (TDZ)");
            return;
        }
        if let Some(n) = self.note(value, || name.into())
            && self.nodes[n].root.is_none()
        {
            self.nodes[n].root = Some(i);
        }
        self.drain();
    }

    /// Records a reference to `v`; its node, if it is one the dump writes.
    fn note(&mut self, v: Value, path: impl FnOnce() -> String) -> Option<usize> {
        match v.kind() {
            Kind::Object | Kind::Symbol | Kind::Internal => {}
            Kind::BigInt
            | Kind::Undefined
            | Kind::Null
            | Kind::Bool
            | Kind::Number
            | Kind::String => {
                return None;
            }
            Kind::Hole => return None,
            _ => {
                self.refuse(path(), "an internal value");
                return None;
            }
        }
        if let Some(c) = v
            .as_object()
            .map(|o| o.offset())
            .or(v.as_symbol().map(|s| s.offset()))
        {
            if let Some(p) = self.builtins.get(&c).cloned() {
                let root = p.strip_prefix('%').and_then(|r| r.split('%').next());
                if p.contains('~') || p.contains("%intrinsic:") {
                    self.refuse(path(), &format!("a builtin's internal value ({p})"));
                } else if let Some(r) = root
                    && !LOADER_ROOTS.iter().any(|(n, _)| *n == r)
                {
                    self.refuse(path(), &format!("a builtin the loader cannot name ({p})"));
                }
                return None;
            }
            if self.host.contains_key(&c) || c == self.vm.global().offset() {
                return None;
            }
            if c < self.base_end {
                self.refuse(path(), "a builtin without a name");
                return None;
            }
        }
        let key = v.to_bits();
        if let Some(&n) = self.ids.get(&key) {
            self.nodes[n].refs += 1;
            return Some(n);
        }
        let n = self.nodes.len();
        self.ids.insert(key, n);
        self.nodes.push_out(Node {
            v,
            path: path(),
            refs: 1,
            edges: Vec::new(),
            cyclic: false,
            root: None,
            state: State::New,
            loc: String::new(),
            group: usize::MAX,
        });
        Some(n)
    }

    /// Visits the nodes found so far (and the ones they lead to).
    fn drain(&mut self) {
        while self.visited < self.nodes.len() {
            let i = self.visited;
            self.visited += 1;
            self.visit(i);
        }
    }

    fn edge(&mut self, from: usize, v: Value, path: impl FnOnce() -> String) {
        if let Some(n) = self.note(v, path) {
            self.nodes[from].edges.push_out(n);
        }
    }

    fn key_member(&self, k: Atom) -> String {
        match self.h.atom_symbol(k) {
            Some(s) => {
                let v = Value::symbol(s);
                match self.builtins.get(&s.offset()) {
                    Some(p) => format!("[{p}]"),
                    None => {
                        let _ = v;
                        format!("[{}]", self.h.atom_name(k))
                    }
                }
            }
            None => member(&self.h.atom_name(k)),
        }
    }

    fn props(&mut self, n: usize, o: Obj, skip: &dyn Fn(&str) -> bool) {
        let h = self.h;
        for k in h.own_keys(o) {
            if h.is_private_atom(k) {
                let p = self.nodes[n].path.clone();
                self.refuse(p, "an object with private fields");
                continue;
            }
            if h.atom_symbol(k).is_none() && skip(&h.atom_name(k)) {
                continue;
            }
            if let Some(s) = h.atom_symbol(k) {
                let base = self.nodes[n].path.clone();
                self.edge(n, Value::symbol(s), || format!("{base}<key>"));
            }
            let path = format!("{}{}", self.nodes[n].path, self.key_member(k));
            match h.get_own_property(o, k) {
                Some(Property::Data { value, .. }) => self.edge(n, value, || path),
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.edge(n, getter, || format!("get {path}"));
                    self.edge(n, setter, || format!("set {path}"));
                }
                None => {}
            }
        }
        if let Some(p) = h.get_prototype_of(o)
            && !self.default_proto(o, Some(p))
        {
            let path = format!("{}.[[Prototype]]", self.nodes[n].path);
            self.edge(n, Value::object(p), || path);
        }
    }

    /// The prototype a literal or constructor call of `o`'s class gives it.
    fn class_proto(&self, o: Obj) -> Option<Option<Intrinsic>> {
        let h = self.h;
        Some(Some(match h.object_class(o) {
            ObjectClass::Ordinary => Intrinsic::ObjectProto,
            ObjectClass::Array => Intrinsic::ArrayProto,
            ObjectClass::Function => {
                let t = h.closure_template(o);
                let f = u32::from(h.arena().get::<u16>(t, FuncTemplate::FLAGS));
                return Some(Some(
                    match (f & func_flags::ASYNC != 0, f & func_flags::GENERATOR != 0) {
                        (false, false) => Intrinsic::FunctionProto,
                        (true, false) => Intrinsic::AsyncFunctionProto,
                        (false, true) => Intrinsic::GeneratorFunctionProto,
                        (true, true) => Intrinsic::AsyncGeneratorFunctionProto,
                    },
                ));
            }
            ObjectClass::Map => Intrinsic::MapProto,
            ObjectClass::Set => Intrinsic::SetProto,
            ObjectClass::Date => Intrinsic::DateProto,
            ObjectClass::RegExp => Intrinsic::RegExpProto,
            ObjectClass::Promise => Intrinsic::PromiseProto,
            ObjectClass::Boolean => Intrinsic::BooleanProto,
            ObjectClass::Number => Intrinsic::NumberProto,
            ObjectClass::String => Intrinsic::StringProto,
            ObjectClass::ArrayBuffer => Intrinsic::ArrayBufferProto,
            ObjectClass::BoundFunction => Intrinsic::FunctionProto,
            _ => return None,
        }))
    }

    fn default_proto(&self, o: Obj, p: Option<Obj>) -> bool {
        let h = self.h;
        match h.object_class(o) {
            // Typed arrays, DataViews and errors: the constructor call picks it; only a
            // non-builtin prototype is state.
            ObjectClass::TypedArray | ObjectClass::DataView | ObjectClass::Error => {
                p.is_some_and(|p| p.offset() < self.base_end)
            }
            _ => match self.class_proto(o) {
                Some(Some(i)) => intrinsic(h, i).as_object() == p,
                _ => true,
            },
        }
    }

    fn visit(&mut self, n: usize) {
        let h = self.h;
        let v = self.nodes[n].v;
        if let Some(b) = v.as_internal() {
            let inner: Value = h.arena().get(b, BoxCell::VALUE);
            let path = self.nodes[n].path.clone();
            if inner.is_uninitialized() {
                self.refuse(path, "a captured binding before its declaration ran (TDZ)");
                return;
            }
            self.edge(n, inner, || path);
            return;
        }
        if let Some(s) = v.as_symbol() {
            let _ = s;
            return;
        }
        let o = v.as_object().expect("an object");
        let path = self.nodes[n].path.clone();
        match h.object_class(o) {
            ObjectClass::Ordinary => self.props(n, o, &|_| false),
            ObjectClass::Array => {
                let len = h.array_length(o);
                let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
                for i in 0..len {
                    let v = match el.get() {
                        Some(el) if i < h.value_array_len(el) => h.value_array_get(el, i),
                        _ => Value::HOLE,
                    };
                    let p = format!("{path}[{i}]");
                    self.edge(n, v, || p);
                }
                // Sparse elements and named properties.
                let dense = el.get().map_or(0, |el| h.value_array_len(el));
                self.props(n, o, &|k| {
                    k == "length" || k.parse::<u32>().is_ok_and(|i| i < dense)
                });
            }
            ObjectClass::Function => self.closure(n, o),
            ObjectClass::Map | ObjectClass::Set => {
                for (i, (k, v)) in h.collection_entries_list(o).into_iter().enumerate() {
                    let p = format!("{path}<entries>[{i}]");
                    self.edge(n, k, || p.clone());
                    if h.object_class(o) == ObjectClass::Map {
                        self.edge(n, v, || p);
                    }
                }
                self.props(n, o, &|_| false);
            }
            ObjectClass::Date | ObjectClass::Boolean | ObjectClass::Number => {
                self.props(n, o, &|_| false)
            }
            ObjectClass::String => self.props(n, o, &|k| k == "length" || k.parse::<u32>().is_ok()),
            ObjectClass::RegExp => self.props(n, o, &|k| k == "lastIndex"),
            ObjectClass::Error => self.props(n, o, &|k| k == "message" || k == "stack"),
            ObjectClass::Promise => {
                let state = h.object_slot(o, 0).as_int().unwrap_or(0);
                if state == crate::interp::run::promise::PENDING {
                    self.refuse(path, "a pending promise");
                    return;
                }
                let r = h.object_slot(o, 1);
                self.edge(n, r, || format!("{path}<result>"));
                self.props(n, o, &|_| false);
            }
            ObjectClass::BoundFunction => {
                self.edge(n, h.object_slot(o, 0), || format!("{path}<target>"));
                self.edge(n, h.object_slot(o, 1), || format!("{path}<this>"));
                if let Some(args) = h.object_slot(o, 2).as_object() {
                    let len = h.array_length(args);
                    let el: Ref = h.arena().get(args.offset(), ObjectCell::ELEMENTS);
                    for i in 0..len {
                        let v = el
                            .get()
                            .map_or(Value::UNDEFINED, |el| h.value_array_get(el, i));
                        self.edge(n, v, || format!("{path}<args>[{i}]"));
                    }
                }
                self.props(n, o, &|k| k == "name" || k == "length");
            }
            #[cfg(feature = "full")]
            ObjectClass::ArrayBuffer => {
                if h.object_slot(o, 1).is_number() {
                    self.refuse(path, "a resizable ArrayBuffer");
                }
                self.props(n, o, &|_| false);
            }
            #[cfg(feature = "full")]
            ObjectClass::TypedArray | ObjectClass::DataView => {
                let b = h.object_slot(o, 0);
                self.edge(n, b, || format!("{path}.buffer"));
                self.props(n, o, &|k| k.parse::<u32>().is_ok());
            }
            ObjectClass::HostFunction => {
                self.refuse(path, "a host function the host does not re-bind by name")
            }
            ObjectClass::Generator => self.refuse(path, "a suspended generator or async function"),
            ObjectClass::Proxy => self.refuse(path, "a Proxy"),
            c => self.refuse(path, &format!("an object of class {c:?}")),
        }
    }

    fn closure(&mut self, n: usize, o: Obj) {
        let h = self.h;
        let path = self.nodes[n].path.clone();
        let t = h.closure_template(o);
        let a = h.arena();
        let flags = u32::from(a.get::<u16>(t, FuncTemplate::FLAGS));
        let kind = func_flags::kind(flags);
        if flags & func_flags::PRIVILEGED != 0 {
            return self.refuse(path, "an internal function of a builtin");
        }
        if matches!(
            kind,
            func_flags::KIND_CLASS_CTOR
                | func_flags::KIND_DERIVED_CTOR
                | func_flags::KIND_FIELD_INIT
        ) {
            return self.refuse(path, "a class (classes are not dumped yet)");
        }
        if flags & func_flags::HOME != 0 {
            return self.refuse(path, "a method that uses super");
        }
        if crate::builtins::function::source_slice(h, o).is_none() {
            return self.refuse(path, "a function without source text");
        }
        let names = self.template_names(t);
        let locals = u32::from(a.get::<u16>(t, FuncTemplate::LOCALS));
        let captures = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
        for i in 0..captures {
            let name = names
                .get((locals + i) as usize)
                .cloned()
                .unwrap_or_default();
            let Some(b) = h.object_slot(o, i).as_internal() else {
                continue;
            };
            if !is_ident(&name) {
                self.refuse(
                    format!("{path}~{name}"),
                    "a captured `this`, `arguments` or `new.target`",
                );
                continue;
            }
            if let Some(&r) = self.root_boxes.get(&b) {
                if self.roots[r].name != name {
                    self.refuse(
                        format!("{path}~{name}"),
                        "captures a root under another name",
                    );
                }
                continue;
            }
            let p = format!("{path}~{name}");
            let bv = Value::internal(b);
            self.edge(n, bv, || p);
        }
        // The user globals its code reads by name are part of the dump.
        let mut globals = BTreeSet::new();
        self.global_refs(t, &mut globals);
        for g in globals {
            if !self.root_names.contains_key(&g) && self.user_globals.contains_key(&g) {
                let (v, decl) = self.user_globals[&g];
                self.add_root_deferred(&g, decl, v);
            }
        }
        let fn_length = u32::from(a.get::<u16>(t, FuncTemplate::LENGTH));
        let _ = fn_length;
        let proto_default = |d: &Self, k: &str| -> bool {
            if k != "prototype" {
                return k == "length" || k == "name";
            }
            d.default_fn_prototype(o)
        };
        let keys: Vec<Atom> = h.own_keys(o);
        for k in keys {
            if h.atom_symbol(k).is_none() && proto_default(self, &h.atom_name(k)) {
                continue;
            }
            let p = format!("{path}{}", self.key_member(k));
            match h.get_own_property(o, k) {
                Some(Property::Data { value, .. }) => self.edge(n, value, || p),
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.edge(n, getter, || p.clone());
                    self.edge(n, setter, || p);
                }
                None => {}
            }
        }
        if !self.default_proto(o, h.get_prototype_of(o)) {
            self.refuse(path, "a function with another prototype");
        }
    }

    /// Roots found while visiting: added after the current node.
    fn add_root_deferred(&mut self, name: &str, decl: Decl, v: Value) {
        if self.root_names.contains_key(name) {
            return;
        }
        let i = self.roots.len();
        self.root_names.insert(name.into(), i);
        self.roots.push_out(Root {
            name: name.into(),
            decl,
            value: v,
            state: State::New,
        });
        if v.is_uninitialized() {
            self.refuse(name.into(), "a binding before its declaration ran (TDZ)");
            return;
        }
        if let Some(n) = self.note(v, || name.into())
            && self.nodes[n].root.is_none()
        {
            self.nodes[n].root = Some(i);
        }
    }

    /// A function's `prototype` as its definition makes it: absent, or an object with
    /// only `constructor` (back to the function), or an empty generator prototype.
    fn default_fn_prototype(&self, f: Obj) -> bool {
        let h = self.h;
        let Some(atom) = h
            .own_keys(f)
            .into_iter()
            .find(|k| h.atom_name(*k) == "prototype" && h.atom_symbol(*k).is_none())
        else {
            return true;
        };
        let Some(Property::Data { value, .. }) = h.get_own_property(f, atom) else {
            return false;
        };
        let Some(p) = value.as_object() else {
            return false;
        };
        let keys = h.own_keys(p);
        keys.is_empty()
            || (keys.len() == 1
                && h.atom_name(keys[0]) == "constructor"
                && h.get_own_property(p, keys[0]).and_then(|p| p.value()) == Some(Value::object(f)))
    }

    fn template_names(&self, t: u32) -> Vec<String> {
        let a = self.h.arena();
        let names = template::debug_info(self.h, t).names;
        match names.get() {
            Some(n) => {
                let len: u32 = a.get(n, U32Array::LEN);
                (0..len)
                    .map(|i| {
                        self.h
                            .atom_name(Atom::from_id(a.read::<u32>(U32Array::DATA.at(n, i))))
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    fn global_refs(&self, t: u32, out: &mut BTreeSet<String>) {
        for insn in crate::bytecode::insns(template::code(self.h, t)).flatten() {
            if matches!(
                insn.info().name,
                "get_global" | "get_global_typeof" | "put_global"
            ) {
                out.insert(self.h.atom_name(insn.atom(0)));
            }
        }
        for i in 0..template::child_count(self.h, t) {
            if let Some(c) = template::child(self.h, t, i).and_then(|r| r.get()) {
                self.global_refs(c, out);
            }
        }
        let _ = FuncInfo::SCRIPT;
    }

    // ---------------------------------------------------------------- analysis

    /// Closure groups (closures that share a captured box, transitively) and the nodes
    /// on cycles (Tarjan's strongly connected components, iterative).
    fn analyze(&mut self) {
        let n = self.nodes.len();
        // Union-find over closures and their boxes.
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        let mut is_closure = alloc::vec![false; n];
        for i in 0..n {
            let v = self.nodes[i].v;
            if v.as_object()
                .is_some_and(|o| self.h.object_class(o) == ObjectClass::Function)
            {
                is_closure[i] = true;
                for &e in &self.nodes[i].edges {
                    if self.nodes[e].v.as_internal().is_some() {
                        let (a, b) = (find(&mut parent, i), find(&mut parent, e));
                        parent[a] = b;
                    }
                }
            }
        }
        let mut group_of: BTreeMap<usize, usize> = BTreeMap::new();
        for i in 0..n {
            let is_box = self.nodes[i].v.as_internal().is_some();
            if !is_box && !is_closure[i] {
                self.nodes[i].group = usize::MAX;
                continue;
            }
            let r = find(&mut parent, i);
            // A closure without captured boxes is free: no group.
            if is_closure[i]
                && r == i
                && !self.nodes[i]
                    .edges
                    .iter()
                    .any(|&e| self.nodes[e].v.as_internal().is_some())
                && !parent.iter().enumerate().any(|(j, &p)| j != i && p == i)
            {
                self.nodes[i].group = usize::MAX;
                continue;
            }
            let g = *group_of.entry(r).or_insert_with(|| {
                self.groups.push_out(Group {
                    closures: Vec::new(),
                    boxes: Vec::new(),
                    state: State::New,
                    loc: String::new(),
                });
                self.groups.len() - 1
            });
            self.nodes[i].group = g;
            if is_box {
                self.groups[g].boxes.push_out(i);
            } else {
                self.groups[g].closures.push_out(i);
            }
        }
        // Tarjan.
        let mut index = alloc::vec![usize::MAX; n];
        let mut low = alloc::vec![0usize; n];
        let mut on = alloc::vec![false; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut next = 0usize;
        for s in 0..n {
            if index[s] != usize::MAX {
                continue;
            }
            let mut work: Vec<(usize, usize)> = alloc::vec![(s, 0)];
            while let Some(&mut (v, ref mut ei)) = work.last_mut() {
                if *ei == 0 && index[v] == usize::MAX {
                    index[v] = next;
                    low[v] = next;
                    next += 1;
                    stack.push_out(v);
                    on[v] = true;
                }
                if let Some(&w) = self.nodes[v].edges.get(*ei) {
                    *ei += 1;
                    if index[w] == usize::MAX {
                        work.push_out((w, 0));
                    } else if on[w] {
                        low[v] = low[v].min(index[w]);
                    }
                    continue;
                }
                work.pop();
                if let Some(&(u, _)) = work.last() {
                    low[u] = low[u].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp.push_out(w);
                        if w == v {
                            break;
                        }
                    }
                    let self_loop = self.nodes[v].edges.contains(&v);
                    if comp.len() > 1 || self_loop {
                        for w in comp {
                            self.nodes[w].cyclic = true;
                        }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- emission

    fn finish(mut self) -> Result<String, DumpError> {
        self.drain();
        if !self.refusals.is_empty() {
            return Err(DumpError(self.refusals));
        }
        self.analyze();
        for r in 0..self.roots.len() {
            self.emit_root(r);
        }
        let patches = core::mem::take(&mut self.builtin_patches);
        let mut builtin_lines = Vec::new();
        for (expr, k, p) in patches {
            let target = format!("{expr}{}", self.key_member(k));
            let key = self.key_expr(k);
            builtin_lines.push_out(match p {
                None => format!("delete {target};"),
                Some(Property::Data { value, flags }) if flags == PropFlags::ALL => {
                    format!("{target} = {};", self.expr(value, None, None))
                }
                Some(p) => format!(
                    "Object.defineProperty({expr}, {key}, {});",
                    self.descriptor(&p)
                ),
            });
        }
        if !self.refusals.is_empty() {
            return Err(DumpError(self.refusals));
        }
        let mut s = String::from(HEADER);
        let names: Vec<&str> = self.roots.iter().map(|r| r.name.as_str()).collect();
        s.push_str(&format!(
            ": {} binding{} ({}).\n// Run it as a Script to recreate them (djs REPL: .restore FILE). Builtins are\n// referenced by name, never written",
            names.len(),
            if names.len() == 1 { "" } else { "s" },
            names.join(", ")
        ));
        if !self.host_names.is_empty() {
            let h: Vec<&str> = self.host_names.iter().map(|s| s.as_str()).collect();
            s.push_str(&format!(
                "; the host's {} are re-bound by name",
                h.join(", ")
            ));
        }
        s.push_str(".\n");
        let internal = !self.helpers.is_empty()
            || self
                .out
                .iter()
                .chain(&self.patches)
                .chain(&builtin_lines)
                .chain(&self.finals)
                .any(|l| l.contains("$dump"));
        if internal {
            s.push_str("globalThis.$dump = {};\n");
            for (name, text) in HELPERS {
                if self.helpers.contains(name) {
                    s.push_str(text);
                    s.push('\n');
                }
            }
            if self.helpers.contains("builtin") {
                s.push_str(&loader_builtin());
            }
        }
        for l in &self.out {
            s.push_str(l);
            s.push('\n');
        }
        if !builtin_lines.is_empty() {
            s.push_str("// Builtins the session changed.\n");
            for l in &builtin_lines {
                s.push_str(l);
                s.push('\n');
            }
        }
        if !self.patches.is_empty() || !self.finals.is_empty() {
            s.push_str("// Links that close cycles, and attributes.\n");
            for l in self.patches.iter().chain(&self.finals) {
                s.push_str(l);
                s.push('\n');
            }
        }
        if internal {
            s.push_str("void delete globalThis.$dump;\n");
        }
        Ok(s)
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("$dump.{prefix}{}", self.next)
    }

    fn key_expr(&mut self, k: Atom) -> String {
        match self.h.atom_symbol(k) {
            Some(s) => self.expr(Value::symbol(s), None, None),
            None => quote(&self.h.atom_name(k)),
        }
    }

    /// The key in an object literal: `name`, `"a b"`, `[Symbol.iterator]`.
    fn literal_key(&mut self, k: Atom) -> String {
        match self.h.atom_symbol(k) {
            Some(s) => format!("[{}]", self.expr(Value::symbol(s), None, None)),
            None => {
                let name = self.h.atom_name(k);
                if name == "__proto__" {
                    format!("[{}]", quote(&name))
                } else if is_ident(&name)
                    || name.parse::<u32>().is_ok_and(|i| i.to_string() == name)
                {
                    name
                } else {
                    quote(&name)
                }
            }
        }
    }

    fn descriptor(&mut self, p: &Property) -> String {
        match *p {
            Property::Data { value, flags } => format!(
                "{{ value: {}, writable: {}, enumerable: {}, configurable: {} }}",
                self.expr(value, None, None),
                flags.writable(),
                flags.enumerable(),
                flags.configurable()
            ),
            Property::Accessor {
                getter,
                setter,
                flags,
            } => format!(
                "{{ get: {}, set: {}, enumerable: {}, configurable: {} }}",
                self.expr(getter, None, None),
                self.expr(setter, None, None),
                flags.enumerable(),
                flags.configurable()
            ),
        }
    }

    fn emit_root(&mut self, r: usize) {
        if self.roots[r].state != State::New {
            return;
        }
        self.roots[r].state = State::Busy;
        let name = self.roots[r].name.clone();
        let value = self.roots[r].value;
        let kw = match self.roots[r].decl {
            Decl::Const => "const",
            Decl::Let => "let",
            Decl::Var => "var",
        };
        let node = self.node_of(value);
        let line = match node {
            Some(n) if self.nodes[n].root == Some(r) && self.nodes[n].state == State::New => {
                self.nodes[n].loc = name.clone();
                if self.free_closure(n)
                    && let Some(src) = self.fn_source(n)
                    && declared_name(self.fn_kind(n), &src) == Some(name.as_str())
                {
                    // As written: `function name() {...}`.
                    self.nodes[n].state = State::Done;
                    self.fn_patches(n, Some(&name));
                    src
                } else {
                    let e = self.render(n, Some(&name), Some(&name));
                    format!("{kw} {name} = {e};")
                }
            }
            _ => format!("{kw} {name} = {};", self.expr(value, None, Some(&name))),
        };
        self.out.push_out(line);
        self.roots[r].state = State::Done;
    }

    fn node_of(&self, v: Value) -> Option<usize> {
        self.ids.get(&v.to_bits()).copied()
    }

    fn free_closure(&self, n: usize) -> bool {
        self.nodes[n].group == usize::MAX
            && self.nodes[n]
                .v
                .as_object()
                .is_some_and(|o| self.h.object_class(o) == ObjectClass::Function)
    }

    fn fn_kind(&self, n: usize) -> u32 {
        let o = self.nodes[n].v.as_object().expect("a function");
        let t = self.h.closure_template(o);
        func_flags::kind(u32::from(self.h.arena().get::<u16>(t, FuncTemplate::FLAGS)))
    }

    fn fn_source(&self, n: usize) -> Option<String> {
        crate::builtins::function::source_slice(self.h, self.nodes[n].v.as_object()?)
    }

    /// The expression for a value, in a context that may give a name (`ctx_name`: the
    /// binding or key an anonymous function would be named after) and a location
    /// (`loc`: an expression that reaches the value afterwards, for patches).
    fn expr(&mut self, v: Value, loc: Option<&str>, ctx_name: Option<&str>) -> String {
        match v.kind() {
            Kind::Undefined => return "undefined".into(),
            Kind::Null => return "null".into(),
            Kind::Bool => return v.as_bool().unwrap_or(false).to_string(),
            Kind::Number => return number(v.as_number().unwrap_or(f64::NAN)),
            Kind::String => return quote_units(self.h.str_units(v.as_string().expect("a string"))),
            Kind::BigInt => {
                return format!("{}n", self.h.bigint(v).to_string_radix(10));
            }
            Kind::Hole => return String::new(),
            _ => {}
        }
        let cell = v
            .as_object()
            .map(|o| o.offset())
            .or(v.as_symbol().map(|s| s.offset()));
        if let Some(c) = cell {
            if let Some(p) = self.builtins.get(&c) {
                let p = p.clone();
                let e = self.builtin_expr_raw(&p);
                if e.starts_with("$dump") {
                    self.helpers.insert("builtin");
                }
                return e;
            }
            if let Some(e) = self.host.get(&c) {
                return e.clone();
            }
            if c == self.vm.global().offset() {
                return "globalThis".into();
            }
        }
        let Some(n) = self.node_of(v) else {
            return "undefined".into();
        };
        match self.nodes[n].state {
            State::Done => return self.nodes[n].loc.clone(),
            // Only nodes on a cycle are reached while busy; the caller patches.
            State::Busy => return String::new(),
            State::New => {}
        }
        if let Some(r) = self.nodes[n].root
            && self.roots[r].state == State::New
        {
            self.emit_root(r);
            return self.nodes[n].loc.clone();
        }
        if self.nodes[n].group != usize::MAX {
            let g = self.nodes[n].group;
            self.emit_group(g);
            return self.nodes[n].loc.clone();
        }
        let inline = self.nodes[n].refs == 1 && !self.nodes[n].cyclic && self.depth < 64;
        if inline && (loc.is_some() || self.simple(n, ctx_name)) {
            self.nodes[n].loc = loc.unwrap_or_default().into();
            self.depth += 1;
            let e = self.render(n, loc, ctx_name);
            self.depth -= 1;
            return e;
        }
        let name = self.fresh("n");
        self.nodes[n].loc = name.clone();
        let e = self.render(n, Some(&name), None);
        self.out.push_out(format!("{name} = {e};"));
        name
    }

    /// Whether node `n` needs no patches when written in a context without a location.
    fn simple(&self, n: usize, ctx_name: Option<&str>) -> bool {
        let h = self.h;
        let Some(o) = self.nodes[n].v.as_object() else {
            return false;
        };
        if !h.is_extensible(o) {
            return false;
        }
        match h.object_class(o) {
            ObjectClass::Ordinary | ObjectClass::Array => h.own_keys(o).into_iter().all(|k| {
                matches!(h.get_own_property(o, k), Some(Property::Data { value, flags })
                    if (flags == PropFlags::ALL || (h.object_class(o) == ObjectClass::Array && h.atom_name(k) == "length"))
                        && self.node_of(value).is_none_or(|c| self.nodes[c].state == State::Done || (self.nodes[c].refs == 1 && !self.nodes[c].cyclic && self.simple(c, Some(&h.atom_name(k))))))
            }) && h.get_prototype_of(o).is_some_and(|p| self.default_proto(o, Some(p))),
            ObjectClass::Function => {
                self.fn_name_matches(n, ctx_name) && h.own_keys(o).into_iter().all(|k| {
                    let name = h.atom_name(k);
                    name == "length" || name == "name" || (name == "prototype" && self.default_fn_prototype(o))
                })
            }
            ObjectClass::ArrayBuffer
            | ObjectClass::Date
            | ObjectClass::RegExp
            | ObjectClass::TypedArray
            | ObjectClass::DataView
            | ObjectClass::Boolean
            | ObjectClass::Number => {
                h.own_keys(o).into_iter().all(|k| {
                    let name = h.atom_name(k);
                    name.parse::<u32>().is_ok()
                        || (name == "lastIndex"
                            && h.get_own_property(o, k).and_then(|p| p.value()) == Some(Value::int(0)))
                }) && self.default_proto(o, h.get_prototype_of(o))
            }
            _ => false,
        }
    }

    fn fn_name(&self, n: usize) -> String {
        let h = self.h;
        let o = self.nodes[n].v.as_object().expect("a function");
        h.own_keys(o)
            .into_iter()
            .find(|k| h.atom_name(*k) == "name" && h.atom_symbol(*k).is_none())
            .and_then(|k| h.get_own_property(o, k))
            .and_then(|p| p.value())
            .and_then(|v| v.as_string())
            .map(|s| h.str_to_string(s))
            .unwrap_or_default()
    }

    fn fn_name_matches(&self, n: usize, ctx_name: Option<&str>) -> bool {
        let Some(src) = self.fn_source(n) else {
            return false;
        };
        !anonymous_form(self.fn_kind(n), &src) || self.fn_name(n) == ctx_name.unwrap_or("")
    }

    /// Own properties of a function beyond its definition, and a `name` the context
    /// does not give it.
    fn fn_patches(&mut self, n: usize, ctx_name: Option<&str>) {
        let h = self.h;
        let o = self.nodes[n].v.as_object().expect("a function");
        let loc = self.nodes[n].loc.clone();
        if !self.fn_name_matches(n, ctx_name) {
            let name = self.fn_name(n);
            self.patches.push_out(format!(
                "Object.defineProperty({loc}, \"name\", {{ value: {} }});",
                quote(&name)
            ));
        }
        for k in h.own_keys(o) {
            let name = h.atom_name(k);
            if h.atom_symbol(k).is_none()
                && (name == "length"
                    || name == "name"
                    || (name == "prototype" && self.default_fn_prototype(o)))
            {
                continue;
            }
            if let Some(p) = h.get_own_property(o, k) {
                self.prop_patch(&loc, k, &p);
            }
        }
        if !h.is_extensible(o) {
            self.finals
                .push_out(format!("Object.preventExtensions({loc});"));
        }
    }

    fn prop_patch(&mut self, loc: &str, k: Atom, p: &Property) {
        let key = self.key_expr(k);
        let line = match *p {
            Property::Data { value, flags } if flags == PropFlags::ALL => {
                let target = format!("{loc}{}", self.key_member(k));
                let e = self.expr(value, Some(&target), None);
                format!("{target} = {e};")
            }
            _ => format!(
                "Object.defineProperty({loc}, {key}, {});",
                self.descriptor(p)
            ),
        };
        self.patches.push_out(line);
    }

    fn emit_group(&mut self, g: usize) {
        if self.groups[g].state != State::New {
            return;
        }
        self.groups[g].state = State::Busy;
        let loc = self.fresh("s");
        self.groups[g].loc = loc.clone();
        let h = self.h;
        // Each closure's home: a box of the group that holds it under its name, else a
        // key of the returned object.
        let closures = self.groups[g].closures.clone();
        let boxes = self.groups[g].boxes.clone();
        let mut box_name: BTreeMap<usize, String> = BTreeMap::new();
        let mut taken: BTreeSet<String> = BTreeSet::new();
        for &b in &boxes {
            let p = &self.nodes[b].path;
            let name = p.rsplit('~').next().unwrap_or("").to_string();
            if !taken.insert(name.clone()) {
                let path = self.nodes[b].path.clone();
                self.refuse(
                    path,
                    "two captured variables of the same name in one scope group",
                );
            }
            box_name.insert(b, name);
        }
        let mut home: BTreeMap<usize, String> = BTreeMap::new();
        for &b in &boxes {
            let inner: Value = h.arena().get(
                self.nodes[b].v.as_internal().expect("a box"),
                BoxCell::VALUE,
            );
            if let Some(c) = self.node_of(inner)
                && closures.contains(&c)
                && !home.contains_key(&c)
            {
                home.insert(c, box_name[&b].clone());
            }
        }
        let mut keys: BTreeMap<usize, String> = BTreeMap::new();
        for &c in &closures {
            let key = match home.get(&c) {
                Some(name) => name.clone(),
                None => {
                    let fname = self.fn_name(c);
                    if is_ident(&fname) && !taken.contains(&fname) {
                        fname
                    } else {
                        format!("$f{c}")
                    }
                }
            };
            taken.insert(key.clone());
            self.nodes[c].loc = format!("{loc}.{key}");
            self.nodes[c].state = State::Busy;
            keys.insert(c, key);
        }
        for &b in &boxes {
            self.nodes[b].state = State::Busy;
        }
        let mut body: Vec<String> = Vec::new();
        let mut sets: Vec<(String, Value)> = Vec::new();
        for &b in &boxes {
            let name = box_name[&b].clone();
            let inner: Value = h.arena().get(
                self.nodes[b].v.as_internal().expect("a box"),
                BoxCell::VALUE,
            );
            match self.node_of(inner) {
                Some(c) if home.get(&c) == Some(&name) => {
                    let src = self.fn_source(c).unwrap_or_default();
                    if declared_name(self.fn_kind(c), &src) == Some(name.as_str()) {
                        body.push_out(src);
                    } else {
                        let e = self.fn_expr(c, &src);
                        body.push_out(format!("let {name} = {e};"));
                    }
                }
                Some(c) if self.nodes[c].state == State::Busy => {
                    body.push_out(format!("let {name};"));
                    sets.push_out((name, inner));
                }
                _ => {
                    let e = self.expr(inner, None, Some(&name));
                    body.push_out(format!("let {name} = {e};"));
                }
            }
        }
        let mut ret: Vec<String> = Vec::new();
        for &c in &closures {
            let key = keys[&c].clone();
            if home.contains_key(&c) {
                ret.push_out(key);
            } else {
                let src = self.fn_source(c).unwrap_or_default();
                if self.method_text(c, &key, func_flags::KIND_METHOD) {
                    ret.push_out(src);
                } else {
                    let e = self.fn_expr(c, &src);
                    ret.push_out(format!("{key}: {e}"));
                }
            }
        }
        if !sets.is_empty() {
            let names: Vec<&str> = sets.iter().map(|(n, _)| n.as_str()).collect();
            ret.push_out(format!("$set(v) {{ ({{ {} }} = v); }}", names.join(", ")));
        }
        let mut names: Vec<String> = closures.iter().map(|c| keys[c].clone()).collect();
        names.dedup();
        let shared: Vec<&str> = boxes.iter().map(|b| box_name[b].as_str()).collect();
        let mut text = format!(
            "// {} sharing {}\n{loc} = (() => {{\n",
            names.join(", "),
            shared.join(", ")
        );
        for l in body {
            text.push_str(&indent(&l));
        }
        text.push_str(&format!("  return {{ {} }};\n}})();", ret.join(", ")));
        self.out.push_out(text);
        for &b in &boxes {
            self.nodes[b].state = State::Done;
        }
        for &c in &closures {
            self.nodes[c].state = State::Done;
            let ctx = keys[&c].clone();
            self.fn_patches(c, Some(&ctx));
        }
        self.groups[g].state = State::Done;
        if !sets.is_empty() {
            let mut fields = Vec::new();
            for (name, v) in sets {
                let e = self.expr_or_loc(v);
                fields.push_out(format!("{name}: {e}"));
            }
            self.patches
                .push_out(format!("{loc}.$set({{ {} }});", fields.join(", ")));
        }
    }

    /// A function's source as an expression: methods and accessors through `$dump.fn`.
    fn fn_expr(&mut self, n: usize, src: &str) -> String {
        match self.fn_kind(n) {
            func_flags::KIND_METHOD | func_flags::KIND_GETTER | func_flags::KIND_SETTER => {
                self.helpers.insert("fn");
                format!("$dump.fn({{ {src} }})")
            }
            _ => src.into(),
        }
    }

    /// Writes node `n` (state New) as an expression; `loc` reaches it afterwards.
    fn render(&mut self, n: usize, loc: Option<&str>, ctx_name: Option<&str>) -> String {
        self.nodes[n].state = State::Busy;
        let loc_s = loc.map(String::from).unwrap_or_default();
        self.nodes[n].loc = loc_s.clone();
        let h = self.h;
        let v = self.nodes[n].v;
        let e = if let Some(s) = v.as_symbol() {
            let d = h.symbol_description(s);
            let reg = self.registered(Value::symbol(s));
            match (reg, d.as_string()) {
                (Some(k), _) => format!("Symbol.for({k})"),
                (None, Some(d)) => format!("Symbol({})", quote_units(h.str_units(d))),
                (None, None) => "Symbol()".into(),
            }
        } else {
            let o = v.as_object().expect("an object");
            self.render_object(n, o, &loc_s, ctx_name)
        };
        self.nodes[n].state = State::Done;
        e
    }

    /// The `Symbol.for` key of a registered symbol, quoted.
    fn registered(&self, s: Value) -> Option<String> {
        let h = self.h;
        let reg = intrinsic(h, Intrinsic::SymbolRegistry).as_object()?;
        for k in h.own_keys(reg) {
            if h.get_own_property(reg, k).and_then(|p| p.value()) == Some(s) {
                return Some(quote(&h.atom_name(k)));
            }
        }
        None
    }

    fn render_object(&mut self, n: usize, o: Obj, loc: &str, ctx_name: Option<&str>) -> String {
        let h = self.h;
        let class = h.object_class(o);
        let mut extra = true;
        let e = match class {
            ObjectClass::Ordinary => {
                extra = false;
                self.literal(n, o, loc)
            }
            ObjectClass::Array => {
                extra = false;
                let len = h.array_length(o);
                let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
                let mut items = Vec::new();
                for i in 0..len {
                    let v = match el.get() {
                        Some(el) if i < h.value_array_len(el) => h.value_array_get(el, i),
                        _ => Value::HOLE,
                    };
                    let at = format!("{loc}[{i}]");
                    let e = self.expr(v, Some(&at), None);
                    if e.is_empty() && !v.is_hole() {
                        self.patches.push_out(format!(
                            "{at} = {};",
                            self.nodes[self.node_of(v).expect("a node")].loc
                        ));
                        items.push_out("undefined".into());
                    } else {
                        items.push_out(e);
                    }
                }
                if len > 0
                    && el.get().is_none_or(|el| {
                        len > h.value_array_len(el) || h.value_array_get(el, len - 1).is_hole()
                    })
                {
                    items.push_out(String::new());
                }
                let dense = el.get().map_or(0, |el| h.value_array_len(el));
                self.other_props(o, loc, &|k| {
                    k == "length" || k.parse::<u32>().is_ok_and(|i| i < dense)
                });
                if !h.is_extensible(o) {
                    self.finals.push_out(self.freeze_line(o, loc));
                }
                format!("[{}]", items.join(", "))
            }
            ObjectClass::Function => {
                extra = false;
                let src = self.fn_source(n).unwrap_or_default();
                let e = self.fn_expr(n, &src);
                self.fn_patches(n, ctx_name);
                e
            }
            ObjectClass::Map | ObjectClass::Set => {
                let is_map = class == ObjectClass::Map;
                let entries = h.collection_entries_list(o);
                let mut items = Vec::new();
                let mut later = false;
                for (k, v) in &entries {
                    let ke = self.expr(*k, None, None);
                    let ve = if is_map {
                        self.expr(*v, None, None)
                    } else {
                        String::new()
                    };
                    if ke.is_empty() || (is_map && ve.is_empty()) {
                        later = true;
                    }
                    items.push_out(if is_map { format!("[{ke}, {ve}]") } else { ke });
                }
                let ctor = if is_map { "Map" } else { "Set" };
                if later {
                    // An entry closes a cycle: all entries in order, after the data.
                    for (k, v) in &entries {
                        let ke = self.expr_or_loc(*k);
                        if is_map {
                            let ve = self.expr_or_loc(*v);
                            self.patches.push_out(format!("{loc}.set({ke}, {ve});"));
                        } else {
                            self.patches.push_out(format!("{loc}.add({ke});"));
                        }
                    }
                    format!("new {ctor}()")
                } else if items.is_empty() {
                    format!("new {ctor}()")
                } else {
                    format!("new {ctor}([{}])", items.join(", "))
                }
            }
            ObjectClass::Date => format!(
                "new Date({})",
                number(h.object_slot(o, 0).as_number().unwrap_or(f64::NAN))
            ),
            ObjectClass::Boolean | ObjectClass::Number | ObjectClass::String => {
                format!("Object({})", self.expr(h.object_slot(o, 0), None, None))
            }
            ObjectClass::RegExp => {
                let src = h.object_slot(o, crate::builtins::regexp::SLOT_SOURCE);
                let flags = crate::builtins::regexp::flags(h, o);
                let f: String = crate::builtins::regexp::FLAG_CHARS
                    .iter()
                    .filter(|(_, fl)| flags.contains(*fl))
                    .map(|(c, _)| *c as char)
                    .collect();
                let s = src
                    .as_string()
                    .map(|s| h.str_to_string(s))
                    .unwrap_or_default();
                if !s.is_empty() && !s.contains('/') && !s.contains('\n') && !s.contains('\r') {
                    format!("/{s}/{f}")
                } else {
                    format!("new RegExp({}, {})", quote(&s), quote(&f))
                }
            }
            ObjectClass::Error => {
                let proto = h.get_prototype_of(o).map(|p| p.offset());
                let ctor = proto
                    .and_then(|p| self.builtins.get(&p))
                    .and_then(|p| p.strip_suffix(".prototype"))
                    .filter(|c| is_ident(c))
                    .unwrap_or("Error")
                    .to_string();
                let msg = h
                    .own_keys(o)
                    .into_iter()
                    .find(|k| h.atom_name(*k) == "message")
                    .and_then(|k| h.get_own_property(o, k))
                    .and_then(|p| p.value());
                let args = match msg {
                    Some(m) => self.expr(m, None, None),
                    None => String::new(),
                };
                if let Some(stack) = self.vm.error_stack(Value::object(o)) {
                    self.patches.push_out(format!(
                        "Object.defineProperty({loc}, \"stack\", {{ value: {}, writable: true, configurable: true }});",
                        quote(&stack)
                    ));
                }
                if !proto.is_some_and(|p| p < self.base_end) {
                    let p = self.expr(
                        h.get_prototype_of(o).map_or(Value::NULL, Value::object),
                        None,
                        None,
                    );
                    self.patches
                        .push_out(format!("Object.setPrototypeOf({loc}, {p});"));
                }
                format!("new {ctor}({args})")
            }
            ObjectClass::Promise => {
                let state = h.object_slot(o, 0).as_int().unwrap_or(0);
                let r = self.expr(h.object_slot(o, 1), None, None);
                if state == crate::interp::run::promise::FULFILLED {
                    format!("Promise.resolve({r})")
                } else {
                    self.helpers.insert("rejected");
                    format!("$dump.rejected({r})")
                }
            }
            ObjectClass::BoundFunction => {
                let target = self.expr(h.object_slot(o, 0), None, None);
                let this = self.expr(h.object_slot(o, 1), None, None);
                let mut args = alloc::vec![this];
                if let Some(a) = h.object_slot(o, 2).as_object() {
                    let el: Ref = h.arena().get(a.offset(), ObjectCell::ELEMENTS);
                    for i in 0..h.array_length(a) {
                        let v = el
                            .get()
                            .map_or(Value::UNDEFINED, |el| h.value_array_get(el, i));
                        args.push_out(self.expr(v, None, None));
                    }
                }
                if target.is_empty() || args.iter().any(String::is_empty) {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "a bound function on a cycle");
                }
                format!("{target}.bind({})", args.join(", "))
            }
            #[cfg(feature = "full")]
            ObjectClass::ArrayBuffer => {
                self.helpers.insert("bytes");
                format!("$dump.bytes(\"{}\")", base64(h.buffer_data(o)))
            }
            #[cfg(feature = "full")]
            ObjectClass::TypedArray | ObjectClass::DataView => {
                let b = h.object_slot(o, 0);
                let (name, off, len) = if class == ObjectClass::TypedArray {
                    (
                        h.ta_kind(o).name(),
                        h.ta_byte_offset(o),
                        h.object_slot(o, 3).as_number().unwrap_or(-1.0) as i64,
                    )
                } else {
                    (
                        "DataView",
                        h.object_slot(o, 1).as_number().unwrap_or(0.0) as u32,
                        h.object_slot(o, 2).as_number().unwrap_or(-1.0) as i64,
                    )
                };
                let whole = off == 0
                    && b.as_object().is_some_and(|b| {
                        let size = if class == ObjectClass::TypedArray {
                            h.ta_kind(o).size()
                        } else {
                            1
                        };
                        len >= 0 && h.buffer_len(b) as i64 == len * size as i64
                    });
                // A small typed array that is its buffer's only view: its elements.
                let bn = self.node_of(b).expect("a buffer node");
                if whole
                    && class == ObjectClass::TypedArray
                    && !h.ta_kind(o).is_bigint()
                    && len <= 64
                    && self.nodes[bn].refs == 1
                    && self.nodes[bn].state == State::New
                    && self.simple(bn, None)
                    && self.simple(n, None)
                {
                    self.nodes[bn].state = State::Done;
                    self.nodes[bn].loc = format!("{loc}.buffer");
                    let kind = h.ta_kind(o);
                    let data = h.buffer_data(b.as_object().expect("a buffer"));
                    let items: Vec<String> = data
                        .chunks(kind.size() as usize)
                        .map(|c| number(crate::heap::typed::decode(kind, c)))
                        .collect();
                    return format!("new {name}([{}])", items.join(", "));
                }
                let buf = self.expr(b, None, None);
                if whole {
                    format!("new {name}({buf})")
                } else if len < 0 {
                    format!("new {name}({buf}, {off})")
                } else {
                    format!("new {name}({buf}, {off}, {len})")
                }
            }
            _ => String::new(),
        };
        if extra {
            let skip: &dyn Fn(&str) -> bool = match class {
                ObjectClass::RegExp => &|k| k == "lastIndex",
                ObjectClass::Error => &|k| k == "message" || k == "stack",
                ObjectClass::String => &|k| k == "length" || k.parse::<u32>().is_ok(),
                ObjectClass::BoundFunction => &|k| k == "name" || k == "length",
                ObjectClass::TypedArray => &|k| k.parse::<u32>().is_ok(),
                _ => &|_| false,
            };
            self.other_props(o, loc, skip);
            if class == ObjectClass::RegExp
                && let Some(li) = h
                    .own_keys(o)
                    .into_iter()
                    .find(|k| h.atom_name(*k) == "lastIndex")
                && let Some(Property::Data { value, .. }) = h.get_own_property(o, li)
                && value.as_number() != Some(0.0)
            {
                let e = self.expr(value, None, None);
                self.patches.push_out(format!("{loc}.lastIndex = {e};"));
            }
            if !matches!(
                class,
                ObjectClass::Error | ObjectClass::TypedArray | ObjectClass::DataView
            ) && !self.default_proto(o, h.get_prototype_of(o))
            {
                let p = self.expr_or_loc(h.get_prototype_of(o).map_or(Value::NULL, Value::object));
                self.patches
                    .push_out(format!("Object.setPrototypeOf({loc}, {p});"));
            }
            if !h.is_extensible(o) {
                self.finals.push_out(self.freeze_line(o, loc));
            }
        }
        e
    }

    fn expr_or_loc(&mut self, v: Value) -> String {
        let e = self.expr(v, None, None);
        if e.is_empty() {
            self.node_of(v)
                .map(|n| self.nodes[n].loc.clone())
                .unwrap_or_default()
        } else {
            e
        }
    }

    fn freeze_line(&self, o: Obj, loc: &str) -> String {
        let h = self.h;
        let keys = h.own_keys(o);
        let all = |f: &dyn Fn(PropFlags, bool) -> bool| {
            keys.iter().all(|k| match h.get_own_property(o, *k) {
                Some(Property::Data { flags, .. }) => f(flags, true),
                Some(Property::Accessor { flags, .. }) => f(flags, false),
                None => true,
            })
        };
        if all(&|f, data| !f.configurable() && (!data || !f.writable())) {
            format!("Object.freeze({loc});")
        } else if all(&|f, _| !f.configurable()) {
            format!("Object.seal({loc});")
        } else {
            format!("Object.preventExtensions({loc});")
        }
    }

    /// Properties of a non-literal object (a Map's, a regexp's) as patches.
    fn other_props(&mut self, o: Obj, loc: &str, skip: &dyn Fn(&str) -> bool) {
        let h = self.h;
        for k in h.own_keys(o) {
            if h.atom_symbol(k).is_none() && skip(&h.atom_name(k)) {
                continue;
            }
            if let Some(p) = h.get_own_property(o, k) {
                self.prop_patch(loc, k, &p);
            }
        }
    }

    /// An ordinary object as a literal; attributes, accessors that do not fit and links
    /// that close cycles are patched (a placeholder keeps the key order).
    fn literal(&mut self, n: usize, o: Obj, loc: &str) -> String {
        let h = self.h;
        let mut items: Vec<String> = Vec::new();
        let p = h.get_prototype_of(o);
        if !self.default_proto(o, p) {
            match p {
                None => items.push_out("__proto__: null".into()),
                Some(p) => {
                    let e = self.expr(Value::object(p), None, None);
                    if e.is_empty() {
                        let l = self.expr_or_loc(Value::object(p));
                        self.patches
                            .push_out(format!("Object.setPrototypeOf({loc}, {l});"));
                    } else {
                        items.push_out(format!("__proto__: {e}"));
                    }
                }
            }
        }
        let freeze = if h.is_extensible(o) {
            0
        } else {
            match self.freeze_line(o, loc) {
                l if l.starts_with("Object.freeze") => 2,
                l if l.starts_with("Object.seal") => 1,
                _ => 0,
            }
        };
        for k in h.own_keys(o) {
            let key = self.literal_key(k);
            let prop = h.get_own_property(o, k);
            let at = format!("{loc}{}", self.key_member(k));
            match prop {
                Some(Property::Data { value, flags }) => {
                    let name = h.atom_name(k);
                    // A method written in place: `m() {...}`.
                    if flags == PropFlags::ALL
                        && let Some(c) = self.node_of(value)
                        && self.inline_method(c, &name, func_flags::KIND_METHOD)
                    {
                        let src = self.fn_source(c).unwrap_or_default();
                        self.nodes[c].loc = at.clone();
                        self.nodes[c].state = State::Done;
                        self.fn_patches(c, Some(&name));
                        items.push_out(src);
                        continue;
                    }
                    let e = self.expr(value, Some(&at), Some(&name));
                    if e.is_empty() {
                        let l = self.expr_or_loc(value);
                        self.patches.push_out(format!("{at} = {l};"));
                        items.push_out(format!("{key}: undefined"));
                    } else if e == key {
                        items.push_out(e);
                    } else {
                        items.push_out(format!("{key}: {e}"));
                    }
                    // `Object.freeze`/`seal` at the end set these attributes.
                    let implied = match freeze {
                        2 => flags == PropFlags::ENUMERABLE,
                        1 => flags == PropFlags::new(true, true, false),
                        _ => false,
                    };
                    if flags != PropFlags::ALL && !implied {
                        let ke = self.key_expr(k);
                        self.patches.push_out(format!(
                            "Object.defineProperty({loc}, {}, {{ writable: {}, enumerable: {}, configurable: {} }});",
                            ke,
                            flags.writable(),
                            flags.enumerable(),
                            flags.configurable()
                        ));
                    }
                }
                Some(Property::Accessor {
                    getter,
                    setter,
                    flags,
                }) => {
                    let name = h.atom_name(k);
                    let g = self
                        .node_of(getter)
                        .filter(|&c| self.inline_method(c, &name, func_flags::KIND_GETTER));
                    let s = self
                        .node_of(setter)
                        .filter(|&c| self.inline_method(c, &name, func_flags::KIND_SETTER));
                    let fits = flags.enumerable()
                        && flags.configurable()
                        && (getter.is_undefined() || g.is_some())
                        && (setter.is_undefined() || s.is_some());
                    if fits {
                        for c in [g, s].into_iter().flatten() {
                            let src = self.fn_source(c).unwrap_or_default();
                            self.nodes[c].loc = format!(
                                "Object.getOwnPropertyDescriptor({loc}, {}).{}",
                                self.key_expr(k),
                                if Some(c) == g { "get" } else { "set" }
                            );
                            self.nodes[c].state = State::Done;
                            items.push_out(src);
                        }
                    } else {
                        items.push_out(format!("{key}: undefined"));
                        let d = self.descriptor(&Property::Accessor {
                            getter,
                            setter,
                            flags,
                        });
                        let ke = self.key_expr(k);
                        self.patches
                            .push_out(format!("Object.defineProperty({loc}, {ke}, {d});"));
                    }
                }
                None => {}
            }
        }
        if !h.is_extensible(o) {
            self.finals.push_out(self.freeze_line(o, loc));
        }
        let _ = n;
        if items.is_empty() {
            "{}".into()
        } else {
            format!("{{ {} }}", items.join(", "))
        }
    }

    /// Whether closure `c` can be written as a method, getter or setter in its holder's
    /// literal: free, written once, of that kind, and its text starts with the key.
    fn inline_method(&self, c: usize, key: &str, kind: u32) -> bool {
        if !self.free_closure(c)
            || self.nodes[c].refs != 1
            || self.nodes[c].state != State::New
            || self.nodes[c].root.is_some()
        {
            return false;
        }
        if self.fn_kind(c) != kind {
            return false;
        }
        self.method_text(c, key, kind)
    }

    /// Whether closure `c`'s text is a method (getter, setter) definition of `key`.
    fn method_text(&self, c: usize, key: &str, kind: u32) -> bool {
        if self.fn_kind(c) != kind {
            return false;
        }
        let Some(src) = self.fn_source(c) else {
            return false;
        };
        let s = match kind {
            func_flags::KIND_GETTER => src.strip_prefix("get"),
            func_flags::KIND_SETTER => src.strip_prefix("set"),
            _ => {
                let s = src.strip_prefix("async").map_or(src.as_str(), |s| {
                    if s.starts_with(char::is_whitespace) {
                        s
                    } else {
                        &src
                    }
                });
                Some(s)
            }
        };
        let Some(s) = s else { return false };
        let s = s.trim_start();
        let s = s.strip_prefix('*').map_or(s, str::trim_start);
        is_ident(key)
            && s.strip_prefix(key)
                .is_some_and(|r| r.trim_start().starts_with('('))
    }
}

fn same_property(h: &Heap, p: &Property, base: &Heap, bp: &Property) -> bool {
    let same = |a: Value, b: Value| {
        a.to_bits() == b.to_bits()
            || match (a.as_string(), b.as_string()) {
                (Some(x), Some(y)) => h.str_to_string(x) == base.str_to_string(y),
                _ => false,
            }
    };
    match (p, bp) {
        (Property::Data { value: a, flags: f }, Property::Data { value: b, flags: g }) => {
            f == g && same(*a, *b)
        }
        (
            Property::Accessor {
                getter: a,
                setter: b,
                flags: f,
            },
            Property::Accessor {
                getter: c,
                setter: d,
                flags: g,
            },
        ) => f == g && same(*a, *c) && same(*b, *d),
        _ => false,
    }
}

fn indent(text: &str) -> String {
    let mut s = String::new();
    for l in text.lines() {
        s.push_str("  ");
        s.push_str(l);
        s.push('\n');
    }
    s
}

/// `$dump.builtin(path)`: resolves a canonical builtin path (D2371) at load time.
fn loader_builtin() -> String {
    let mut roots = String::new();
    for (name, e) in LOADER_ROOTS {
        roots.push_str(&format!("    {name}: () => {e},\n"));
    }
    format!(
        "$dump.builtin = (path) => {{\n  const roots = {{\n{roots}  }};\n  \
         const [, acc, root, rest] = /^(get |set )?(%\\w+%|[\\w$]+)(.*)$/.exec(path);\n  \
         let v = root[0] === \"%\" ? roots[root.slice(1, -1)]() : globalThis[root];\n  \
         const segs = rest.match(/\\.\\[\\[Prototype\\]\\]|\\.[\\w$]+|\\[\"(?:[^\"\\\\]|\\\\.)*\"\\]|\\[Symbol\\.\\w+\\]/g) || [];\n  \
         segs.forEach((s, i) => {{\n    \
         if (s === \".[[Prototype]]\") return void (v = Object.getPrototypeOf(v));\n    \
         const key = s[0] === \".\" ? s.slice(1) : s.startsWith(\"[Symbol.\") ? Symbol[s.slice(8, -1)] : JSON.parse(s.slice(1, -1));\n    \
         if (acc && i === segs.length - 1) {{\n      const d = Object.getOwnPropertyDescriptor(v, key);\n      v = acc === \"get \" ? d.get : d.set;\n    }} else v = v[key];\n  }});\n  \
         return v;\n}};\n"
    )
}
