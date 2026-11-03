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
//! - What is in flight (pending promises and their reactions, suspended activations,
//!   open host calls, iterators) is written as the calls that recreate it
//!   (dump_flight.rs, 0172). Closures that share variables of one name in several scopes
//!   (a loop's per-iteration bindings) get nested factories.
//! - Live things are refused with the path of the value: weak collections, host
//!   functions the host does not name, decorated classes, ...
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

/// The first line of every dump this build writes: format 2, a Script that
/// `Vm::load_dump` runs with its `$dump` (D2581). Version 1 dumps (D2353), plain Scripts
/// that make their own `$dump`, still load.
pub const HEADER: &str = "// djs-dump 2";

/// What [`Vm::dump`] needs besides the VM.
#[derive(Clone, Copy)]
pub struct DumpOptions<'a> {
    /// The base image the VM was made from: builtins are named by their paths on it.
    pub base: &'a [u8],
    /// Globals the host defines in every VM (`console`, `tools`): never dumped; values
    /// reached from them are referenced by name (`tools.read`).
    pub host: &'a [&'a str],
    /// The host's name of its function `fn_id` (`tools.read`), for open calls and for
    /// host functions held as values; the loader's `LoadOptions::host_fn` is the
    /// inverse. `None` refuses.
    pub host_fn_name: &'a dyn Fn(u32) -> Option<String>,
    /// Values the host holds (a run's promise, a REPL input's): dumped as
    /// `$dump.handle(name, v)`, so the loader gives them back.
    pub handles: &'a [(&'a str, Value)],
}

fn no_host_fn_name(_: u32) -> Option<String> {
    None
}

impl Default for DumpOptions<'_> {
    fn default() -> Self {
        DumpOptions {
            base: &[],
            host: &[],
            host_fn_name: &no_host_fn_name,
            handles: &[],
        }
    }
}

/// A dump and what it holds in flight ([`Vm::dump_report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dumped {
    pub text: String,
    /// The suspended activations it resumes, in the dump's order.
    pub activations: Vec<ResumedSite>,
    /// Open host calls and pending promises it recreates.
    pub open_calls: usize,
    pub pending_promises: usize,
}

/// Where a resumed activation stopped: the location the dump gives it (`$dump.a3`), its
/// function, the site (`await#1`), and the site's place in the function's script, with
/// the source line (`col` counts characters from 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumedSite {
    pub loc: String,
    pub function: String,
    pub site: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub source_line: String,
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
        self.dump_report(opts, names).map(|d| d.text)
    }

    /// [`Vm::dump`] with what the dump holds in flight, for hosts and readers: the
    /// activations it resumes (where each stopped) and the counts. The text itself has
    /// no comments about them (0180 moves comments out of the format).
    pub fn dump_report(
        &self,
        opts: &DumpOptions<'_>,
        names: Option<&[&str]>,
    ) -> Result<Dumped, DumpError> {
        let mut d = Dumper::new(self, opts)?;
        d.flight_roots(opts);
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
        d.flight_roots(opts);
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
        d.finish().map(|d| d.text)
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
    /// A function or object the evaluation of a class makes: its methods, accessors and
    /// prototype, and its hidden closures (field initializers, private methods).
    member: Option<Member>,
    /// A closure that uses a class's own scope (private names, hidden bindings): only
    /// the class's text can make it.
    class_scope: bool,
    /// Private fields and brands (D1400): key and property.
    privates: Vec<(Atom, Property)>,
    /// In flight (dump_flight.rs): the index of its pending promise, activation or
    /// iterator record there, else `NO_FLIGHT`.
    fl: usize,
    /// A box an activation's frame holds: its scope group's factory hands out its cell.
    frame_box: bool,
}

