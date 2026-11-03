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
//!
//! The output follows the order the graph is walked in (roots in order, keys in property
//! order), so it is deterministic without sorting. Text is built with `crate::text`
//! (D2339) and lookups go through one small hash index of cells and atoms: the dumper
//! adds no ordered maps, sorts or `core::fmt` to the module.

use alloc::string::String;
use alloc::vec::Vec;

use super::Vm;
use crate::buf::PushOut;
use crate::bytecode::template::{self, func_flags};
use crate::heap::cells::{BoxCell, FuncTemplate, Object as ObjectCell, U32Array};
use crate::heap::{Atom, Heap, ObjectClass, PropFlags, Property, Ref, atoms};
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
                    match d.globals.iter().position(|g| g.name == n) {
                        Some(g) => d.add_global_root(g),
                        None if d.host_names.contains(&n) => {}
                        None => d.refuse(String::from(n), "no such global"),
                    }
                }
            }
            None => {
                for g in 0..d.globals.len() {
                    d.add_global_root(g);
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
        // The boxes first, so that a closure of one root that captures another root's
        // box refers to that root, whichever comes first.
        for (i, (_, v)) in roots.iter().enumerate() {
            if let Some(b) = v.as_internal() {
                d.cells.insert(key(b), ROOT_BOX | i as u32);
            }
        }
        for (name, v) in roots {
            d.add_root(name, Decl::Let, *v);
        }
        d.explicit = d.roots.len();
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
    /// A captured variable's box (`dump_values`): closures over it read the root.
    boxed: bool,
}

/// A user global (a binding of the global lexical record, or a property of the global
/// object the base does not have).
struct Global {
    name: String,
    value: Value,
    decl: Decl,
    root: Option<usize>,
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

/// How a builtin is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Named {
    /// By its path, an expression on the globals (`Array.prototype.map`).
    Path,
    /// Through `$dump.builtin(path)`.
    Loader,
    /// Not at all: an internal value of a builtin (refused).
    Internal,
    /// Not at all: under a realm root the loader has no expression for (refused).
    Unreachable,
}

struct Builtin {
    cell: u32,
    path: String,
    named: Named,
}

/// What a cell in [`Dumper::cells`] is: the tag in the top two bits, an index below.
const BUILTIN: u32 = 0;
const HOST: u32 = 1 << 30;
const NODE: u32 = 2 << 30;
const ROOT_BOX: u32 = 3 << 30;
const TAG: u32 = 3 << 30;

/// The key of a cell offset or atom id in an [`Index`] (0 is the empty slot).
fn key(n: u32) -> u64 {
    u64::from(n) + 1
}

/// A hash index from nonzero keys to `u32`s: open addressing, linear probing, a
/// power-of-two table at most half full.
#[derive(Default)]
struct Index {
    slots: Vec<(u64, u32)>,
    len: usize,
}

impl Index {
    fn slot(&self, key: u64) -> usize {
        (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as usize & (self.slots.len() - 1)
    }

    fn get(&self, key: u64) -> Option<u32> {
        if self.slots.is_empty() {
            return None;
        }
        let mut i = self.slot(key);
        loop {
            let (k, v) = self.slots[i];
            if k == key {
                return Some(v);
            }
            if k == 0 {
                return None;
            }
            i = (i + 1) & (self.slots.len() - 1);
        }
    }

    /// Sets the value of `key`, replacing an earlier one.
    fn insert(&mut self, key: u64, v: u32) {
        if (self.len + 1) * 2 > self.slots.len() {
            let size = (self.slots.len() * 2).max(64);
            let old = core::mem::replace(&mut self.slots, alloc::vec![(0, 0); size]);
            self.len = 0;
            for (k, v) in old {
                if k != 0 {
                    self.insert(k, v);
                }
            }
        }
        let mut i = self.slot(key);
        loop {
            let k = self.slots[i].0;
            if k == key || k == 0 {
                self.len += usize::from(k == 0);
                self.slots[i] = (key, v);
                return;
            }
            i = (i + 1) & (self.slots.len() - 1);
        }
    }
}

/// The helpers a dump defines on `$dump` when it uses them.
const H_BYTES: u8 = 1;
const H_FN: u8 = 2;
const H_REJECTED: u8 = 4;
const H_BUILTIN: u8 = 8;

const HELPERS: &[(u8, &str)] = &[
    (
        H_BYTES,
        "$dump.bytes = (s) => {\n  const b = atob(s), u = new Uint8Array(b.length);\n  \
         for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);\n  return u.buffer;\n};\n",
    ),
    (
        H_FN,
        "$dump.fn = (o) => {\n  const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]);\n  \
         return d.value || d.get || d.set;\n};\n",
    ),
    (
        H_REJECTED,
        "$dump.rejected = (e) => {\n  const p = Promise.reject(e);\n  p.catch(() => {});\n  return p;\n};\n",
    ),
];

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

/// `$dump.builtin(path)`: resolves a canonical builtin path (D2371) at load time. The
/// realm roots go between these two parts.
const LOADER_HEAD: &str = "$dump.builtin = (path) => {\n  const roots = {\n";
const LOADER_TAIL: &str = "  };\n  \
     const [, acc, root, rest] = /^(get |set )?(%\\w+%|[\\w$]+)(.*)$/.exec(path);\n  \
     let v = root[0] === \"%\" ? roots[root.slice(1, -1)]() : globalThis[root];\n  \
     const segs = rest.match(/\\.\\[\\[Prototype\\]\\]|\\.[\\w$]+|\\[\"(?:[^\"\\\\]|\\\\.)*\"\\]|\\[Symbol\\.\\w+\\]/g) || [];\n  \
     segs.forEach((s, i) => {\n    \
     if (s === \".[[Prototype]]\") return void (v = Object.getPrototypeOf(v));\n    \
     const key = s[0] === \".\" ? s.slice(1) : s.startsWith(\"[Symbol.\") ? Symbol[s.slice(8, -1)] : JSON.parse(s.slice(1, -1));\n    \
     if (acc && i === segs.length - 1) {\n      const d = Object.getOwnPropertyDescriptor(v, key);\n      v = acc === \"get \" ? d.get : d.set;\n    } else v = v[key];\n  });\n  \
     return v;\n};\n";

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

fn is_ident(s: &str) -> bool {
    let mut cs = s.bytes();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == b'_' || c == b'$')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
        && !RESERVED.contains(&s)
}

/// Whether `s` contains `pat` (a plain scan: the needles are two or three bytes).
fn has(s: &str, pat: &str) -> bool {
    s.as_bytes().windows(pat.len()).any(|w| w == pat.as_bytes())
}

fn tf(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

/// `items` separated by `sep`.
fn join(items: &[String], sep: &str) -> String {
    let mut s = String::new();
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            s.push_str(sep);
        }
        s.push_str(it);
    }
    s
}

/// A JavaScript string literal for UTF-16 code units.
fn quote_units(units: impl Iterator<Item = u16>) -> String {
    let mut out = String::from("\"");
    let mut units = units.peekable();
    while let Some(u) = units.next() {
        let c = match u {
            0xD800..=0xDBFF => match units.peek() {
                Some(&lo @ 0xDC00..=0xDFFF) => {
                    units.next();
                    let c = 0x10000 + ((u32::from(u) - 0xD800) << 10) + (u32::from(lo) - 0xDC00);
                    char::from_u32(c).unwrap_or('\u{FFFD}')
                }
                _ => {
                    cat_to!(&mut out, "\\u", crate::text::hex(u, 4));
                    continue;
                }
            },
            0xDC00..=0xDFFF => {
                cat_to!(&mut out, "\\u", crate::text::hex(u, 4));
                continue;
            }
            _ => char::from_u32(u32::from(u)).unwrap_or('\u{FFFD}'),
        };
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' | '\u{2029}' => cat_to!(&mut out, "\\u", crate::text::hex(c as u32, 4)),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                cat_to!(&mut out, "\\x", crate::text::hex(c as u32, 2))
            }
            c => out.push_out(c),
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
        cat!(".", key)
    } else {
        cat!("[", quote(key), "]")
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
    s.get(..end).filter(|n| !n.is_empty())
}

/// The heap cell of an object, symbol or internal value.
fn cell_of(v: Value) -> Option<u32> {
    v.as_object()
        .map(|o| o.offset())
        .or_else(|| v.as_symbol().map(|s| s.offset()))
        .or_else(|| v.as_internal())
}

/// How a builtin's path can be written in a dump.
fn named(path: &str) -> Named {
    let p = path
        .strip_prefix("get ")
        .or_else(|| path.strip_prefix("set "))
        .unwrap_or(path);
    if path.contains('~') || p.starts_with("%intrinsic:") {
        return Named::Internal;
    }
    if let Some(r) = p.strip_prefix('%') {
        let r = r.split('%').next().unwrap_or_default();
        if !LOADER_ROOTS.iter().any(|(n, _)| *n == r) {
            return Named::Unreachable;
        }
    }
    if p.len() != path.len() || p.starts_with('%') || has(p, "[[") {
        Named::Loader
    } else {
        Named::Path
    }
}