/// Where a class member sits: `C.m`, `C.prototype.m`, an accessor's function, the
/// prototype itself, or nowhere (a hidden closure of the class).
#[derive(Clone, Copy)]
struct Member {
    class: usize,
    part: Part,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// `C.prototype`.
    Prototype,
    /// A method of the class (`proto`: of its prototype) under `key`.
    Value {
        proto: bool,
        key: Atom,
    },
    Get {
        proto: bool,
        key: Atom,
    },
    Set {
        proto: bool,
        key: Atom,
    },
    /// A hidden closure (a field initializer, a private method).
    Internal,
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
const H_BYTES: u16 = 1;
const H_FN: u16 = 2;
const H_REJECTED: u16 = 4;
const H_BUILTIN: u16 = 8;
const H_KEEP: u16 = 16;
const H_ADVANCE: u16 = 32;
const H_SPENT: u16 = 64;
const H_FROM_SYNC: u16 = 128;

/// A node that is not in flight (`Node::fl`).
const NO_FLIGHT: usize = usize::MAX;

#[path = "dump_flight.rs"]
mod flight;

const HELPERS: &[(u16, &str)] = &[
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
    (
        H_ADVANCE,
        "$dump.advance = (it, n) => {\n  for (let i = 0; i < n; i++) it.next();\n  return it;\n};\n",
    ),
    (
        H_SPENT,
        "$dump.spent = () => {\n  let f;\n  new Promise((res, rej) => (f = [res, rej]));\n  f[0]();\n  return f;\n};\n",
    ),
    (
        H_FROM_SYNC,
        "$dump.closeIt = (it) => {\n  try {\n    const m = it.return;\n    if (m !== undefined && m !== null) m.call(it);\n  } catch {}\n};\n\
$dump.closeThrow = (it) => (e) => {\n  $dump.closeIt(it);\n  throw e;\n};\n\
$dump.fromSync = (it, next) => {\n  \
const step = (r, close) => {\n    \
if (Object(r) !== r) return Promise.reject(new TypeError(\"iterator must return an object\"));\n    \
let done, wrapper;\n    try {\n      done = !!r.done;\n      const value = r.value;\n      \
try {\n        wrapper = Promise.resolve(value);\n      } catch (e) {\n        if (!done && close) $dump.closeIt(it);\n        throw e;\n      }\n    \
} catch (e) {\n      return Promise.reject(e);\n    }\n    \
return wrapper.then((v) => ({ value: v, done }), !done && close ? $dump.closeThrow(it) : undefined);\n  };\n  \
const call = (f, args, close) => {\n    let r;\n    try {\n      r = f.apply(it, args);\n    } catch (e) {\n      return Promise.reject(e);\n    }\n    return step(r, close);\n  };\n  \
return {\n    next(...args) {\n      return call(next, args, true);\n    },\n    \
return(...args) {\n      let m;\n      try {\n        m = it.return;\n      } catch (e) {\n        return Promise.reject(e);\n      }\n      \
if (m === undefined || m === null) return Promise.resolve({ value: args[0], done: true });\n      return call(m, args, false);\n    },\n    \
throw(...args) {\n      let m;\n      try {\n        m = it.throw;\n      } catch (e) {\n        return Promise.reject(e);\n      }\n      \
if (m === undefined || m === null) {\n        $dump.closeIt(it);\n        return Promise.reject(new TypeError(\"throw is not a method\"));\n      }\n      \
return call(m, args, true);\n    },\n  };\n};\n",
    ),
    (
        H_KEEP,
        "$dump.keep = (o, keys) => {\n  for (const k of Reflect.ownKeys(o)) {\n    \
         if (![\"length\", \"name\", \"prototype\", \"constructor\"].includes(k) && !keys.includes(k)) delete o[k];\n  }\n  \
         for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError(\"the class does not make \" + String(k));\n};\n",
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
    /// A written function comes from a TypeScript script: the dump is compiled as
    /// TypeScript (`// djs-dump 2 typescript`).
    ts: core::cell::Cell<bool>,
    /// The base heap: the checker compiles the dump into it.
    scratch: Option<Heap>,
    /// The classes of the factory being written, by their names inside it.
    class_locals: Vec<(usize, String)>,
    /// The private names of the dumped classes: symbol cell, class node, name, and
    /// whether the name is ambiguous in its class.
    private_names: Vec<(u32, usize, String, bool)>,
    patches: Vec<String>,
    finals: Vec<String>,
    /// What is in flight: open calls, pending promises, activations (dump_flight.rs).
    fl: flight::Flight<'a>,
    /// Objects being written as data (`render`): promises made by activations or
    /// reactions are patched into them instead (dump_flight.rs `make`).
    data_depth: u32,
    helpers: u16,
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
            ts: core::cell::Cell::new(false),
            scratch: None,
            class_locals: Vec::new(),
            private_names: Vec::new(),
            patches: Vec::new(),
            finals: Vec::new(),
            fl: flight::Flight::new(opts.host_fn_name),
            data_depth: 0,
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
        self.nodes.push(Node {
            v,
            path: path(),
            refs: 1,
            edges: Vec::new(),
            cyclic: false,
            root: None,
            state: State::New,
            loc: String::new(),
            group: usize::MAX,
            member: None,
            class_scope: false,
            privates: Vec::new(),
            fl: NO_FLIGHT,
            frame_box: false,
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
            // (In its TDZ: its factory declares it after its `return`.)
            if !inner.is_uninitialized() {
                self.edge(n, inner, || path);
            }
            return;
        }
        let Some(o) = v.as_object() else {
            return; // A symbol.
        };
        // Private fields (their values) and brands; written with `$dump.private`.
        let privates = h.private_entries(o);
        for (k, p) in &privates {
            if let Property::Data { value, .. } = p
                && !self.is_brand(*k)
            {
                let path = cat!(path, ".", h.atom_name(*k));
                self.edge(n, *value, || path);
            }
        }
        self.nodes[n].privates = privates;
        match h.object_class(o) {
            ObjectClass::Ordinary => self.props(n, o, &|_, _| false),
            ObjectClass::Array => {
                let len = h.array_length(o);
                let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
                for i in 0..len {
                    let v = array_element(h, o, el, i);
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
                    self.visit_pending(n, o);
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
                let id = h.object_slot(o, 0).as_int().unwrap_or(-1) as u32;
                if (self.fl.host_fn_name)(id).is_none() {
                    self.refuse(path, "a host function the host does not name")
                }
            }
            ObjectClass::Generator => self.visit_activation(n, o),
            ObjectClass::BuiltinClosure => self.visit_builtin_closure(n, o),
            ObjectClass::Record => {
                // Written by dump_flight.rs (or refused there).
                self.nodes[n].fl = 0;
                for i in 0..h.object_slot_count(o) {
                    let v = h.object_slot(o, i);
                    self.edge(n, v, || cat!(path, "<", i, ">"));
                }
            }
            ObjectClass::ArrayIterator | ObjectClass::CollectionIterator => {
                self.visit_iterator(n, o)
            }
            ObjectClass::Arguments => {
                let len = self.arguments_len(o);
                match len {
                    Some(len) => {
                        for i in 0..len {
                            let v = Atom::int(i)
                                .and_then(|k| h.get_own_property(o, k))
                                .and_then(|p| p.value())
                                .unwrap_or(Value::UNDEFINED);
                            self.edge(n, v, || cat!(path, "[", i, "]"));
                        }
                    }
                    None => self.refuse(path, "an arguments object that was changed"),
                }
            }
            ObjectClass::Proxy => match (h.proxy_target(o), h.proxy_handler(o)) {
                (Some(t), Some(hd)) => {
                    self.edge(n, Value::object(t), || cat!(path, "<target>"));
                    self.edge(n, Value::object(hd), || cat!(path, "<handler>"));
                }
                _ => self.refuse(path, "a revoked Proxy"),
            },
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
        let Some(src) = crate::builtins::function::source_slice(h, o) else {
            return self.refuse(path, "a function without source text");
        };
        let class = matches!(
            kind,
            func_flags::KIND_CLASS_CTOR | func_flags::KIND_DERIVED_CTOR
        );
        if class {
            if src.contains('@') {
                return self.refuse(path, "a decorated class (its decorators would run again)");
            }
            if kind == func_flags::KIND_DERIVED_CTOR && heritage(&src).is_none() {
                return self.refuse(
                    path,
                    "a class whose heritage is an expression (only `extends Name` or a path is written)",
                );
            }
        }
        if kind == func_flags::KIND_FIELD_INIT {
            self.nodes[n].class_scope = true;
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
            if name.starts_with('#') || name.starts_with('<') {
                // The class's private names and hidden bindings: made by its text. The
                // hidden closures (field initializers, private methods) are members.
                self.nodes[n].class_scope = true;
                let inner: Value = a.get(b, BoxCell::VALUE);
                if name.starts_with('<')
                    && inner
                        .as_object()
                        .is_some_and(|f| h.object_class(f) == ObjectClass::Function)
                {
                    self.edge(n, inner, || p);
                }
                continue;
            }
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
            let skip = if class {
                k == atoms::LENGTH || k == atoms::NAME
            } else {
                h.atom_symbol(k).is_none() && self.default_fn_key(o, k)
            };
            if skip {
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
        let proto = h.get_prototype_of(o);
        if class {
            // A derived class's parent (`extends`), or a base class's changed prototype.
            if let Some(p) = proto
                && (kind == func_flags::KIND_DERIVED_CTOR || !self.default_proto(o, Some(p)))
            {
                self.edge(n, Value::object(p), || cat!(path, ".[[Prototype]]"));
            }
        } else if !self.default_proto(o, proto) {
            self.refuse(path, "a function with another prototype");
        }
    }

    /// The length of an arguments object as a call makes it (indexes `0..length` with
    /// default attributes, `length`, `callee` and `Symbol.iterator` untouched), else
    /// `None`.
    fn arguments_len(&self, o: Obj) -> Option<u32> {
        let h = self.h;
        let len = h
            .get_own_property(o, atoms::LENGTH)
            .and_then(|p| p.value())
            .and_then(|v| v.as_int())? as u32;
        let keys = h.own_keys(o);
        let fixed = |k: Atom| k == atoms::LENGTH || k == atoms::CALLEE || k == atoms::SYM_ITERATOR;
        let ok = keys.len() == len as usize + 3
            && keys.iter().all(|&k| {
                fixed(k)
                    || h.atom_array_index(k).is_some_and(|i| i < len)
                        && matches!(h.get_own_property(o, k), Some(Property::Data { flags, .. }) if flags == PropFlags::ALL)
            })
            && h.get_prototype_of(o).map(Value::object) == Some(intrinsic(h, Intrinsic::ObjectProto));
        ok.then_some(len)
    }

    /// A private key that holds a method or accessor (a brand), not a field.
    fn is_brand(&self, k: Atom) -> bool {
        self.h.atom_symbol(k).is_some_and(|s| {
            self.h.symbol_private_flags(s) & crate::heap::cells::flags::SYM_PRIVATE_METHOD != 0
        })
    }

    /// After discovery: the members of every class (what its evaluation makes), and the
    /// refusal of class-scope closures no class makes.
    fn assign_members(&mut self) {
        let h = self.h;
        for c in 0..self.nodes.len() {
            let Some(co) = self.nodes[c].v.as_object() else {
                continue;
            };
            if !super::dump_load::is_class(h, co) {
                continue;
            }
            let span = fn_span(h, co);
            let mut members: Vec<usize> = Vec::new();
            let mut set = |d: &mut Self, v: Value, part: Part| {
                if let Some(m) = d.node_of(v)
                    && d.nodes[m].member.is_none()
                {
                    d.nodes[m].member = Some(Member { class: c, part });
                    members.push(m);
                }
            };
            let proto = match h.get_own_property(co, atoms::PROTOTYPE) {
                Some(Property::Data { value, .. }) => value.as_object(),
                _ => None,
            };
            if let Some(p) = proto {
                set(self, Value::object(p), Part::Prototype);
            }
            for (holder, is_proto) in [(Some(co), false), (proto, true)] {
                let Some(holder) = holder else { continue };
                for k in h.own_keys(holder) {
                    let owned = |v: Value| {
                        v.as_object().is_some_and(|f| {
                            is_method_of(h, f, span) && !super::dump_load::is_class(h, f)
                        })
                    };
                    match h.get_own_property(holder, k) {
                        Some(Property::Data { value, .. }) if owned(value) => set(
                            self,
                            value,
                            Part::Value {
                                proto: is_proto,
                                key: k,
                            },
                        ),
                        Some(Property::Accessor { getter, setter, .. }) => {
                            if owned(getter) {
                                set(
                                    self,
                                    getter,
                                    Part::Get {
                                        proto: is_proto,
                                        key: k,
                                    },
                                );
                            }
                            if owned(setter) {
                                set(
                                    self,
                                    setter,
                                    Part::Set {
                                        proto: is_proto,
                                        key: k,
                                    },
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            // The hidden closures the class and its members hold.
            let mut queue = members.clone();
            queue.push(c);
            while let Some(m) = queue.pop() {
                let edges = self.nodes[m].edges.clone();
                for e in edges {
                    if self.nodes[e].member.is_none()
                        && self.nodes[e].class_scope
                        && self.nodes[e]
                            .v
                            .as_object()
                            .is_some_and(|f| is_method_of(h, f, span) || fn_span(h, f) == span)
                        && self.edge_is_hidden(m, e)
                    {
                        self.nodes[e].member = Some(Member {
                            class: c,
                            part: Part::Internal,
                        });
                        queue.push(e);
                    }
                }
            }
        }
        for n in 0..self.nodes.len() {
            if self.nodes[n].class_scope && self.nodes[n].member.is_none() && !self.is_class_node(n)
            {
                let p = self.nodes[n].path.clone();
                self.refuse(
                    p,
                    "a function that uses a class's private names or fields outside the class",
                );
            }
        }
    }

    /// The classes of the private keys the dumped objects hold.
    fn resolve_privates(&mut self) {
        let h = self.h;
        for c in 0..self.nodes.len() {
            let Some(co) = self.nodes[c].v.as_object() else {
                continue;
            };
            if super::dump_load::is_class(h, co) {
                for p in super::dump_load::class_privates(h, co) {
                    self.private_names
                        .push_out((p.sym.offset(), c, p.name, p.ambiguous));
                }
            }
        }
        for n in 0..self.nodes.len() {
            for i in 0..self.nodes[n].privates.len() {
                let k = self.nodes[n].privates[i].0;
                let Some(s) = h.atom_symbol(k) else { continue };
                let why = match self.private_names.iter().find(|p| p.0 == s.offset()) {
                    None => "a private field or method of a class the dump does not hold",
                    Some(p) if p.3 => "a private name the class has twice (a nested class's)",
                    Some(_) => continue,
                };
                let path = cat!(self.nodes[n].path, ".", h.atom_name(k));
                self.refuse(path, why);
            }
        }
    }

    /// Whether node `m` holds closure `e` in a hidden binding of a class (`<fields 0>`,
    /// `<#m>`).
    fn edge_is_hidden(&self, m: usize, e: usize) -> bool {
        let h = self.h;
        let (Some(f), Some(target)) = (self.nodes[m].v.as_object(), self.nodes[e].v.as_object())
        else {
            return false;
        };
        if h.object_class(f) != ObjectClass::Function {
            return false;
        }
        let t = h.closure_template(f);
        let a = h.arena();
        let locals = usize::from(a.get::<u16>(t, FuncTemplate::LOCALS));
        let names = self.template_names(t);
        (0..u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES))).any(|i| {
            names
                .get(locals + i as usize)
                .is_some_and(|n| n.starts_with('<'))
                && h.object_slot(f, i)
                    .as_internal()
                    .is_some_and(|b| a.get::<Value>(b, BoxCell::VALUE) == Value::object(target))
        })
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
        // Union-find over closures and the boxes they capture, and over a class and the
        // functions its evaluation makes (one text, one scope). A set without boxes is
        // free: no group.
        let is_box: Vec<bool> = self
            .nodes
            .iter()
            .map(|d| d.v.as_internal().is_some())
            .collect();
        let mut parent: Vec<usize> = (0..n).collect();
        for (i, d) in self.nodes.iter().enumerate() {
            if is_box[i] {
                continue;
            }
            // Closures share their boxes; an activation's frame holds its own, which
            // join a group only through the closures that capture them.
            let closure =
                d.v.as_object()
                    .is_some_and(|o| self.h.object_class(o) == ObjectClass::Function);
            for &e in &d.edges {
                if closure && is_box[e] {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, e));
                    parent[a] = b;
                }
            }
            if let Some(m) = d.member
                && m.part != Part::Prototype
            {
                let (a, b) = (find(&mut parent, i), find(&mut parent, m.class));
                parent[a] = b;
            }
        }
        let mut has_box = alloc::vec![false; n];
        for (i, &b) in is_box.iter().enumerate() {
            if b {
                let r = find(&mut parent, i);
                has_box[r] = true;
            }
        }
        let mut grouped = alloc::vec![false; n];
        for i in 0..n {
            let function = self.nodes[i]
                .v
                .as_object()
                .is_some_and(|o| self.h.object_class(o) == ObjectClass::Function);
            let r = find(&mut parent, i);
            grouped[i] = has_box[r] && (is_box[i] || function);
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

    fn finish(mut self) -> Result<Dumped, DumpError> {
        self.drain();
        self.assign_members();
        self.resolve_privates();
        if !self.refusals.is_empty() {
            return Err(DumpError(self.refusals));
        }
        self.analyze();
        self.flight_start();
        for r in 0..self.roots.len() {
            self.emit_root(r);
        }
        self.flight_finish();
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
        if self.ts.get() {
            s.push_str(" typescript");
        }
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
            ").\n// Load it with Vm::load_dump (djs REPL: .restore FILE), which passes in $dump.\n\
             // Builtins are referenced by name, never written",
        );
        if !self.host_names.is_empty() {
            s.push_str(if self.host_names.len() == 1 {
                ". The host global "
            } else {
                ". The host globals "
            });
            for (i, n) in self.host_names.iter().enumerate() {
                cat_to!(&mut s, if i > 0 { ", " } else { "" }, *n);
            }
            s.push_str(if self.host_names.len() == 1 {
                " is re-bound by name"
            } else {
                " are re-bound by name"
            });
        }
        s.push_str(".\n");
        if self.helpers != 0 {
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
        let (open_calls, _, pending_promises) = self.fl.counts();
        Ok(Dumped {
            text: s,
            activations: self.fl.take_sites(),
            open_calls,
            pending_promises,
        })
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
            let opts = super::dump_load::dump_options(self.ts.get());
            let r = crate::compiler::compile(&mut scratch, text.as_bytes(), &opts);
            (scratch, r)
        });
        let (scratch, t) = match compiled {
            Some((scratch, Ok(t))) => match t.get() {
                Some(t) => (scratch, t),
                None => return self.refuse("the dump".into(), "does not compile"),
            },
            Some((_, Err(e))) => {
                let why = cat!("does not compile: ", e.message, " at ", e.line, ":", e.col);
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
            // Functions of the same text (a class's constructor and field initializer
            // share the class's): one of them must keep every name.
            let mut found = false;
            let mut first: Option<Vec<String>> = None;
            while let Some(&(a, b, ref new, next)) = written.get(at as usize) {
                at = next;
                if text.get(a as usize..b as usize) != Some(src.as_str()) {
                    continue;
                }
                found = true;
                let mut why: Vec<String> = Vec::new();
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
                        why.push_out(cat!(
                            "captures `",
                            c,
                            "`, which is not in scope where the dump writes the function"
                        ));
                    }
                }
                for g in &orig.globals {
                    if new.captures.contains(g) {
                        why.push_out(cat!(
                            "reads the global `",
                            g,
                            "`, which a variable of the dump hides"
                        ));
                    } else if !self.roots.iter().any(|r| r.name == *g)
                        && !self.host_names.contains(&g.as_str())
                        && !self.base_globals.contains(g)
                        && self.globals.iter().any(|u| u.name == *g)
                    {
                        why.push_out(cat!(
                            "reads the global `",
                            g,
                            "`, which the dump does not hold"
                        ));
                    }
                }
                if why.is_empty() {
                    first = None;
                    break;
                }
                first.get_or_insert(why);
            }
            for w in first.unwrap_or_default() {
                self.refuse(path.clone(), &w);
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
                    && self.nodes[n].fl == NO_FLIGHT
                    && self.nodes[n].state == State::New
                    && self.nodes[n].group == usize::MAX
                    && self.nodes[n].member.is_none() =>
            {
                self.nodes[n].loc = name.clone();
                if self.is_class_node(n)
                    && self.roots[r].decl != Decl::Var
                    && let Some(src) = self.fn_source(n)
                    && class_head(&src) == Some((Some(name.as_str()), None))
                {
                    // As written: `class Name {...}`.
                    self.nodes[n].state = State::Busy;
                    self.out.push_out(src);
                    self.skeleton.push_out(None);
                    self.nodes[n].state = State::Done;
                    self.class_done(n, &name, Some(&name));
                    self.roots[r].state = State::Done;
                    return;
                }
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
        let f = self.nodes[n].v.as_object()?;
        // A function of a TypeScript script makes the dump TypeScript (D2615).
        let t = self.h.closure_template(f);
        if let Some(script) = template::debug_info(self.h, t).script.get()
            && self
                .h
                .arena()
                .get::<u32>(script, crate::heap::cells::Script::OPTIONS)
                & crate::compiler::CompileOptions::TYPESCRIPT_BIT
                != 0
        {
            self.ts.set(true);
        }
        crate::builtins::function::source_slice(self.h, f)
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
        if self.nodes[n]
            .member
            .is_some_and(|m| m.part == Part::Internal)
        {
            let p = self.nodes[n].path.clone();
            self.refuse(
                p,
                "an internal function of a class (a field initializer, a private method)",
            );
            return "undefined".into();
        }
        if self.nodes[n].fl != NO_FLIGHT {
            return self.flight_expr(n);
        }
        match self.nodes[n].state {
            State::Done => return self.nodes[n].loc.clone(),
            // Only nodes on a cycle are reached while busy; the caller patches.
            State::Busy => return String::new(),
            State::New => {}
        }
        if let Some(m) = self.nodes[n].member
            && self.nodes[n]
                .root
                .is_none_or(|r| self.roots[r].state != State::New)
        {
            // Made by its class's text.
            let cv = self.nodes[m.class].v;
            self.expr(cv, None, None);
            return if self.nodes[n].state == State::Done {
                self.nodes[n].loc.clone()
            } else {
                String::new()
            };
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
        if !h.is_extensible(o) || !self.nodes[n].privates.is_empty() || self.is_class_node(n) {
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
        // A class's members are made by its text: they get no key of their own.
        let (members, closures): (Vec<usize>, Vec<usize>) = self.groups[g]
            .closures
            .iter()
            .partition(|&&c| self.nodes[c].member.is_some());
        let boxes = self.groups[g].boxes.clone();
        let box_value = |d: &Self, b: usize| -> Value {
            h.arena()
                .get(d.nodes[b].v.as_internal().expect("a box"), BoxCell::VALUE)
        };
        // The boxes' variable names (their paths end in `~name`).
        let box_names: Vec<String> = boxes
            .iter()
            .map(|&b| String::from(self.nodes[b].path.rsplit('~').next().unwrap_or_default()))
            .collect();
        let dup = |name: &str| box_names.iter().filter(|n| *n == name).count() > 1;
        for (bi, n) in box_names.iter().enumerate() {
            if !is_ident(n) {
                // A frame's hidden binding (a class's `<fields 0>`) that closures share.
                let p = self.nodes[boxes[bi]].path.clone();
                self.refuse(p, "an activation's hidden binding that closures share");
            }
        }

        // Scopes. A variable's name must be unique where it is declared; variables of the
        // same name (a loop's per-iteration bindings captured by several closures) go into
        // nested factories, one per set of closures that capture them. Those sets must
        // nest (or be disjoint), as scopes do.
        let all: Vec<usize> = closures.iter().chain(&members).copied().collect();
        let caps: Vec<Vec<usize>> = boxes
            .iter()
            .map(|&b| {
                all.iter()
                    .copied()
                    .filter(|&c| self.nodes[c].edges.contains(&b))
                    .collect()
            })
            .collect();
        // The nested scopes: the closure sets of duplicated names. Scope 0 is the factory.
        let mut sets: Vec<Vec<usize>> = alloc::vec![Vec::new()];
        for (bi, s) in caps.iter().enumerate() {
            if dup(&box_names[bi]) && !sets[1..].contains(s) {
                sets.push_out(s.clone());
            }
        }
        let subset = |a: &[usize], b: &[usize]| a.iter().all(|x| b.contains(x));
        for i in 1..sets.len() {
            for j in 1..sets.len() {
                let (a, b) = (&sets[i], &sets[j]);
                if i != j && a.iter().any(|x| b.contains(x)) && !subset(a, b) && !subset(b, a) {
                    let p = self.nodes[boxes[0]].path.clone();
                    self.refuse(p, "closures that share variables in crossing ways");
                }
            }
        }
        // A scope's parent: the smallest other scope that contains it.
        let parent: Vec<usize> = (0..sets.len())
            .map(|i| {
                if i == 0 {
                    return usize::MAX;
                }
                (1..sets.len())
                    .filter(|&j| j != i && subset(&sets[i], &sets[j]))
                    .min_by_key(|&j| sets[j].len())
                    .unwrap_or(0)
            })
            .collect();
        let depth = |mut s: usize| {
            let mut d = 0;
            while s != 0 {
                s = parent[s];
                d += 1;
            }
            d
        };
        // The deepest scope whose closures include all of `of`.
        let deepest = |of: &[usize]| -> usize {
            (1..sets.len())
                .filter(|&k| !of.is_empty() && subset(of, &sets[k]))
                .max_by_key(|&k| depth(k))
                .unwrap_or(0)
        };
        let box_scope: Vec<usize> = (0..boxes.len())
            .map(|bi| {
                if dup(&box_names[bi]) {
                    sets.iter().position(|s| *s == caps[bi]).unwrap_or(0)
                } else {
                    deepest(&caps[bi])
                }
            })
            .collect();
        for s in 0..sets.len() {
            for (i, &si) in box_scope.iter().enumerate() {
                if si == s
                    && box_scope[..i]
                        .iter()
                        .zip(&box_names)
                        .any(|(&sj, n)| sj == s && *n == box_names[i])
                {
                    let p = self.nodes[boxes[i]].path.clone();
                    self.refuse(p, "two captured variables of the same name in one scope");
                }
            }
        }
        let fn_scope: Vec<usize> = closures.iter().map(|&c| deepest(&[c])).collect();
        for &m in &members {
            if deepest(&[m]) != 0 {
                let p = self.nodes[m].path.clone();
                self.refuse(p, "a class in a nested scope of shared variables");
            }
        }
        // `s` is `a` or inside it.
        let within = |mut s: usize, a: usize| loop {
            if s == a {
                return true;
            }
            if s == 0 {
                return false;
            }
            s = parent[s];
        };

        // Each closure's home: a box of the group that holds it under its name (where all
        // the variables it captures are in scope), else a key of the returned object.
        let mut home: Vec<Option<usize>> = alloc::vec![None; closures.len()];
        for (bi, &b) in boxes.iter().enumerate() {
            if let Some(c) = self.node_of(box_value(self, b))
                && let Some(j) = closures.iter().position(|&x| x == c)
                && home[j].is_none()
                && boxes
                    .iter()
                    .enumerate()
                    .filter(|(_, bb)| self.nodes[c].edges.contains(bb))
                    .all(|(k, _)| within(box_scope[bi], box_scope[k]))
            {
                home[j] = Some(bi);
            }
        }
        // Keys: unique in the group.
        let mut taken: Vec<String> = box_names.iter().filter(|n| !dup(n)).cloned().collect();
        let mut keys: Vec<String> = Vec::new();
        for (j, &c) in closures.iter().enumerate() {
            let key = match home[j] {
                Some(bi) if !dup(&box_names[bi]) => box_names[bi].clone(),
                _ => {
                    let fname = self.fn_name(c);
                    let rname = self.nodes[c].root.map(|r| self.roots[r].name.clone());
                    if home[j].is_none() && is_ident(&fname) && !taken.contains(&fname) {
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
        for &m in &members {
            self.nodes[m].state = State::Busy;
        }
        for &b in &boxes {
            self.nodes[b].state = State::Busy;
        }
        let mut text = cat!("// ", join(&keys, ", "), " sharing ");
        for (i, n) in box_names.iter().enumerate() {
            cat_to!(&mut text, if i > 0 { ", " } else { "" }, n);
        }
        cat_to!(&mut text, "\n", loc, " = (() => {\n");
        // Classes first, a parent before its subclasses: their texts only read the
        // group's variables when their methods run.
        let mut classes: Vec<usize> = Vec::new();
        let mut pending: Vec<usize> = (0..closures.len())
            .filter(|&j| self.is_class_node(closures[j]))
            .collect();
        while !pending.is_empty() {
            let ready = pending
                .iter()
                .position(|&j| {
                    let parent = self.class_parent(closures[j]);
                    !pending.iter().any(|&k| Some(closures[k]) == parent)
                })
                .unwrap_or(0);
            classes.push_out(pending.remove(ready));
        }
        let mut locals: Vec<String> = alloc::vec![String::new(); closures.len()];
        for &j in &classes {
            let c = closures[j];
            if fn_scope[j] != 0 {
                let p = self.nodes[c].path.clone();
                self.refuse(p, "a class in a nested scope of shared variables");
            }
            locals[j] = match home[j] {
                Some(bi) => box_names[bi].clone(),
                None => cat!("$c", c),
            };
            let src = self.fn_source(c).unwrap_or_default();
            let line = match class_head(&src) {
                Some((Some(name), None)) if name == locals[j] => src,
                _ => {
                    let e = self.class_text(c);
                    cat!("let ", locals[j], " = ", e, ";")
                }
            };
            self.class_locals.push_out((c, locals[j].clone()));
            // Only the first line is indented: the rest belongs to a function's text,
            // which must stay as it was.
            cat_to!(&mut text, "  ", line, '\n');
        }
        // The scopes, innermost first: each is the body of a factory of its own whose
        // returned object its parent spreads into its own.
        let mut order: Vec<usize> = (0..sets.len()).collect();
        crate::sort::sort_by(&mut order, |&a, &b| depth(a) > depth(b));
        let mut bodies: Vec<String> = alloc::vec![String::new(); sets.len()];
        let mut set_fns: Vec<(String, Vec<(usize, Value)>)> = Vec::new();
        for &s in &order {
            let mut body = String::new();
            let mut sets_here: Vec<(usize, Value)> = Vec::new();
            let mut tdz: Vec<String> = Vec::new();
            for (k, &child) in parent.iter().enumerate() {
                if child == s && k != 0 {
                    cat_to!(
                        &mut body,
                        "const $s",
                        k,
                        " = (() => {\n",
                        bodies[k],
                        "})();\n"
                    );
                }
            }
            for (bi, &b) in boxes.iter().enumerate() {
                if box_scope[bi] != s {
                    continue;
                }
                let name = &box_names[bi];
                let inner = box_value(self, b);
                if inner.is_uninitialized() {
                    // In its TDZ: declared after the factory's `return`, so never
                    // initialized.
                    tdz.push_out(cat!("let ", name, ";"));
                    continue;
                }
                let c = self.node_of(inner);
                let at_home = c.and_then(|c| closures.iter().position(|&x| x == c));
                let line = match (c, at_home) {
                    (Some(c), Some(j)) if home[j] == Some(bi) => {
                        if self.is_class_node(c) {
                            continue;
                        }
                        let src = self.fn_source(c).unwrap_or_default();
                        if declared_name(self.fn_kind(c), &src) == Some(name.as_str()) {
                            src
                        } else {
                            let e = self.fn_expr(c, &src);
                            cat!("let ", name, " = ", e, ";")
                        }
                    }
                    (Some(c), _) if self.nodes[c].state == State::Busy => {
                        sets_here.push_out((bi, inner));
                        cat!("let ", name, ";")
                    }
                    _ => {
                        let e = self.expr(inner, None, Some(name));
                        if e == *name {
                            if !self.nodes[b].frame_box {
                                // A captured `tools` that holds the host's `tools`: the
                                // closures read the global (`let tools = tools;` would be
                                // a TDZ error).
                                continue;
                            }
                            // A frame shares the variable: it must exist, with the value
                            // read before the factory.
                            let v = self.fresh("v");
                            self.out.push_out(cat!(v, " = ", e, ";"));
                            self.skeleton.push_out(Some(String::new()));
                            cat!("let ", name, " = ", v, ";")
                        } else {
                            cat!("let ", name, " = ", e, ";")
                        }
                    }
                };
                // Only the first line is indented: the rest belongs to a function's
                // text, which must stay as it was.
                cat_to!(&mut body, "  ", line, '\n');
            }
            let mut ret: Vec<String> = Vec::new();
            for (k, &child) in parent.iter().enumerate() {
                if child == s && k != 0 {
                    ret.push_out(cat!("...$s", k));
                }
            }
            for (j, &c) in closures.iter().enumerate() {
                if fn_scope[j] != s && home[j].is_none_or(|bi| box_scope[bi] != s) {
                    continue;
                }
                if home[j].is_some_and(|bi| box_scope[bi] != s) {
                    continue;
                }
                let key = &keys[j];
                if let Some(bi) = home[j] {
                    if *key == box_names[bi] {
                        ret.push_out(key.clone());
                    } else {
                        ret.push_out(cat!(key, ": ", box_names[bi]));
                    }
                } else if self.is_class_node(c) {
                    ret.push_out(cat!(key, ": ", locals[j]));
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
            // The cells of the variables an activation's frame shares with the closures.
            for (bi, &b) in boxes.iter().enumerate() {
                if box_scope[bi] == s && self.nodes[b].frame_box {
                    ret.push_out(cat!("$c", b, ": $dump.cell(() => ", box_names[bi], ")"));
                }
            }
            if !sets_here.is_empty() {
                let f = if s == 0 {
                    String::from("$set")
                } else {
                    cat!("$set", s)
                };
                let mut t = cat!(f, "(v) { ({ ");
                for (i, (bi, _)) in sets_here.iter().enumerate() {
                    cat_to!(&mut t, if i > 0 { ", " } else { "" }, box_names[*bi]);
                }
                t.push_str(" } = v); }");
                ret.push_out(t);
                set_fns.push_out((f, sets_here));
            }
            cat_to!(&mut body, "  return { ", join(&ret, ", "), " };\n");
            for l in &tdz {
                cat_to!(&mut body, "  ", l, '\n');
            }
            bodies[s] = body;
        }
        text.push_str(&bodies[0]);
        text.push_str("})();");
        self.out.push_out(text);
        self.skeleton.push_out(None);
        self.class_locals.clear();
        for &b in &boxes {
            self.nodes[b].state = State::Done;
            if self.nodes[b].frame_box {
                self.nodes[b].loc = cat!(loc, ".$c", b);
            }
        }
        for (j, &c) in closures.iter().enumerate() {
            self.nodes[c].state = State::Done;
            if self.is_class_node(c) {
                let l = self.nodes[c].loc.clone();
                self.class_done(c, &l, Some(&locals[j]));
            } else {
                self.fn_patches(c, Some(&keys[j]));
            }
        }
        self.groups[g].state = State::Done;
        for (f, sets) in set_fns {
            let mut fields = Vec::new();
            for (bi, v) in sets {
                let e = self.expr_or_loc(v);
                fields.push_out(cat!(box_names[bi], ": ", e));
            }
            self.patches
                .push_out(cat!(loc, ".", f, "({ ", join(&fields, ", "), " });"));
        }
    }

    fn is_class_node(&self, n: usize) -> bool {
        self.nodes[n]
            .v
            .as_object()
            .is_some_and(|o| super::dump_load::is_class(self.h, o))
    }

    /// The node of a class's parent (its `[[Prototype]]`), if it is one.
    fn class_parent(&self, n: usize) -> Option<usize> {
        let o = self.nodes[n].v.as_object()?;
        self.node_of(Value::object(self.h.get_prototype_of(o)?))
    }

    /// A class's text as an expression. A derived class's heritage path is bound to the
    /// parent it has now: `((B) => class A extends B {...})(parent)`, so the text
    /// stays verbatim and `extends` evaluates to the right class.
    fn class_text(&mut self, n: usize) -> String {
        let h = self.h;
        let src = self.fn_source(n).unwrap_or_default();
        let Some(path) = heritage(&src) else {
            return src;
        };
        let o = self.nodes[n].v.as_object().expect("a class");
        let parent = h.get_prototype_of(o).map_or(Value::NULL, Value::object);
        // `extends Map`: the builtin of that path needs no binding.
        if let Some(c) = cell_of(parent)
            && let Some(b) = self.builtin(c)
            && b.named == Named::Path
            && b.path == path
        {
            return src;
        }
        let pe = match self.node_of(parent).and_then(|p| {
            self.class_locals
                .iter()
                .find(|(c, _)| *c == p)
                .map(|(_, l)| l.clone())
        }) {
            Some(local) => local,
            None => self.expr_or_loc(parent),
        };
        let mut segs = path.split('.');
        let root = String::from(segs.next().unwrap_or_default());
        let mut arg = pe;
        let rest: Vec<&str> = segs.collect();
        for seg in rest.iter().rev() {
            arg = cat!("{ ", *seg, ": ", arg, " }");
        }
        // The parameter hides a variable of that name from the class's code.
        let mut uses = Vec::new();
        uses.push_out(n);
        for (m, d) in self.nodes.iter().enumerate() {
            if d.member.is_some_and(|mm| mm.class == n) {
                uses.push_out(m);
            }
        }
        let hidden = uses.iter().any(|&u| {
            self.nodes[u].edges.iter().any(|&e| {
                self.nodes[e].v.as_internal().is_some()
                    && self.nodes[e].path.rsplit('~').next() == Some(root.as_str())
            })
        });
        if hidden {
            let p = self.nodes[n].path.clone();
            self.refuse(
                p,
                "a class whose methods use a variable named like its heritage",
            );
        }
        cat!("((", root, ") => ", src, ")(", arg, ")")
    }

    /// After a class's text, at `loc`: its members' locations and what its evaluation
    /// does not make: its name if the text gives another, static fields and the
    /// properties added since (attributes kept), members deleted since, prototypes,
    /// private fields and brands, extensibility.
    fn class_done(&mut self, n: usize, loc: &str, ctx_name: Option<&str>) {
        let h = self.h;
        let o = self.nodes[n].v.as_object().expect("a class");
        let src = self.fn_source(n).unwrap_or_default();
        let head = class_head(&src).unwrap_or((None, None));
        self.nodes[n].loc = String::from(loc);
        let proto_obj = match h.get_own_property(o, atoms::PROTOTYPE) {
            Some(Property::Data { value, .. }) => value.as_object(),
            _ => None,
        };
        let proto_loc = cat!(loc, ".prototype");
        // Members.
        let mut kept: [Vec<Atom>; 2] = [Vec::new(), Vec::new()];
        for m in 0..self.nodes.len() {
            let Some(mm) = self.nodes[m].member.filter(|mm| mm.class == n) else {
                continue;
            };
            let holder = |proto: bool| {
                if proto {
                    proto_loc.clone()
                } else {
                    String::from(loc)
                }
            };
            let mloc = match mm.part {
                Part::Prototype => proto_loc.clone(),
                Part::Value { proto, key } => {
                    kept[usize::from(proto)].push_out(key);
                    cat!(holder(proto), self.key_member(key))
                }
                Part::Get { proto, key } | Part::Set { proto, key } => {
                    if !kept[usize::from(proto)].contains(&key) {
                        kept[usize::from(proto)].push_out(key);
                    }
                    let which = if matches!(mm.part, Part::Get { .. }) {
                        ").get"
                    } else {
                        ").set"
                    };
                    let ke = self.key_expr(key);
                    cat!(
                        "Object.getOwnPropertyDescriptor(",
                        holder(proto),
                        ", ",
                        ke,
                        which
                    )
                }
                Part::Internal => String::new(),
            };
            self.nodes[m].loc = mloc;
            self.nodes[m].state = State::Done;
        }
        // The name, if the text does not give it.
        let given = match head {
            (Some(name), _) => String::from(name),
            (None, Some(_)) => String::new(),
            (None, _) => String::from(ctx_name.unwrap_or("")),
        };
        if let Some(Property::Data { value, .. }) = h.get_own_property(o, atoms::NAME)
            && let Some(s) = value.as_string()
            && h.str_to_string(s) != given
        {
            let q = quote(&h.str_to_string(s));
            self.patches.push_out(cat!(
                "Object.defineProperty(",
                loc,
                ", \"name\", { value: ",
                q,
                ", configurable: true });"
            ));
        }
        // What the class's evaluation makes, and nothing else: members deleted since go.
        self.helpers |= H_KEEP;
        for (i, holder) in [String::from(loc), proto_loc.clone()].iter().enumerate() {
            let mut list = String::new();
            for (j, k) in kept[i].iter().enumerate() {
                let ke = self.key_expr(*k);
                cat_to!(&mut list, if j > 0 { ", " } else { "" }, ke);
            }
            self.patches
                .push_out(cat!("$dump.keep(", holder, ", [", list, "]);"));
        }
        // Static state and properties added since.
        for (holder_obj, holder) in [(Some(o), String::from(loc)), (proto_obj, proto_loc.clone())] {
            let Some(ho) = holder_obj else { continue };
            let is_proto = holder != loc;
            for k in h.own_keys(ho) {
                let default = if is_proto {
                    k == atoms::CONSTRUCTOR
                } else {
                    k == atoms::LENGTH || k == atoms::NAME || k == atoms::PROTOTYPE
                };
                if default || kept[usize::from(is_proto)].contains(&k) {
                    continue;
                }
                if let Some(p) = h.get_own_property(ho, k) {
                    let key = self.key_expr(k);
                    let d = self.descriptor(&p);
                    self.patches.push_out(cat!(
                        "Object.defineProperty(",
                        holder,
                        ", ",
                        key,
                        ", ",
                        d,
                        ");"
                    ));
                }
            }
        }
        // Prototypes: a base class's, and the prototype object's when it is not the
        // parent's `prototype` now.
        if head.1.is_none()
            && let Some(p) = h.get_prototype_of(o)
            && !self.default_proto(o, Some(p))
        {
            let e = self.expr_or_loc(Value::object(p));
            self.patches
                .push_out(cat!("Object.setPrototypeOf(", loc, ", ", e, ");"));
        }
        if let Some(po) = proto_obj {
            let expected = match h.get_prototype_of(o) {
                Some(parent) if head.1.is_some() => {
                    match h.get_own_property(parent, atoms::PROTOTYPE) {
                        Some(Property::Data { value, .. }) => value.as_object(),
                        _ => None,
                    }
                }
                _ => intrinsic(h, Intrinsic::ObjectProto).as_object(),
            };
            let now = h.get_prototype_of(po);
            if now != expected {
                let e = self.expr_or_loc(now.map_or(Value::NULL, Value::object));
                self.patches
                    .push_out(cat!("Object.setPrototypeOf(", proto_loc, ", ", e, ");"));
            }
            self.private_patches_of(po, &proto_loc);
            if !h.is_extensible(po) {
                self.finals.push_out(self.freeze_line(po, &proto_loc));
            }
        }
        self.private_patches_of(o, loc);
        if !h.is_extensible(o) {
            self.finals.push_out(self.freeze_line(o, loc));
        }
    }

    /// `$dump.private(...)` for the private fields and brands of object `o` at `loc`.
    fn private_patches_of(&mut self, o: Obj, loc: &str) {
        let Some(n) = self.node_of(Value::object(o)) else {
            return;
        };
        let privates = core::mem::take(&mut self.nodes[n].privates);
        for (k, p) in &privates {
            let Some(s) = self.h.atom_symbol(*k) else {
                continue;
            };
            let Some((_, c, name, _)) = self
                .private_names
                .iter()
                .find(|q| q.0 == s.offset())
                .cloned()
            else {
                continue;
            };
            if c == n && (self.is_brand(*k) || matches!(p, Property::Accessor { .. })) {
                // A class's static private methods: its evaluation installs them.
                continue;
            }
            let ce = self.expr_or_loc(self.nodes[c].v);
            let line = match p {
                Property::Data { value, .. } if !self.is_brand(*k) => {
                    let ve = self.expr_or_loc(*value);
                    cat!(
                        "$dump.private(",
                        loc,
                        ", ",
                        ce,
                        ", ",
                        quote(&name),
                        ", ",
                        ve,
                        ");"
                    )
                }
                _ => cat!("$dump.private(", loc, ", ", ce, ", ", quote(&name), ");"),
            };
            self.patches.push_out(line);
        }
        self.nodes[n].privates = privates;
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
            self.data_depth += 1;
            let e = self.render_object(n, o, &loc, ctx_name);
            self.data_depth -= 1;
            if !self.is_class_node(n) {
                self.private_patches_of(o, &loc);
            }
            e
        };
        self.nodes[n].state = State::Done;
        e
    }

    /// Whether closure `n` has a home object (a method that uses `super`).
    fn has_home(&self, n: usize) -> bool {
        self.nodes[n].v.as_object().is_some_and(|o| {
            self.h.object_class(o) == ObjectClass::Function
                && u32::from(
                    self.h
                        .arena()
                        .get::<u16>(self.h.closure_template(o), FuncTemplate::FLAGS),
                ) & func_flags::HOME
                    != 0
        })
    }

    /// A method's home object.
    fn home_of(&self, n: usize) -> Option<Value> {
        let o = self.nodes[n].v.as_object()?;
        let t = self.h.closure_template(o);
        let captures = u32::from(self.h.arena().get::<u16>(t, FuncTemplate::CAPTURES));
        self.has_home(n).then(|| self.h.object_slot(o, captures))
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
                if self.nodes[n].group != usize::MAX || self.nodes[n].member.is_some() {
                    // Its captured variables exist only in its group's factory; a member
                    // only in its class.
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "internal: a closure written outside its factory");
                }
                if self.is_class_node(n) {
                    let e = self.class_text(n);
                    self.class_done(n, loc, ctx_name);
                    return e;
                }
                if self.has_home(n) {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "a method that uses super outside its object literal");
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
            ObjectClass::Arguments => {
                let len = self.arguments_len(o).unwrap_or(0);
                let mut args = Vec::new();
                for i in 0..len {
                    let v = Atom::int(i)
                        .and_then(|k| h.get_own_property(o, k))
                        .and_then(|p| p.value())
                        .unwrap_or(Value::UNDEFINED);
                    args.push_out(self.expr(v, None, None));
                }
                if args.iter().any(String::is_empty) {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "an arguments object on a reference cycle");
                }
                return cat!(
                    "(function () { \"use strict\"; return arguments; })(",
                    join(&args, ", "),
                    ")"
                );
            }
            ObjectClass::Proxy => {
                let t = h.proxy_target(o).map_or(Value::NULL, Value::object);
                let hd = h.proxy_handler(o).map_or(Value::NULL, Value::object);
                let (t, hd) = (self.expr(t, None, None), self.expr(hd, None, None));
                if t.is_empty() || hd.is_empty() {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "a Proxy on a reference cycle");
                }
                return cat!("new Proxy(", t, ", ", hd, ")");
            }
            ObjectClass::HostFunction => {
                let id = h.object_slot(o, 0).as_int().unwrap_or(-1) as u32;
                let name = (self.fl.host_fn_name)(id).unwrap_or_default();
                let flags = h.cell_flags(o.offset());
                let asynch = flags & crate::heap::cells::flags::OBJ_HOST_ASYNC != 0;
                self.ns = true;
                return cat!("$dump.host(", quote(&name), ", ", tf(asynch), ")");
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
            let v = array_element(h, o, el, i);
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
        if len > 0 && array_element(h, o, el, len - 1).is_hole() {
            items.push_out(String::new());
        }
        let dense = el.get().map_or(0, |el| h.value_array_len(el));
        self.other_props(o, loc, &|k, i| {
            k == atoms::LENGTH || i.is_some_and(|i| i < dense)
        });
        // An instance of a subclass of Array.
        if !self.default_proto(o, h.get_prototype_of(o)) {
            let p = self.expr_or_loc(h.get_prototype_of(o).map_or(Value::NULL, Value::object));
            self.patches
                .push_out(cat!("Object.setPrototypeOf(", loc, ", ", p, ");"));
        }
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
                        && self.inline_method(c, &name, func_flags::KIND_METHOD, o)
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
                        .filter(|&c| self.inline_method(c, &name, func_flags::KIND_GETTER, o));
                    let s = self
                        .node_of(setter)
                        .filter(|&c| self.inline_method(c, &name, func_flags::KIND_SETTER, o));
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
    fn inline_method(&self, c: usize, key: &str, kind: u32, holder: Obj) -> bool {
        self.free_closure(c)
            && self.nodes[c].member.is_none()
            && self.home_of(c).is_none_or(|h| h == Value::object(holder))
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

/// Element `i` of array `o` whose dense elements are `el`: a frozen array keeps its
/// elements as properties, the dense slots holding holes.
fn array_element(h: &Heap, o: Obj, el: Ref, i: u32) -> Value {
    match el.get() {
        Some(el) if i < h.value_array_len(el) && !h.value_array_get(el, i).is_hole() => {
            h.value_array_get(el, i)
        }
        _ => Atom::int(i)
            .and_then(|k| h.get_own_property(o, k))
            .and_then(|p| p.value())
            .unwrap_or(Value::HOLE),
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

/// The names of a template's locals and then its captures (its debug info).
pub(super) fn template_names(h: &Heap, t: u32) -> Vec<String> {
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

/// A function's text position: its script and source span.
fn fn_span(h: &Heap, f: Obj) -> (u32, u32, u32) {
    let d = template::debug_info(h, h.closure_template(f));
    (d.script.get().unwrap_or(0), d.src_start, d.src_end)
}

/// Whether `f` is a method, getter or setter written inside the text at `span`.
fn is_method_of(h: &Heap, f: Obj, span: (u32, u32, u32)) -> bool {
    if h.object_class(f) != ObjectClass::Function {
        return false;
    }
    let kind = func_flags::kind(u32::from(
        h.arena()
            .get::<u16>(h.closure_template(f), FuncTemplate::FLAGS),
    ));
    let (script, a, b) = fn_span(h, f);
    matches!(
        kind,
        func_flags::KIND_METHOD | func_flags::KIND_GETTER | func_flags::KIND_SETTER
    ) && script == span.0
        && a >= span.1
        && b <= span.2
        && (a, b) != (span.1, span.2)
}

/// The words of a class's head: its name (if any) and its heritage path (`extends a.b`).
/// `None` for a heritage that is not a name or a path.
fn class_head(src: &str) -> Option<(Option<&str>, Option<&str>)> {
    fn ident(s: &str) -> (&str, &str) {
        let end = s
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
            .unwrap_or(s.len());
        (s.get(..end).unwrap_or(""), s.get(end..).unwrap_or(""))
    }
    let s = src.strip_prefix("class")?.trim_start();
    let (word, rest) = ident(s);
    let (name, rest) = match word {
        "" | "extends" => (None, s),
        w => (Some(w), rest.trim_start()),
    };
    let rest = rest.trim_start();
    if rest.starts_with('{') {
        return Some((name, None));
    }
    let rest = rest.strip_prefix("extends")?.trim_start();
    let mut end = 0;
    loop {
        let (w, after) = ident(rest.get(end..)?);
        if w.is_empty() || w.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        end += w.len();
        if after.starts_with('.') {
            end += 1;
        } else {
            break;
        }
    }
    let path = rest.get(..end)?;
    rest.get(end..)?
        .trim_start()
        .starts_with('{')
        .then_some((name, Some(path)))
}

/// A derived class's heritage path.
fn heritage(src: &str) -> Option<&str> {
    class_head(src)?.1
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
            ..Default::default()
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
    fn a_class_outside_its_scope_is_refused() {
        let setup = "function mk(t) { return class K { m() { return t; } }; } const K = mk(1);";
        let r = check(
            setup,
            &["K"],
            &[],
            "const K = class K { m() { return t; } };\n",
        );
        assert!(
            r.iter()
                .any(|l| l.contains("captures `t`, which is not in scope")),
            "{r:?}"
        );
        let ok = "const K = (() => { let t = 1; return class K { m() { return t; } }; })();\n";
        assert_eq!(check(setup, &["K"], &[], ok), Vec::<String>::new());
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