struct Dumper<'a> {
    vm: &'a Vm,
    h: &'a Heap,
    base_end: u32,
    /// Builtin cells (below `BASE_END`), host values, nodes and root boxes by cell offset.
    cells: Index,
    builtins: Vec<Builtin>,
    host_exprs: Vec<String>,
    host_names: Vec<&'a str>,
    /// The global object's properties on the base: the builtin globals.
    base_globals: Vec<String>,
    globals: Vec<Global>,
    /// `globals` by atom id.
    global_atoms: Index,
    roots: Vec<Root>,
    /// The roots given by name to `dump_values` (the first ones).
    explicit: usize,
    nodes: Vec<Node>,
    groups: Vec<Group>,
    refusals: Vec<String>,
    /// Builtin properties the session changed: (holder expression, key, new property).
    builtin_patches: Vec<(String, Atom, Option<Property>)>,
    out: Vec<String>,
    /// Per line of `out`, what the checker compiles instead when the line writes no
    /// function: `let name;` for a root, nothing for a `$dump` object (`None`: the line).
    skeleton: Vec<Option<String>>,
    /// Functions whose source was read so far (an over-count: a line that read none
    /// writes none).
    fns_read: core::cell::Cell<u32>,
    /// The base heap: the checker compiles the dump into it.
    scratch: Option<Heap>,
    patches: Vec<String>,
    finals: Vec<String>,
    helpers: u8,
    /// Whether the text uses the `$dump` namespace.
    ns: bool,
    next: u32,
    depth: u32,
    visited: usize,
}

impl<'a> Dumper<'a> {
    fn new(vm: &'a Vm, opts: &DumpOptions<'a>) -> Result<Dumper<'a>, DumpError> {
        let h = &vm.heap;
        let refuse = |m: &str| DumpError(alloc::vec![String::from(m)]);
        let base_end = h.base_end();
        if base_end == 0 || opts.base.len() < base_end as usize {
            return Err(refuse("the VM was not made from this base image"));
        }
        // A private copy, kept for the checker to compile the dump into.
        let base = Heap::from_bytes_trusted(opts.base.to_vec(), crate::heap::DEFAULT_LIMIT)
            .map_err(|_| refuse("the base image does not load"))?;
        let mut d = Dumper {
            vm,
            h,
            base_end,
            cells: Index::default(),
            builtins: Vec::new(),
            host_exprs: Vec::new(),
            host_names: opts.host.to_vec(),
            base_globals: Vec::new(),
            globals: Vec::new(),
            global_atoms: Index::default(),
            roots: Vec::new(),
            explicit: 0,
            nodes: Vec::new(),
            groups: Vec::new(),
            refusals: Vec::new(),
            builtin_patches: Vec::new(),
            out: Vec::new(),
            skeleton: Vec::new(),
            fns_read: core::cell::Cell::new(0),
            scratch: None,
            patches: Vec::new(),
            finals: Vec::new(),
            helpers: 0,
            ns: false,
            next: 0,
            depth: 0,
            visited: 0,
        };
        for (cell, path) in super::registry::reachable_names(&base) {
            if cell < base_end {
                let named = named(&path);
                d.cells.insert(key(cell), BUILTIN | d.builtins.len() as u32);
                d.builtins.push_out(Builtin { cell, path, named });
            }
        }
        if let Some(bg) = intrinsic(&base, Intrinsic::Global).as_object() {
            d.base_globals = base
                .own_keys(bg)
                .into_iter()
                .map(|k| base.atom_name(k))
                .collect();
        }
        d.read_globals(&base);
        d.read_patches(&base);
        d.scratch = Some(base);
        Ok(d)
    }

    fn refuse(&mut self, path: String, why: &str) {
        let line = cat!(path, ": ", why);
        if !self.refusals.contains(&line) {
            self.refusals.push_out(line);
        }
    }

    /// What `cell` is to the dump: its tag and index.
    fn cell(&self, cell: u32) -> Option<(u32, usize)> {
        let t = self.cells.get(key(cell))?;
        Some((t & TAG, (t & !TAG) as usize))
    }

    fn builtin(&self, cell: u32) -> Option<&Builtin> {
        match self.cell(cell)? {
            (BUILTIN, i) => Some(&self.builtins[i]),
            _ => None,
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
                self.global_binding(k, value, decl);
            }
        }
        let bkeys = bg.map(|bg| base.own_keys(bg)).unwrap_or_default();
        for k in h.own_keys(g) {
            if h.atom_symbol(k).is_some() {
                continue;
            }
            let name = h.atom_name(k);
            let Some(p) = h.get_own_property(g, k) else {
                continue;
            };
            // The VM's atom table starts with the base's: the same keys, the same atoms.
            let same = bg
                .filter(|_| bkeys.contains(&k))
                .and_then(|bg| Some(base.get_own_property(bg, k)? == p));
            if same == Some(true) {
                continue;
            }
            match p {
                Property::Data { value, .. } => self.global_binding(k, value, Decl::Var),
                Property::Accessor { .. } => {
                    if !self.host_names.contains(&name.as_str()) {
                        self.refuse(name, "a global accessor");
                    }
                }
            }
        }
    }

    fn global_binding(&mut self, k: Atom, value: Value, decl: Decl) {
        let name = self.h.atom_name(k);
        if self.host_names.contains(&name.as_str()) {
            self.host_values(value, name, 0);
        } else if name != "$dump" && self.global_atoms.get(key(k.id())).is_none() {
            // A lexical binding shadows a global object property of the same name.
            self.global_atoms
                .insert(key(k.id()), self.globals.len() as u32);
            self.globals.push_out(Global {
                name,
                value,
                decl,
                root: None,
            });
        }
    }

    fn host_values(&mut self, v: Value, expr: String, depth: u32) {
        let Some(o) = v.as_object() else { return };
        if o.offset() < self.base_end || self.cell(o.offset()).is_some() {
            return;
        }
        self.cells
            .insert(key(o.offset()), HOST | self.host_exprs.len() as u32);
        self.host_exprs.push_out(expr.clone());
        if depth >= 2 || self.h.object_class(o) == ObjectClass::Proxy {
            return;
        }
        for k in self.h.own_keys(o) {
            if self.h.atom_symbol(k).is_some() {
                continue;
            }
            if let Some(Property::Data { value, .. }) = self.h.get_own_property(o, k) {
                let e = cat!(expr, member(&self.h.atom_name(k)));
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
        // In naming order, so that the patches come out in a stable order.
        for i in 0..self.builtins.len() {
            let cell = self.builtins[i].cell;
            if base.cell_kind(cell) != crate::heap::CellKind::Object {
                continue;
            }
            let o = Obj(cell);
            let path = self.builtins[i].path.clone();
            if h.get_prototype_of(o).map(|p| p.offset())
                != base.get_prototype_of(o).map(|p| p.offset())
            {
                self.refuse(path.clone(), "a builtin whose prototype was changed");
            }
            let keys = h.own_keys(o);
            let bkeys = base.own_keys(o);
            let mut expr = None;
            if h.is_extensible(o) != base.is_extensible(o) {
                let e = self.builtin_expr(i);
                self.finals
                    .push_out(cat!("Object.preventExtensions(", e, ");"));
            }
            for k in &bkeys {
                let patch = match keys.iter().find(|k2| *k2 == k) {
                    None => Some((*k, None)),
                    Some(k2) => {
                        let (Some(p), Some(bp)) =
                            (h.get_own_property(o, *k2), base.get_own_property(o, *k))
                        else {
                            continue;
                        };
                        (!same_property(h, &p, base, &bp)).then_some((*k2, Some(p)))
                    }
                };
                if let Some((k, p)) = patch {
                    let e = expr.get_or_insert_with(|| self.builtin_expr(i)).clone();
                    self.builtin_patches.push_out((e, k, p));
                }
            }
            for k in keys {
                if !bkeys.contains(&k) {
                    let p = h.get_own_property(o, k);
                    let e = expr.get_or_insert_with(|| self.builtin_expr(i)).clone();
                    self.builtin_patches.push_out((e, k, p));
                }
            }
        }
        let patches = core::mem::take(&mut self.builtin_patches);
        for (expr, k, p) in &patches {
            let path = cat!(expr, self.key_path(*k));
            match p {
                Some(Property::Data { value, .. }) => {
                    self.note(*value, || path.clone());
                }
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.note(*getter, || cat!("get ", path));
                    self.note(*setter, || cat!("set ", path));
                }
                None => {}
            }
        }
        self.builtin_patches = patches;
    }

    /// The expression for builtin `i`: its path, or `$dump.builtin(path)`.
    fn builtin_expr(&mut self, i: usize) -> String {
        let b = &self.builtins[i];
        if b.named == Named::Path {
            return b.path.clone();
        }
        let e = cat!("$dump.builtin(", quote(&b.path), ")");
        self.helpers |= H_BUILTIN;
        self.ns = true;
        e
    }

    // ---------------------------------------------------------------- discovery

    fn add_global_root(&mut self, g: usize) {
        if self.globals[g].root.is_some() {
            return;
        }
        let Global {
            ref name,
            value,
            decl,
            ..
        } = self.globals[g];
        let name = name.clone();
        if self.roots[..self.explicit].iter().any(|r| r.name == name) {
            return self.refuse(
                name,
                "a global that a dumped binding of the same name hides",
            );
        }
        self.globals[g].root = Some(self.roots.len());
        self.push_root(name, decl, value);
        self.drain();
    }

    fn add_root(&mut self, name: &str, decl: Decl, v: Value) {
        if self.roots.iter().any(|r| r.name == name) {
            return;
        }
        if name.starts_with("$dump") {
            return self.refuse(name.into(), "the name is the dump's own");
        }
        let value = match v.as_internal() {
            Some(b) => self.h.arena().get(b, BoxCell::VALUE),
            None => v,
        };
        self.push_root(name.into(), decl, value);
        if let Some(r) = self.roots.last_mut() {
            r.boxed = v.as_internal().is_some();
        }
        self.drain();
    }

    /// Adds a root and notes its value (visited by the next `drain`).
    fn push_root(&mut self, name: String, decl: Decl, value: Value) {
        let i = self.roots.len();
        if value.is_uninitialized() {
            self.refuse(name.clone(), "a binding before its declaration ran (TDZ)");
        } else if let Some(n) = self.note(value, || name.clone())
            && self.nodes[n].root.is_none()
        {
            self.nodes[n].root = Some(i);
        }
        self.roots.push_out(Root {
            name,
            decl,
            value,
            state: State::New,
            boxed: false,
        });
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
            | Kind::String
            | Kind::Hole => return None,
            _ => {
                self.refuse(path(), "an internal value");
                return None;
            }
        }
        let c = cell_of(v)?;
        match self.cell(c) {
            Some((BUILTIN, i)) => {
                let b = &self.builtins[i];
                let why = match b.named {
                    Named::Path | Named::Loader => return None,
                    Named::Internal => "a builtin's internal value (",
                    Named::Unreachable => "a builtin the loader cannot name (",
                };
                let why = cat!(why, b.path, ")");
                self.refuse(path(), &why);
                return None;
            }
            Some((NODE, n)) => {
                self.nodes[n].refs += 1;
                return Some(n);
            }
            Some(_) => return None,
            None => {}
        }
        if c == self.vm.global().offset() {
            return None;
        }
        if c < self.base_end {
            self.refuse(path(), "a builtin without a name");
            return None;
        }
        let n = self.nodes.len();
        self.cells.insert(key(c), NODE | n as u32);
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

    fn edge(&mut self, from: usize, v: Value, path: impl FnOnce() -> String) -> Option<usize> {
        let n = self.note(v, path)?;
        self.nodes[from].edges.push_out(n);
        Some(n)
    }

    /// A key as a path segment: `.name`, `["a b"]`, `[Symbol.iterator]`, `[Symbol(tag)]`.
    fn key_path(&self, k: Atom) -> String {
        let h = self.h;
        match h.atom_symbol(k) {
            Some(s) => match self.builtin(s.offset()) {
                Some(b) => cat!("[", b.path, "]"),
                None => {
                    let d = h.symbol_description(s).as_string();
                    cat!(
                        "[Symbol(",
                        d.map(|d| h.str_to_string(d)).unwrap_or_default(),
                        ")]"
                    )
                }
            },
            None => member(&h.atom_name(k)),
        }
    }

    /// A key as code after an expression: `.name`, `["a b"]`, `[sym]`.
    fn key_member(&mut self, k: Atom) -> String {
        match self.h.atom_symbol(k) {
            Some(s) => cat!("[", self.expr(Value::symbol(s), None, None), "]"),
            None => member(&self.h.atom_name(k)),
        }
    }

    /// The edges of `o`'s own properties (but the ones `skip` takes: name and array
    /// index) and of a prototype its literal or constructor would not give it.
    fn props(&mut self, n: usize, o: Obj, skip: &dyn Fn(Atom, Option<u32>) -> bool) {
        let h = self.h;
        for k in h.own_keys(o) {
            if h.is_private_atom(k) {
                let p = self.nodes[n].path.clone();
                self.refuse(p, "an object with private fields");
                continue;
            }
            if h.atom_symbol(k).is_none() && skip(k, h.atom_array_index(k)) {
                continue;
            }
            self.key_edge(n, k);
            let prop = h.get_own_property(o, k);
            // Paths only for what may become a node: most values are primitives.
            let refs = match prop {
                Some(Property::Data { value, .. }) => cell_of(value).is_some(),
                Some(Property::Accessor { .. }) => true,
                None => false,
            };
            if !refs {
                continue;
            }
            let path = cat!(self.nodes[n].path, self.key_path(k));
            match prop {
                Some(Property::Data { value, .. }) => {
                    self.edge(n, value, || path);
                }
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.edge(n, getter, || cat!("get ", path));
                    self.edge(n, setter, || cat!("set ", path));
                }
                None => {}
            }
        }
        if let Some(p) = h.get_prototype_of(o)
            && !self.default_proto(o, Some(p))
        {
            let path = cat!(self.nodes[n].path, ".[[Prototype]]");
            self.edge(n, Value::object(p), || path);
        }
    }

    /// The edge of a symbol key. A key may be written twice (in a literal and in an
    /// attribute patch), so it counts twice: a unique symbol is made once, by name.
    fn key_edge(&mut self, n: usize, k: Atom) {
        if let Some(s) = self.h.atom_symbol(k) {
            let base = self.nodes[n].path.clone();
            if let Some(c) = self.edge(n, Value::symbol(s), || cat!(base, "<key>")) {
                self.nodes[c].refs += 1;
            }
        }
    }

    /// The prototype a literal or constructor call of `o`'s class gives it.
    fn class_proto(&self, o: Obj) -> Option<Intrinsic> {
        let h = self.h;
        Some(match h.object_class(o) {
            ObjectClass::Ordinary => Intrinsic::ObjectProto,
            ObjectClass::Array => Intrinsic::ArrayProto,
            ObjectClass::Function => {
                let t = h.closure_template(o);
                let f = u32::from(h.arena().get::<u16>(t, FuncTemplate::FLAGS));
                match (f & func_flags::ASYNC != 0, f & func_flags::GENERATOR != 0) {
                    (false, false) => Intrinsic::FunctionProto,
                    (true, false) => Intrinsic::AsyncFunctionProto,
                    (false, true) => Intrinsic::GeneratorFunctionProto,
                    (true, true) => Intrinsic::AsyncGeneratorFunctionProto,
                }
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
        })
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
                Some(i) => intrinsic(h, i).as_object() == p,
                None => true,
            },
        }
    }

    fn visit(&mut self, n: usize) {
        let h = self.h;
        let v = self.nodes[n].v;
        let path = self.nodes[n].path.clone();
        if let Some(b) = v.as_internal() {
            let inner: Value = h.arena().get(b, BoxCell::VALUE);
            if inner.is_uninitialized() {
                self.refuse(path, "a captured binding before its declaration ran (TDZ)");
                return;
            }
            self.edge(n, inner, || path);
            return;
        }
        let Some(o) = v.as_object() else {
            return; // A symbol.
        };
        match h.object_class(o) {
            ObjectClass::Ordinary => self.props(n, o, &|_, _| false),
            ObjectClass::Array => {
                let len = h.array_length(o);
                let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
                for i in 0..len {
                    let v = match el.get() {
                        Some(el) if i < h.value_array_len(el) => h.value_array_get(el, i),
                        _ => Value::HOLE,
                    };
                    self.edge(n, v, || cat!(path, "[", i, "]"));
                }
                // Sparse elements and named properties.
                let dense = el.get().map_or(0, |el| h.value_array_len(el));
                self.props(n, o, &|k, i| {
                    k == atoms::LENGTH || i.is_some_and(|i| i < dense)
                });
            }
            ObjectClass::Function => self.closure(n, o),
            ObjectClass::Map | ObjectClass::Set => {
                let map = h.object_class(o) == ObjectClass::Map;
                for (i, (k, v)) in h.collection_entries_list(o).into_iter().enumerate() {
                    let p = cat!(path, "<entries>[", i, "]");
                    self.edge(n, k, || p.clone());
                    if map {
                        self.edge(n, v, || p);
                    }
                }
                self.props(n, o, &|_, _| false);
            }
            ObjectClass::Date | ObjectClass::Boolean | ObjectClass::Number => {
                self.props(n, o, &|_, _| false)
            }
            ObjectClass::String => self.props(n, o, &|k, i| k == atoms::LENGTH || i.is_some()),
            ObjectClass::RegExp => self.props(n, o, &|k, _| k == atoms::LAST_INDEX),
            ObjectClass::Error => {
                self.props(n, o, &|k, _| k == atoms::MESSAGE || k == atoms::STACK)
            }
            ObjectClass::Promise => {
                let state = h.object_slot(o, 0).as_int().unwrap_or(0);
                if state == crate::interp::run::promise::PENDING {
                    self.refuse(path, "a pending promise");
                    return;
                }
                let r = h.object_slot(o, 1);
                self.edge(n, r, || cat!(path, "<result>"));
                self.props(n, o, &|_, _| false);
            }
            ObjectClass::BoundFunction => {
                self.edge(n, h.object_slot(o, 0), || cat!(path, "<target>"));
                self.edge(n, h.object_slot(o, 1), || cat!(path, "<this>"));
                if let Some(args) = h.object_slot(o, 2).as_object() {
                    let len = h.array_length(args);
                    let el: Ref = h.arena().get(args.offset(), ObjectCell::ELEMENTS);
                    for i in 0..len {
                        let v = el
                            .get()
                            .map_or(Value::UNDEFINED, |el| h.value_array_get(el, i));
                        self.edge(n, v, || cat!(path, "<args>[", i, "]"));
                    }
                }
                self.props(n, o, &|k, _| k == atoms::NAME || k == atoms::LENGTH);
            }
            #[cfg(feature = "full")]
            ObjectClass::ArrayBuffer => {
                if h.object_slot(o, 1).is_number() {
                    self.refuse(path, "a resizable ArrayBuffer");
                }
                self.props(n, o, &|_, _| false);
            }
            #[cfg(feature = "full")]
            ObjectClass::TypedArray | ObjectClass::DataView => {
                let b = h.object_slot(o, 0);
                self.edge(n, b, || cat!(path, ".buffer"));
                self.props(n, o, &|_, i| i.is_some());
            }
            ObjectClass::HostFunction => {
                self.refuse(path, "a host function the host does not re-bind by name")
            }
            ObjectClass::Generator => self.refuse(path, "a suspended generator or async function"),
            ObjectClass::Proxy => self.refuse(path, "a Proxy"),
            ObjectClass::WeakMap | ObjectClass::WeakSet => self.refuse(path, "a weak collection"),
            _ => self.refuse(path, "an object of a kind dumps do not write"),
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
            let p = cat!(path, "~", name);
            if !is_ident(&name) {
                self.refuse(p, "a captured `this`, `arguments` or `new.target`");
                continue;
            }
            if let Some((ROOT_BOX, r)) = self.cell(b) {
                if self.roots.get(r).is_some_and(|r| r.name != name) {
                    self.refuse(p, "captures a root under another name");
                }
                continue;
            }
            self.edge(n, Value::internal(b), || p);
        }
        // The user globals its code reads by name are part of the dump.
        self.global_refs(t);
        for k in h.own_keys(o) {
            if h.atom_symbol(k).is_none() && self.default_fn_key(o, k) {
                continue;
            }
            self.key_edge(n, k);
            let p = cat!(path, self.key_path(k));
            match h.get_own_property(o, k) {
                Some(Property::Data { value, .. }) => {
                    self.edge(n, value, || p);
                }
                Some(Property::Accessor { getter, setter, .. }) => {
                    self.edge(n, getter, || cat!("get ", p));
                    self.edge(n, setter, || cat!("set ", p));
                }
                None => {}
            }
        }
        if !self.default_proto(o, h.get_prototype_of(o)) {
            self.refuse(path, "a function with another prototype");
        }
    }

    /// Whether own property `k` of function `f` is one its definition makes.
    fn default_fn_key(&self, f: Obj, k: Atom) -> bool {
        k == atoms::LENGTH
            || k == atoms::NAME
            || (k == atoms::PROTOTYPE && self.default_fn_prototype(f))
    }

    /// A function's `prototype` as its definition makes it: absent, or an object with
    /// only `constructor` (back to the function), or an empty generator prototype.
    fn default_fn_prototype(&self, f: Obj) -> bool {
        let h = self.h;
        let Some(atom) = h.own_keys(f).into_iter().find(|k| *k == atoms::PROTOTYPE) else {
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
                && keys[0] == atoms::CONSTRUCTOR
                && h.get_own_property(p, keys[0]).and_then(|p| p.value()) == Some(Value::object(f)))
    }

    fn template_names(&self, t: u32) -> Vec<String> {
        template_names(self.h, t)
    }

    /// Adds the user globals template `t` (and its children) reads or writes by name
    /// as roots, visited after the current node.
    fn global_refs(&mut self, t: u32) {
        let h = self.h;
        for insn in crate::bytecode::insns(template::code(h, t)).flatten() {
            if matches!(
                insn.info().name,
                "get_global" | "get_global_typeof" | "put_global"
            ) && let Some(g) = self.global_atoms.get(key(insn.atom(0).id()))
            {
                let g = g as usize;
                if self.globals[g].root.is_none() {
                    let name = self.globals[g].name.clone();
                    if self.roots[..self.explicit].iter().any(|r| r.name == name) {
                        continue;
                    }
                    self.globals[g].root = Some(self.roots.len());
                    let (decl, value) = (self.globals[g].decl, self.globals[g].value);
                    self.push_root(name, decl, value);
                }
            }
        }
        for i in 0..template::child_count(h, t) {
            if let Some(c) = template::child(h, t, i).and_then(|r| r.get()) {
                self.global_refs(c);
            }
        }
    }

    // ---------------------------------------------------------------- analysis

    /// Closure groups (closures that share a captured box, transitively) and the nodes
    /// on cycles (Tarjan's strongly connected components, iterative).
    fn analyze(&mut self) {
        let n = self.nodes.len();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        // Union-find over closures and the boxes they capture. A closure without boxes
        // is free: no group.
        let is_box: Vec<bool> = self
            .nodes
            .iter()
            .map(|d| d.v.as_internal().is_some())
            .collect();
        let mut parent: Vec<usize> = (0..n).collect();
        let mut grouped = alloc::vec![false; n];
        for (i, d) in self.nodes.iter().enumerate() {
            if is_box[i] {
                grouped[i] = true;
                continue;
            }
            for &e in &d.edges {
                // Only closures have box edges.
                if is_box[e] {
                    grouped[i] = true;
                    let (a, b) = (find(&mut parent, i), find(&mut parent, e));
                    parent[a] = b;
                }
            }
        }
        let mut group_of_set = alloc::vec![usize::MAX; n];
        let mut group = alloc::vec![usize::MAX; n];
        for (i, g) in group.iter_mut().enumerate() {
            if !grouped[i] {
                continue;
            }
            let r = find(&mut parent, i);
            if group_of_set[r] == usize::MAX {
                group_of_set[r] = self.groups.len();
                self.groups.push_out(Group {
                    closures: Vec::new(),
                    boxes: Vec::new(),
                    state: State::New,
                    loc: String::new(),
                });
            }
            *g = group_of_set[r];
            let gr = &mut self.groups[*g];
            if is_box[i] {
                gr.boxes.push_out(i);
            } else {
                gr.closures.push_out(i);
            }
        }
        for (d, g) in self.nodes.iter_mut().zip(group) {
            d.group = g;
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
            let target = cat!(expr, self.key_member(k));
            builtin_lines.push_out(match p {
                None => cat!("delete ", target, ";"),
                Some(Property::Data { value, flags }) if flags == PropFlags::ALL => {
                    cat!(target, " = ", self.expr(value, None, None), ";")
                }
                Some(p) => {
                    let key = self.key_expr(k);
                    let d = self.descriptor(&p);
                    cat!("Object.defineProperty(", expr, ", ", key, ", ", d, ");")
                }
            });
        }
        if !self.refusals.is_empty() {
            return Err(DumpError(self.refusals));
        }
        let mut s = String::from(HEADER);
        let count = self.roots.len();
        cat_to!(
            &mut s,
            ": ",
            count,
            if count == 1 {
                " binding ("
            } else {
                " bindings ("
            }
        );
        for (i, r) in self.roots.iter().enumerate() {
            cat_to!(&mut s, if i > 0 { ", " } else { "" }, r.name);
        }
        s.push_str(
            ").\n// Run it as a Script to recreate them (djs REPL: .restore FILE). Builtins are\n\
             // referenced by name, never written",
        );
        if !self.host_names.is_empty() {
            s.push_str("; the host's ");
            for (i, n) in self.host_names.iter().enumerate() {
                cat_to!(&mut s, if i > 0 { ", " } else { "" }, *n);
            }
            s.push_str(" are re-bound by name");
        }
        s.push_str(".\n");
        if self.ns {
            s.push_str("globalThis.$dump = {};\n");
            for (bit, text) in HELPERS {
                if self.helpers & bit != 0 {
                    s.push_str(text);
                }
            }
            if self.helpers & H_BUILTIN != 0 {
                s.push_str(LOADER_HEAD);
                for (name, e) in LOADER_ROOTS {
                    cat_to!(&mut s, "    ", *name, ": () => ", *e, ",\n");
                }
                s.push_str(LOADER_TAIL);
            }
        }
        for l in &self.out {
            cat_to!(&mut s, l, '\n');
        }
        if !builtin_lines.is_empty() {
            s.push_str("// Builtins the session changed.\n");
            for l in &builtin_lines {
                cat_to!(&mut s, l, '\n');
            }
        }
        if !self.patches.is_empty() || !self.finals.is_empty() {
            s.push_str("// Links that close cycles, and attributes.\n");
            for l in self.patches.iter().chain(&self.finals) {
                cat_to!(&mut s, l, '\n');
            }
        }
        if self.ns {
            s.push_str("void delete globalThis.$dump;\n");
        }
        // The checker compiles the dump with the lines that write no function cut down to
        // the names they declare: their data cannot change what a name resolves to.
        let mut skeleton = String::new();
        for (l, sk) in self.out.iter().zip(&self.skeleton) {
            cat_to!(&mut skeleton, sk.as_deref().unwrap_or(l), '\n');
        }
        for l in builtin_lines
            .iter()
            .chain(&self.patches)
            .chain(&self.finals)
        {
            cat_to!(&mut skeleton, l, '\n');
        }
        self.check(&skeleton);
        if !self.refusals.is_empty() {
            return Err(DumpError(self.refusals));
        }
        Ok(s)
    }

    /// The checker (0168): compiles the finished dump and compares every function it
    /// writes with the original. A name the original captured must be captured in the
    /// dump too (a variable of its factory) or be a root that holds that variable's box;
    /// a global the original read must stay a global (no factory variable hides it) and
    /// exist after loading if it existed before (a root, a host global, a builtin global).
    /// Anything else would load as a function that reads another binding than it did, so
    /// the dump is refused instead.
    fn check(&mut self, text: &str) {
        let h = self.h;
        let fns: Vec<usize> = (0..self.nodes.len())
            .filter(|&n| {
                self.nodes[n].state == State::Done
                    && self.nodes[n]
                        .v
                        .as_object()
                        .is_some_and(|o| h.object_class(o) == ObjectClass::Function)
            })
            .collect();
        if fns.is_empty() {
            return;
        }
        let compiled = self.scratch.take().map(|mut scratch| {
            let opts = crate::compiler::CompileOptions::script("dump.js");
            let r = crate::compiler::compile(&mut scratch, text.as_bytes(), &opts);
            (scratch, r)
        });
        let (scratch, t) = match compiled {
            Some((scratch, Ok(t))) => match t.get() {
                Some(t) => (scratch, t),
                None => return self.refuse("the dump".into(), "does not compile"),
            },
            Some((_, Err(e))) => {
                let why = cat!("does not compile: ", e.message);
                return self.refuse("the dump".into(), &why);
            }
            None => return self.refuse("the dump".into(), "no memory to check it"),
        };
        // Every function of the dump, by the hash of its text.
        let mut by_text = Index::default();
        let mut written: Vec<(u32, u32, FreeNames, u32)> = Vec::new();
        let mut stack = alloc::vec![t];
        while let Some(t) = stack.pop() {
            let info = template::debug_info(&scratch, t);
            let (a, b) = (info.src_start, info.src_end);
            if let Some(src) = text.get(a as usize..b as usize) {
                let key = hash_text(src);
                let names = free_names(&scratch, t);
                let first = by_text.get(key).unwrap_or(u32::MAX);
                // One entry per text and free names: a thousand closures of one factory
                // function are one entry.
                let mut at = first;
                let mut seen = false;
                while let Some((a2, b2, n2, next)) = written.get(at as usize) {
                    if n2.captures == names.captures
                        && n2.globals == names.globals
                        && text.get(*a2 as usize..*b2 as usize) == Some(src)
                    {
                        seen = true;
                        break;
                    }
                    at = *next;
                }
                if !seen {
                    by_text.insert(key, written.len() as u32);
                    written.push_out((a, b, names, first));
                }
            }
            for i in 0..template::child_count(&scratch, t) {
                if let Some(c) = template::child(&scratch, t, i).and_then(|r| r.get()) {
                    stack.push_out(c);
                }
            }
        }
        for n in fns {
            let o = self.nodes[n].v.as_object().expect("a function");
            let Some(src) = self.fn_source(n) else {
                continue;
            };
            let orig = free_names(h, h.closure_template(o));
            let path = self.nodes[n].path.clone();
            let mut at = by_text.get(hash_text(&src)).unwrap_or(u32::MAX);
            let mut found = false;
            while let Some(&(a, b, ref new, next)) = written.get(at as usize) {
                at = next;
                if text.get(a as usize..b as usize) != Some(src.as_str()) {
                    continue;
                }
                found = true;
                for c in &orig.captures {
                    // Or read as a global: a root holding the variable's box, or a root
                    // or the host's global of the same name holding its current value
                    // (the factory leaves such a variable to the global).
                    let value = captured_value(h, o, c);
                    let ok = new.captures.contains(c)
                        || (new.globals.contains(c)
                            && (self.host_names.contains(&c.as_str())
                                || self
                                    .roots
                                    .iter()
                                    .any(|r| r.name == *c && (r.boxed || Some(r.value) == value))));
                    if !ok {
                        let why = cat!(
                            "captures `",
                            c,
                            "`, which is not in scope where the dump writes the function"
                        );
                        self.refuse(path.clone(), &why);
                    }
                }
                for g in &orig.globals {
                    if new.captures.contains(g) {
                        let why = cat!(
                            "reads the global `",
                            g,
                            "`, which a variable of the dump hides"
                        );
                        self.refuse(path.clone(), &why);
                    } else if !self.roots.iter().any(|r| r.name == *g)
                        && !self.host_names.contains(&g.as_str())
                        && !self.base_globals.contains(g)
                        && self.globals.iter().any(|u| u.name == *g)
                    {
                        let why = cat!("reads the global `", g, "`, which the dump does not hold");
                        self.refuse(path.clone(), &why);
                    }
                }
            }
            if !found {
                self.refuse(path, "internal: the function's text is not in the dump");
            }
        }
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.next += 1;
        self.ns = true;
        cat!("$dump.", prefix, self.next)
    }

    fn key_expr(&mut self, k: Atom) -> String {
        match self.h.atom_symbol(k) {
            Some(s) => self.expr(Value::symbol(s), None, None),
            None => quote(&self.h.atom_name(k)),
        }
    }

    /// The key in an object literal: `name`, `"a b"`, `[Symbol.iterator]`.
    fn literal_key(&mut self, k: Atom) -> String {
        let h = self.h;
        match h.atom_symbol(k) {
            Some(s) => cat!("[", self.expr(Value::symbol(s), None, None), "]"),
            None => {
                let name = h.atom_name(k);
                if name == "__proto__" {
                    cat!("[", quote(&name), "]")
                } else if is_ident(&name) || h.atom_array_index(k).is_some() {
                    name
                } else {
                    quote(&name)
                }
            }
        }
    }

    fn descriptor(&mut self, p: &Property) -> String {
        match *p {
            Property::Data { value, flags } => cat!(
                "{ value: ",
                self.expr(value, None, None),
                ", writable: ",
                tf(flags.writable()),
                ", enumerable: ",
                tf(flags.enumerable()),
                ", configurable: ",
                tf(flags.configurable()),
                " }"
            ),
            Property::Accessor {
                getter,
                setter,
                flags,
            } => {
                let g = self.expr(getter, None, None);
                cat!(
                    "{ get: ",
                    g,
                    ", set: ",
                    self.expr(setter, None, None),
                    ", enumerable: ",
                    tf(flags.enumerable()),
                    ", configurable: ",
                    tf(flags.configurable()),
                    " }"
                )
            }
        }
    }

    fn emit_root(&mut self, r: usize) {
        if self.roots[r].state != State::New {
            return;
        }
        self.roots[r].state = State::Busy;
        let fns_before = self.fns_read.get();
        let name = self.roots[r].name.clone();
        let value = self.roots[r].value;
        let kw = match self.roots[r].decl {
            Decl::Const => "const ",
            Decl::Let => "let ",
            Decl::Var => "var ",
        };
        let line = match self.node_of(value) {
            // A closure of a scope group is made by its group's factory (`expr` emits it);
            // only free values are written in place (0168: a root that was itself a
            // closure over a parameter came out as its bare text).
            Some(n)
                if self.nodes[n].root == Some(r)
                    && self.nodes[n].state == State::New
                    && self.nodes[n].group == usize::MAX =>
            {
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
                    cat!(kw, name, " = ", e, ";")
                }
            }
            _ => {
                let e = self.expr(value, None, Some(&name));
                if e == name {
                    // The host's value under its own name (a binding `text = text`):
                    // the global is the binding.
                    self.roots[r].state = State::Done;
                    return;
                }
                cat!(kw, name, " = ", e, ";")
            }
        };
        self.out.push_out(line);
        self.skeleton
            .push_out((self.fns_read.get() == fns_before).then(|| cat!("let ", name, ";")));
        self.roots[r].state = State::Done;
    }

    fn node_of(&self, v: Value) -> Option<usize> {
        match self.cell(cell_of(v)?)? {
            (NODE, n) => Some(n),
            _ => None,
        }
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
        self.fns_read.set(self.fns_read.get() + 1);
        crate::builtins::function::source_slice(self.h, self.nodes[n].v.as_object()?)
    }

    /// The expression for a value, in a context that may give a name (`ctx_name`: the
    /// binding or key an anonymous function would be named after) and a location
    /// (`loc`: an expression that reaches the value afterwards, for patches). Empty for
    /// a value on a cycle that is being written: the caller patches it in later.
    fn expr(&mut self, v: Value, loc: Option<&str>, ctx_name: Option<&str>) -> String {
        match v.kind() {
            Kind::Undefined => return "undefined".into(),
            Kind::Null => return "null".into(),
            Kind::Bool => return tf(v.as_bool().unwrap_or(false)).into(),
            Kind::Number => return number(v.as_number().unwrap_or(f64::NAN)),
            Kind::String => return quote_units(self.h.str_units(v.as_string().expect("a string"))),
            Kind::BigInt => return cat!(self.h.bigint(v).to_string_radix(10), "n"),
            Kind::Hole => return String::new(),
            _ => {}
        }
        let Some(c) = cell_of(v) else {
            return "undefined".into();
        };
        let n = match self.cell(c) {
            Some((BUILTIN, i)) => return self.builtin_expr(i),
            Some((HOST, i)) => return self.host_exprs[i].clone(),
            Some((NODE, n)) => n,
            _ if c == self.vm.global().offset() => return "globalThis".into(),
            _ => return "undefined".into(),
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
            self.depth += 1;
            let e = self.render(n, loc, ctx_name);
            self.depth -= 1;
            return e;
        }
        let name = self.fresh("n");
        let fns_before = self.fns_read.get();
        let e = self.render(n, Some(&name), None);
        self.out.push_out(cat!(name, " = ", e, ";"));
        self.skeleton
            .push_out((self.fns_read.get() == fns_before).then(String::new));
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
        let class = h.object_class(o);
        match class {
            ObjectClass::Ordinary | ObjectClass::Array => {
                h.own_keys(o).into_iter().all(|k| {
                    let Some(Property::Data { value, flags }) = h.get_own_property(o, k) else {
                        return false;
                    };
                    let attrs = flags == PropFlags::ALL
                        || (class == ObjectClass::Array && k == atoms::LENGTH);
                    attrs
                        && self.node_of(value).is_none_or(|c| {
                            let d = &self.nodes[c];
                            d.state == State::Done
                                || (d.refs == 1
                                    && !d.cyclic
                                    && self.simple(c, Some(&h.atom_name(k))))
                        })
                }) && h
                    .get_prototype_of(o)
                    .is_some_and(|p| self.default_proto(o, Some(p)))
            }
            ObjectClass::Function => {
                self.fn_name_matches(n, ctx_name)
                    && h.own_keys(o).into_iter().all(|k| self.default_fn_key(o, k))
            }
            ObjectClass::ArrayBuffer
            | ObjectClass::Date
            | ObjectClass::RegExp
            | ObjectClass::TypedArray
            | ObjectClass::DataView
            | ObjectClass::Boolean
            | ObjectClass::Number => {
                h.own_keys(o).into_iter().all(|k| {
                    h.atom_array_index(k).is_some()
                        || (k == atoms::LAST_INDEX
                            && h.get_own_property(o, k).and_then(|p| p.value())
                                == Some(Value::int(0)))
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
            .find(|k| *k == atoms::NAME)
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
            let name = quote(&self.fn_name(n));
            self.patches.push_out(cat!(
                "Object.defineProperty(",
                loc,
                ", \"name\", { value: ",
                name,
                " });"
            ));
        }
        for k in h.own_keys(o) {
            if h.atom_symbol(k).is_none() && self.default_fn_key(o, k) {
                continue;
            }
            if let Some(p) = h.get_own_property(o, k) {
                self.prop_patch(&loc, k, &p);
            }
        }
        if !h.is_extensible(o) {
            self.finals
                .push_out(cat!("Object.preventExtensions(", loc, ");"));
        }
    }

    fn prop_patch(&mut self, loc: &str, k: Atom, p: &Property) {
        let line = match *p {
            Property::Data { value, flags } if flags == PropFlags::ALL => {
                let target = cat!(loc, self.key_member(k));
                let e = self.expr(value, Some(&target), None);
                cat!(target, " = ", e, ";")
            }
            _ => {
                let key = self.key_expr(k);
                let d = self.descriptor(p);
                cat!("Object.defineProperty(", loc, ", ", key, ", ", d, ");")
            }
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
        let closures = self.groups[g].closures.clone();
        let boxes = self.groups[g].boxes.clone();
        let box_value = |d: &Self, b: usize| -> Value {
            h.arena()
                .get(d.nodes[b].v.as_internal().expect("a box"), BoxCell::VALUE)
        };
        // The boxes' variable names (their paths end in `~name`).
        let mut box_names: Vec<String> = Vec::new();
        let mut taken: Vec<String> = Vec::new();
        for &b in &boxes {
            let p = &self.nodes[b].path;
            let name = String::from(p.rsplit('~').next().unwrap_or_default());
            if taken.contains(&name) {
                let path = p.clone();
                self.refuse(
                    path,
                    "two captured variables of the same name in one scope group",
                );
            }
            taken.push_out(name.clone());
            box_names.push_out(name);
        }
        // Each closure's home: a box of the group that holds it under its name, else a
        // key of the returned object.
        let mut home: Vec<Option<usize>> = alloc::vec![None; closures.len()];
        for (bi, &b) in boxes.iter().enumerate() {
            if let Some(c) = self.node_of(box_value(self, b))
                && let Some(j) = closures.iter().position(|&x| x == c)
                && home[j].is_none()
            {
                home[j] = Some(bi);
            }
        }
        let mut keys: Vec<String> = Vec::new();
        for (j, &c) in closures.iter().enumerate() {
            let key = match home[j] {
                Some(bi) => box_names[bi].clone(),
                None => {
                    let fname = self.fn_name(c);
                    let rname = self.nodes[c].root.map(|r| self.roots[r].name.clone());
                    if is_ident(&fname) && !taken.contains(&fname) {
                        fname
                    } else if let Some(r) = rname.filter(|r| is_ident(r) && !taken.contains(r)) {
                        r
                    } else {
                        cat!("$f", c)
                    }
                }
            };
            taken.push_out(key.clone());
            self.nodes[c].loc = cat!(loc, ".", key);
            self.nodes[c].state = State::Busy;
            keys.push_out(key);
        }
        for &b in &boxes {
            self.nodes[b].state = State::Busy;
        }
        let mut text = cat!(
            "// ",
            join(&keys, ", "),
            " sharing ",
            join(&box_names, ", ")
        );
        cat_to!(&mut text, "\n", loc, " = (() => {\n");
        let mut sets: Vec<(usize, Value)> = Vec::new();
        for (bi, &b) in boxes.iter().enumerate() {
            let name = &box_names[bi];
            let inner = box_value(self, b);
            let c = self.node_of(inner);
            let at_home = c.and_then(|c| closures.iter().position(|&x| x == c));
            let line = match (c, at_home) {
                (Some(c), Some(j)) if home[j] == Some(bi) => {
                    let src = self.fn_source(c).unwrap_or_default();
                    if declared_name(self.fn_kind(c), &src) == Some(name.as_str()) {
                        src
                    } else {
                        let e = self.fn_expr(c, &src);
                        cat!("let ", name, " = ", e, ";")
                    }
                }
                (Some(c), _) if self.nodes[c].state == State::Busy => {
                    sets.push_out((bi, inner));
                    cat!("let ", name, ";")
                }
                _ => {
                    let e = self.expr(inner, None, Some(name));
                    if e == *name {
                        // A captured `tools` that holds the host's `tools`: the closures
                        // read the global (`let tools = tools;` would be a TDZ error).
                        continue;
                    }
                    cat!("let ", name, " = ", e, ";")
                }
            };
            for l in line.split('\n') {
                cat_to!(&mut text, "  ", l, '\n');
            }
        }
        let mut ret: Vec<String> = Vec::new();
        for (j, &c) in closures.iter().enumerate() {
            let key = &keys[j];
            if home[j].is_some() {
                ret.push_out(key.clone());
            } else {
                let src = self.fn_source(c).unwrap_or_default();
                if self.method_text(c, key, func_flags::KIND_METHOD) {
                    ret.push_out(src);
                } else {
                    let e = self.fn_expr(c, &src);
                    ret.push_out(cat!(key, ": ", e));
                }
            }
        }
        if !sets.is_empty() {
            let mut s = String::from("$set(v) { ({ ");
            for (i, (bi, _)) in sets.iter().enumerate() {
                cat_to!(&mut s, if i > 0 { ", " } else { "" }, box_names[*bi]);
            }
            s.push_str(" } = v); }");
            ret.push_out(s);
        }
        cat_to!(&mut text, "  return { ", join(&ret, ", "), " };\n})();");
        self.out.push_out(text);
        self.skeleton.push_out(None);
        for &b in &boxes {
            self.nodes[b].state = State::Done;
        }
        for (j, &c) in closures.iter().enumerate() {
            self.nodes[c].state = State::Done;
            self.fn_patches(c, Some(&keys[j]));
        }
        self.groups[g].state = State::Done;
        if !sets.is_empty() {
            let mut fields = Vec::new();
            for (bi, v) in sets {
                let e = self.expr_or_loc(v);
                fields.push_out(cat!(box_names[bi], ": ", e));
            }
            self.patches
                .push_out(cat!(loc, ".$set({ ", join(&fields, ", "), " });"));
        }
    }

    /// A function's source as an expression: methods and accessors through `$dump.fn`.
    fn fn_expr(&mut self, n: usize, src: &str) -> String {
        match self.fn_kind(n) {
            func_flags::KIND_METHOD | func_flags::KIND_GETTER | func_flags::KIND_SETTER => {
                self.helpers |= H_FN;
                self.ns = true;
                cat!("$dump.fn({ ", src, " })")
            }
            _ => src.into(),
        }
    }

    /// Writes node `n` (state New) as an expression; `loc` reaches it afterwards.
    fn render(&mut self, n: usize, loc: Option<&str>, ctx_name: Option<&str>) -> String {
        self.nodes[n].state = State::Busy;
        let loc = String::from(loc.unwrap_or_default());
        self.nodes[n].loc = loc.clone();
        let h = self.h;
        let v = self.nodes[n].v;
        let e = if let Some(s) = v.as_symbol() {
            match (
                self.registered(Value::symbol(s)),
                h.symbol_description(s).as_string(),
            ) {
                (Some(k), _) => cat!("Symbol.for(", k, ")"),
                (None, Some(d)) => cat!("Symbol(", quote_units(h.str_units(d)), ")"),
                (None, None) => "Symbol()".into(),
            }
        } else {
            let o = v.as_object().expect("an object");
            self.render_object(n, o, &loc, ctx_name)
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
        let e = match class {
            ObjectClass::Ordinary => return self.literal(o, loc),
            ObjectClass::Array => return self.array(o, loc),
            ObjectClass::Function => {
                if self.nodes[n].group != usize::MAX {
                    // Its captured variables exist only in its group's factory.
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "internal: a closure written outside its factory");
                }
                let src = self.fn_source(n).unwrap_or_default();
                let e = self.fn_expr(n, &src);
                self.fn_patches(n, ctx_name);
                return e;
            }
            ObjectClass::Map | ObjectClass::Set => self.collection(o, loc),
            ObjectClass::Date => cat!(
                "new Date(",
                number(h.object_slot(o, 0).as_number().unwrap_or(f64::NAN)),
                ")"
            ),
            ObjectClass::Boolean | ObjectClass::Number | ObjectClass::String => {
                cat!("Object(", self.expr(h.object_slot(o, 0), None, None), ")")
            }
            ObjectClass::RegExp => {
                let src = h.object_slot(o, crate::builtins::regexp::SLOT_SOURCE);
                let flags = crate::builtins::regexp::flags(h, o);
                let mut f = String::new();
                for &(c, fl) in crate::builtins::regexp::FLAG_CHARS.iter() {
                    if flags.contains(fl) {
                        f.push_out(c as char);
                    }
                }
                let s = src
                    .as_string()
                    .map(|s| h.str_to_string(s))
                    .unwrap_or_default();
                if !s.is_empty() && !s.contains(['/', '\n', '\r']) {
                    cat!("/", s, "/", f)
                } else {
                    cat!("new RegExp(", quote(&s), ", ", quote(&f), ")")
                }
            }
            ObjectClass::Error => {
                let proto = h.get_prototype_of(o).map(|p| p.offset());
                let ctor = proto
                    .and_then(|p| self.builtin(p))
                    .and_then(|b| b.path.strip_suffix(".prototype"))
                    .filter(|c| is_ident(c))
                    .map_or_else(|| String::from("Error"), String::from);
                let msg = h
                    .own_keys(o)
                    .into_iter()
                    .find(|k| *k == atoms::MESSAGE)
                    .and_then(|k| h.get_own_property(o, k))
                    .and_then(|p| p.value());
                let args = match msg {
                    Some(m) => self.expr(m, None, None),
                    None => String::new(),
                };
                if let Some(stack) = self.vm.error_stack(Value::object(o)) {
                    self.patches.push_out(cat!(
                        "Object.defineProperty(",
                        loc,
                        ", \"stack\", { value: ",
                        quote(&stack),
                        ", writable: true, configurable: true });"
                    ));
                }
                if !proto.is_some_and(|p| p < self.base_end) {
                    let p = self.expr(
                        h.get_prototype_of(o).map_or(Value::NULL, Value::object),
                        None,
                        None,
                    );
                    self.patches
                        .push_out(cat!("Object.setPrototypeOf(", loc, ", ", p, ");"));
                }
                cat!("new ", ctor, "(", args, ")")
            }
            ObjectClass::Promise => {
                let state = h.object_slot(o, 0).as_int().unwrap_or(0);
                let r = self.expr(h.object_slot(o, 1), None, None);
                if state == crate::interp::run::promise::FULFILLED {
                    cat!("Promise.resolve(", r, ")")
                } else {
                    self.helpers |= H_REJECTED;
                    self.ns = true;
                    cat!("$dump.rejected(", r, ")")
                }
            }
            ObjectClass::BoundFunction => {
                let target = self.expr(h.object_slot(o, 0), None, None);
                let mut args = alloc::vec![self.expr(h.object_slot(o, 1), None, None)];
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
                cat!(target, ".bind(", join(&args, ", "), ")")
            }
            #[cfg(feature = "full")]
            ObjectClass::ArrayBuffer => {
                self.helpers |= H_BYTES;
                self.ns = true;
                cat!("$dump.bytes(\"", base64(h.buffer_data(o)), "\")")
            }
            #[cfg(feature = "full")]
            ObjectClass::TypedArray | ObjectClass::DataView => {
                if let Some(e) = self.view(o, loc) {
                    return e;
                }
                return String::new();
            }
            _ => String::new(),
        };
        self.extra(o, loc);
        e
    }

    /// Properties, prototype and extensibility of an object written as a constructor
    /// call, as patches.
    fn extra(&mut self, o: Obj, loc: &str) {
        let h = self.h;
        let class = h.object_class(o);
        let skip: &dyn Fn(Atom, Option<u32>) -> bool = match class {
            ObjectClass::RegExp => &|k, _| k == atoms::LAST_INDEX,
            ObjectClass::Error => &|k, _| k == atoms::MESSAGE || k == atoms::STACK,
            ObjectClass::String => &|k, i| k == atoms::LENGTH || i.is_some(),
            ObjectClass::BoundFunction => &|k, _| k == atoms::NAME || k == atoms::LENGTH,
            ObjectClass::TypedArray => &|_, i| i.is_some(),
            _ => &|_, _| false,
        };
        self.other_props(o, loc, skip);
        if class == ObjectClass::RegExp
            && let Some(li) = h.own_keys(o).into_iter().find(|k| *k == atoms::LAST_INDEX)
            && let Some(Property::Data { value, .. }) = h.get_own_property(o, li)
            && value.as_number() != Some(0.0)
        {
            let e = self.expr(value, None, None);
            self.patches.push_out(cat!(loc, ".lastIndex = ", e, ";"));
        }
        if !matches!(
            class,
            ObjectClass::Error | ObjectClass::TypedArray | ObjectClass::DataView
        ) && !self.default_proto(o, h.get_prototype_of(o))
        {
            let p = self.expr_or_loc(h.get_prototype_of(o).map_or(Value::NULL, Value::object));
            self.patches
                .push_out(cat!("Object.setPrototypeOf(", loc, ", ", p, ");"));
        }
        if !h.is_extensible(o) {
            self.finals.push_out(self.freeze_line(o, loc));
        }
    }

    fn array(&mut self, o: Obj, loc: &str) -> String {
        let h = self.h;
        let len = h.array_length(o);
        let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
        let mut items = Vec::new();
        for i in 0..len {
            let v = match el.get() {
                Some(el) if i < h.value_array_len(el) => h.value_array_get(el, i),
                _ => Value::HOLE,
            };
            let at = if cell_of(v).is_some() {
                cat!(loc, "[", i, "]")
            } else {
                String::new()
            };
            let e = self.expr(v, Some(&at), None);
            if e.is_empty() && !v.is_hole() {
                let l = self.expr_or_loc(v);
                self.patches.push_out(cat!(at, " = ", l, ";"));
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
        self.other_props(o, loc, &|k, i| {
            k == atoms::LENGTH || i.is_some_and(|i| i < dense)
        });
        if !h.is_extensible(o) {
            self.finals.push_out(self.freeze_line(o, loc));
        }
        cat!("[", join(&items, ", "), "]")
    }

    fn collection(&mut self, o: Obj, loc: &str) -> String {
        let h = self.h;
        let is_map = h.object_class(o) == ObjectClass::Map;
        let entries = h.collection_entries_list(o);
        let mut items = Vec::new();
        let mut later = false;
        for (k, v) in &entries {
            let ke = self.expr(*k, None, None);
            if is_map {
                let ve = self.expr(*v, None, None);
                later |= ke.is_empty() || ve.is_empty();
                items.push_out(cat!("[", ke, ", ", ve, "]"));
            } else {
                later |= ke.is_empty();
                items.push_out(ke);
            }
        }
        let ctor = if is_map { "new Map(" } else { "new Set(" };
        if later {
            // An entry closes a cycle: all entries in order, after the data.
            for (k, v) in &entries {
                let ke = self.expr_or_loc(*k);
                if is_map {
                    let ve = self.expr_or_loc(*v);
                    self.patches
                        .push_out(cat!(loc, ".set(", ke, ", ", ve, ");"));
                } else {
                    self.patches.push_out(cat!(loc, ".add(", ke, ");"));
                }
            }
            cat!(ctor, ")")
        } else if items.is_empty() {
            cat!(ctor, ")")
        } else {
            cat!(ctor, "[", join(&items, ", "), "])")
        }
    }

    /// A typed array or DataView: a small typed array that is its buffer's only view as
    /// its elements, else a view of its buffer.
    #[cfg(feature = "full")]
    fn view(&mut self, o: Obj, loc: &str) -> Option<String> {
        let h = self.h;
        let class = h.object_class(o);
        let n = self.node_of(Value::object(o))?;
        let b = h.object_slot(o, 0);
        let (name, size, off, len) = if class == ObjectClass::TypedArray {
            let kind = h.ta_kind(o);
            (
                kind.name(),
                kind.size(),
                h.ta_byte_offset(o),
                h.object_slot(o, 3).as_number().unwrap_or(-1.0) as i64,
            )
        } else {
            (
                "DataView",
                1,
                h.object_slot(o, 1).as_number().unwrap_or(0.0) as u32,
                h.object_slot(o, 2).as_number().unwrap_or(-1.0) as i64,
            )
        };
        let whole = off == 0
            && len >= 0
            && b.as_object()
                .is_some_and(|b| h.buffer_len(b) as i64 == len * size as i64);
        let bn = self.node_of(b)?;
        let e = if whole
            && class == ObjectClass::TypedArray
            && !h.ta_kind(o).is_bigint()
            && len <= 64
            && self.nodes[bn].refs == 1
            && self.nodes[bn].state == State::New
            && self.simple(bn, None)
            && self.simple(n, None)
        {
            self.nodes[bn].state = State::Done;
            self.nodes[bn].loc = cat!(loc, ".buffer");
            let kind = h.ta_kind(o);
            let data = h.buffer_data(b.as_object()?);
            let mut s = cat!("new ", name, "([");
            for (i, c) in data.chunks(kind.size() as usize).enumerate() {
                cat_to!(
                    &mut s,
                    if i > 0 { ", " } else { "" },
                    number(crate::heap::typed::decode(kind, c))
                );
            }
            s.push_str("])");
            return Some(s);
        } else {
            let buf = self.expr(b, None, None);
            if whole {
                cat!("new ", name, "(", buf, ")")
            } else if len < 0 {
                cat!("new ", name, "(", buf, ", ", off, ")")
            } else {
                cat!("new ", name, "(", buf, ", ", off, ", ", len, ")")
            }
        };
        self.extra(o, loc);
        Some(e)
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

    /// What makes `o`'s attributes from writable ones: 2 `Object.freeze`, 1
    /// `Object.seal`, 0 `Object.preventExtensions`.
    fn freeze_level(&self, o: Obj) -> u8 {
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
            2
        } else if all(&|f, _| !f.configurable()) {
            1
        } else {
            0
        }
    }

    fn freeze_line(&self, o: Obj, loc: &str) -> String {
        let f = match self.freeze_level(o) {
            2 => "Object.freeze(",
            1 => "Object.seal(",
            _ => "Object.preventExtensions(",
        };
        cat!(f, loc, ");")
    }

    /// Properties of a non-literal object (a Map's, a regexp's) as patches.
    fn other_props(&mut self, o: Obj, loc: &str, skip: &dyn Fn(Atom, Option<u32>) -> bool) {
        let h = self.h;
        for k in h.own_keys(o) {
            if h.atom_symbol(k).is_none() && skip(k, h.atom_array_index(k)) {
                continue;
            }
            if let Some(p) = h.get_own_property(o, k) {
                self.prop_patch(loc, k, &p);
            }
        }
    }

    /// An ordinary object as a literal; attributes, accessors that do not fit and links
    /// that close cycles are patched (a placeholder keeps the key order).
    fn literal(&mut self, o: Obj, loc: &str) -> String {
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
                            .push_out(cat!("Object.setPrototypeOf(", loc, ", ", l, ");"));
                    } else {
                        items.push_out(cat!("__proto__: ", e));
                    }
                }
            }
        }
        let freeze = if h.is_extensible(o) {
            0
        } else {
            self.freeze_level(o)
        };
        for k in h.own_keys(o) {
            let key = self.literal_key(k);
            let prop = h.get_own_property(o, k);
            match prop {
                Some(Property::Data { value, flags }) => {
                    // The name an anonymous function gets from the key; only functions
                    // need it.
                    let name = if cell_of(value).is_some() {
                        h.atom_name(k)
                    } else {
                        String::new()
                    };
                    let at = if cell_of(value).is_some() {
                        cat!(loc, self.key_member(k))
                    } else {
                        String::new()
                    };
                    // A method written in place: `m() {...}`.
                    if flags == PropFlags::ALL
                        && let Some(c) = self.node_of(value)
                        && self.inline_method(c, &name, func_flags::KIND_METHOD)
                    {
                        let src = self.fn_source(c).unwrap_or_default();
                        self.nodes[c].loc = at;
                        self.nodes[c].state = State::Done;
                        self.fn_patches(c, Some(&name));
                        items.push_out(src);
                        continue;
                    }
                    let e = self.expr(value, Some(&at), Some(&name));
                    if e.is_empty() {
                        let l = self.expr_or_loc(value);
                        self.patches.push_out(cat!(at, " = ", l, ";"));
                        items.push_out(cat!(key, ": undefined"));
                    } else if e == key {
                        items.push_out(e);
                    } else {
                        items.push_out(cat!(key, ": ", e));
                    }
                    // `Object.freeze`/`seal` at the end set these attributes.
                    let implied = match freeze {
                        2 => flags == PropFlags::ENUMERABLE,
                        1 => flags == PropFlags::new(true, true, false),
                        _ => false,
                    };
                    if flags != PropFlags::ALL && !implied {
                        let ke = self.key_expr(k);
                        self.patches.push_out(cat!(
                            "Object.defineProperty(",
                            loc,
                            ", ",
                            ke,
                            ", { writable: ",
                            tf(flags.writable()),
                            ", enumerable: ",
                            tf(flags.enumerable()),
                            ", configurable: ",
                            tf(flags.configurable()),
                            " });"
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
                        let ke = self.key_expr(k);
                        for c in [g, s].into_iter().flatten() {
                            let src = self.fn_source(c).unwrap_or_default();
                            let which = if Some(c) == g { ").get" } else { ").set" };
                            self.nodes[c].loc =
                                cat!("Object.getOwnPropertyDescriptor(", loc, ", ", ke, which);
                            self.nodes[c].state = State::Done;
                            items.push_out(src);
                        }
                    } else {
                        items.push_out(cat!(key, ": undefined"));
                        let d = self.descriptor(&Property::Accessor {
                            getter,
                            setter,
                            flags,
                        });
                        let ke = self.key_expr(k);
                        self.patches.push_out(cat!(
                            "Object.defineProperty(",
                            loc,
                            ", ",
                            ke,
                            ", ",
                            d,
                            ");"
                        ));
                    }
                }
                None => {}
            }
        }
        if !h.is_extensible(o) {
            self.finals.push_out(self.freeze_line(o, loc));
        }
        if items.is_empty() {
            "{}".into()
        } else {
            cat!("{ ", join(&items, ", "), " }")
        }
    }

    /// Whether closure `c` can be written as a method, getter or setter in its holder's
    /// literal: free, written once, of that kind, and its text starts with the key.
    fn inline_method(&self, c: usize, key: &str, kind: u32) -> bool {
        self.free_closure(c)
            && self.nodes[c].refs == 1
            && self.nodes[c].state == State::New
            && self.nodes[c].root.is_none()
            && self.method_text(c, key, kind)
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
            _ => Some(match src.strip_prefix("async") {
                Some(s) if s.starts_with(char::is_whitespace) => s,
                _ => src.as_str(),
            }),
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

/// The free names of a function template: the variables it captures (its own and its
/// nested functions' from outside it) and the globals it or a nested function reads or
/// writes by name.
struct FreeNames {
    captures: Vec<String>,
    globals: Vec<String>,
}

fn free_names(h: &Heap, t: u32) -> FreeNames {
    let a = h.arena();
    let names = template_names(h, t);
    let locals = usize::from(a.get::<u16>(t, FuncTemplate::LOCALS));
    let captures = usize::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
    let captures = names
        .iter()
        .skip(locals)
        .take(captures)
        .filter(|n| is_ident(n))
        .cloned()
        .collect();
    let mut globals = Vec::new();
    let mut stack = alloc::vec![t];
    while let Some(t) = stack.pop() {
        for insn in crate::bytecode::insns(template::code(h, t)).flatten() {
            if matches!(
                insn.info().name,
                "get_global" | "get_global_typeof" | "put_global"
            ) {
                let g = h.atom_name(insn.atom(0));
                if !globals.contains(&g) {
                    globals.push_out(g);
                }
            }
        }
        for i in 0..template::child_count(h, t) {
            if let Some(c) = template::child(h, t, i).and_then(|r| r.get()) {
                stack.push_out(c);
            }
        }
    }
    FreeNames { captures, globals }
}

/// The current value of closure `f`'s captured variable `name`.
fn captured_value(h: &Heap, f: Obj, name: &str) -> Option<Value> {
    let t = h.closure_template(f);
    let a = h.arena();
    let locals = usize::from(a.get::<u16>(t, FuncTemplate::LOCALS));
    let captures = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
    let names = template_names(h, t);
    (0..captures)
        .find(|&i| names.get(locals + i as usize).is_some_and(|n| n == name))
        .and_then(|i| h.object_slot(f, i).as_internal())
        .map(|b| a.get(b, BoxCell::VALUE))
}

#[cfg(test)]
pub(super) fn template_names_pub(h: &Heap, t: u32) -> Vec<String> {
    template_names(h, t)
}

/// The names of a template's locals and then its captures (its debug info).
fn template_names(h: &Heap, t: u32) -> Vec<String> {
    let a = h.arena();
    match template::debug_info(h, t).names.get() {
        Some(n) => {
            let len: u32 = a.get(n, U32Array::LEN);
            (0..len)
                .map(|i| h.atom_name(Atom::from_id(a.read::<u32>(U32Array::DATA.at(n, i)))))
                .collect()
        }
        None => Vec::new(),
    }
}

/// FNV-1a of a function's text (an [`Index`] key; 0 is the empty key).
fn hash_text(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h | 1
}

/// The checker against texts the dumper must never write (0168).
#[cfg(test)]
mod check_tests {
    extern crate std;
    use super::*;
    use crate::vm::{Status, VmOptions};

    fn base() -> &'static [u8] {
        static BASE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        BASE.get_or_init(|| {
            let mut v = Vm::new(VmOptions::default()).unwrap();
            v.prepare_base_image().unwrap();
            v.bytes().to_vec()
        })
    }

    /// The refusals of checking `text` as the dump of `names` after `setup`.
    fn check(setup: &str, names: &[&str], drop_roots: &[&str], text: &str) -> Vec<String> {
        let mut vm = Vm::from_base(base(), VmOptions::default()).unwrap();
        let f = vm
            .compile(
                setup.as_bytes(),
                &crate::compiler::CompileOptions::script("t.js"),
            )
            .unwrap();
        let g = Value::object(vm.global());
        vm.call(f, g, &[]).unwrap();
        assert!(matches!(vm.run(u64::MAX), Status::Done(_)));
        let opts = DumpOptions {
            base: base(),
            host: &["console"],
        };
        let mut d = Dumper::new(&vm, &opts).unwrap();
        for n in names {
            let g = d.globals.iter().position(|g| g.name == *n).unwrap();
            d.add_global_root(g);
        }
        d.analyze();
        for n in &mut d.nodes {
            n.state = State::Done;
        }
        d.roots.retain(|r| !drop_roots.contains(&r.name.as_str()));
        d.check(text);
        d.refusals
    }

    const SETUP: &str = "let foo = [1]; function getStuff() { return foo; }\n\
                         function blah(a) { return () => a } const blah2 = blah(2);";

    #[test]
    fn a_closure_outside_its_factory_is_refused() {
        // What 0168's dumper wrote.
        let r = check(SETUP, &["blah2"], &[], "const blah2 = () => a;\n");
        assert_eq!(
            r,
            ["blah2: captures `a`, which is not in scope where the dump writes the function"]
        );
        let ok = "globalThis.$dump = {};\n$dump.s1 = (() => { let a = 2; return { blah2: () => a }; })();\n\
                  const blah2 = $dump.s1.blah2;\n";
        assert_eq!(check(SETUP, &["blah2"], &[], ok), Vec::<String>::new());
    }

    #[test]
    fn a_global_hidden_or_missing_is_refused() {
        let hidden = "let foo = [1];\nconst getStuff = (() => { let foo = 0; return function getStuff() { return foo; }; })();\n";
        let r = check(SETUP, &["getStuff"], &[], hidden);
        assert_eq!(
            r,
            ["getStuff: reads the global `foo`, which a variable of the dump hides"]
        );
        let missing = "function getStuff() { return foo; }\n";
        let r = check(SETUP, &["getStuff"], &["foo"], missing);
        assert_eq!(
            r,
            ["getStuff: reads the global `foo`, which the dump does not hold"]
        );
    }

    #[test]
    fn a_dump_that_does_not_compile_is_refused() {
        let r = check(
            SETUP,
            &["getStuff"],
            &[],
            "function getStuff() { return foo;\n",
        );
        assert!(r[0].starts_with("the dump: does not compile"), "{r:?}");
    }
}
