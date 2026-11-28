//! The inspection as JSON (D3760), for tools and the docs site.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::view::{Js, Node, VALUE_NOTES, esc};
use super::*;

/// A JSON writer: values pushed in order, commas between them.
struct Json {
    out: String,
    /// Whether the open container has an item yet.
    first: Vec<bool>,
}

impl Json {
    fn sep(&mut self) {
        if let Some(f) = self.first.last_mut() {
            if !*f {
                self.out.push(',');
            }
            *f = false;
        }
    }
    fn open(&mut self, c: char) {
        self.sep();
        self.out.push(c);
        self.first.push(true);
    }
    fn close(&mut self, c: char) {
        self.first.pop();
        self.out.push(c);
    }
    fn key(&mut self, k: &str) {
        self.sep();
        self.raw_str(k);
        self.out.push(':');
        // The value that follows needs no comma.
        if let Some(f) = self.first.last_mut() {
            *f = true;
        }
    }
    fn raw_str(&mut self, s: &str) {
        self.out.push('"');
        for c in s.chars() {
            esc(&mut self.out, c);
        }
        self.out.push('"');
    }
    fn str(&mut self, s: &str) {
        self.sep();
        self.raw_str(s);
    }
    fn num(&mut self, n: impl core::fmt::Display) {
        self.sep();
        self.out += &format!("{n}");
    }
    fn bool(&mut self, b: bool) {
        self.sep();
        self.out.push_str(if b { "true" } else { "false" });
    }
    fn null(&mut self) {
        self.sep();
        self.out.push_str("null");
    }
    fn fin(&mut self) {
        // After a key's value: the container's next item needs a comma.
        if let Some(f) = self.first.last_mut() {
            *f = false;
        }
    }
}

impl Inspection<'_> {
    /// The snapshot as JSON, for tools (D3760): its sections with byte ranges, record
    /// types, strings, builtins, host values, scripts with their sources, functions with
    /// their names, spans and hashes, every record with its fields by name, its byte range
    /// and what holds it, the run as a tree from its open calls, and what a loading build
    /// must agree on. `format` is the snapshot format's version: the JSON changes with it.
    pub fn json(&self) -> String {
        let mut j = Json {
            out: String::new(),
            first: Vec::new(),
        };
        j.open('{');
        j.key("format");
        j.num(crate::vm::snapshot::FORMAT);
        j.fin();
        j.key("bytes");
        j.num(self.size);
        j.fin();
        self.kv_str(&mut j, "summary", &self.summary());
        self.kv_str(&mut j, "sizeText", &self.size_text());
        j.key("sizes");
        j.open('[');
        for (part, n, what) in self.sizes() {
            j.open('{');
            self.kv_str(&mut j, "part", part);
            self.kv_num(&mut j, "bytes", n);
            self.kv_str(&mut j, "note", &what);
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("notes");
        j.open('{');
        for (k, n) in VALUE_NOTES {
            self.kv_str(&mut j, k, n);
        }
        j.close('}');
        j.fin();
        j.key("view");
        j.open('{');
        j.key("calls");
        self.jnodes(&mut j, &self.view);
        j.fin();
        j.key("others");
        self.jnodes(&mut j, &self.others_view);
        j.fin();
        j.close('}');
        j.fin();
        j.key("sections");
        j.open('[');
        for w in self.snap.marks.windows(2) {
            j.open('{');
            self.kv_str(&mut j, "name", w[0].0);
            self.kv_num(&mut j, "start", w[0].1);
            self.kv_num(&mut j, "end", w[1].1);
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("types");
        j.open('[');
        for t in &self.snap.types {
            j.open('{');
            self.kv_str(&mut j, "name", &t.name);
            j.key("fields");
            j.open('[');
            for (n, ty) in &t.fields {
                j.open('[');
                j.str(n);
                j.str(&ty_name(ty));
                j.close(']');
            }
            j.close(']');
            j.fin();
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("strings");
        j.open('[');
        for k in 0..self.snap.strings.len() {
            j.str(&self.text_of(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("builtins");
        j.open('[');
        for k in 0..self.snap.bases.len() {
            j.str(&self.builtin(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("intrinsics");
        j.open('[');
        for k in 0..self.snap.intrinsics.len() {
            j.str(&self.intrinsic(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("host");
        j.open('[');
        for (k, e) in self.snap.host.iter().enumerate() {
            j.open('{');
            self.kv_str(&mut j, "name", &self.host_name(k as u32));
            self.kv_str(
                &mut j,
                "kind",
                match e {
                    HostEntry::Root(_) => "value",
                    HostEntry::Member(..) => "member",
                    HostEntry::Function(_) => "function",
                },
            );
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("scripts");
        j.open('[');
        for &(name, params, bits, src) in &self.snap.scripts {
            j.open('{');
            self.kv_str(&mut j, "name", &self.text_of(name));
            j.key("params");
            j.open('[');
            let ps = self.text_of(params);
            for p in ps.split(',').filter(|p| !p.is_empty()) {
                j.str(p);
            }
            j.close(']');
            j.fin();
            self.kv_str(
                &mut j,
                "goal",
                match bits & 3 {
                    1 => "body",
                    2 => "eval",
                    3 => "module",
                    _ => "script",
                },
            );
            j.key("typescript");
            j.bool(bits & 1 << 2 != 0);
            j.fin();
            j.key("debuggerAwaits");
            j.bool(bits & 1 << 4 != 0);
            j.fin();
            match src {
                Ok(s) => {
                    j.key("source");
                    j.str(&String::from_utf8_lossy(s));
                    j.fin();
                }
                Err(h) => self.kv_str(&mut j, "sourceHash", &format!("{h:016x}")),
            }
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("functions");
        j.open('[');
        for (f, &(script, start, end, pinned, hash, ref sites)) in
            self.snap.functions.iter().enumerate()
        {
            let d = &self.functions[f];
            j.open('{');
            self.kv_num(&mut j, "script", script);
            self.kv_num(&mut j, "start", start);
            self.kv_num(&mut j, "end", end);
            self.kv_str(&mut j, "name", &d.name);
            self.kv_str(&mut j, "label", &self.fn_name(f));
            j.key("hash");
            j.str(&format!("{hash:016x}"));
            j.fin();
            self.kv_str(
                &mut j,
                "hashKind",
                if pinned { "bytecode" } else { "interface" },
            );
            if d.known {
                j.key("line");
                j.num(d.at.0);
                j.fin();
                j.key("col");
                j.num(d.at.1);
                j.fin();
                for (k, names) in [
                    ("params", &d.params),
                    ("locals", &d.locals),
                    ("captures", &d.captures),
                ] {
                    j.key(k);
                    j.open('[');
                    for n in names {
                        j.str(n);
                    }
                    j.close(']');
                    j.fin();
                }
            }
            j.key("templateObjects");
            j.open('[');
            for (c, v) in sites {
                j.open('{');
                self.kv_num(&mut j, "constant", *c);
                j.key("value");
                self.jvalue(&mut j, *v);
                j.fin();
                j.close('}');
            }
            j.close(']');
            j.fin();
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("records");
        j.open('[');
        for (k, (t, fs)) in self.snap.records.iter().enumerate() {
            self.jrecord(
                &mut j,
                *t,
                fs,
                self.spans[k],
                Some((k as u32, &self.held_by[k])),
            );
        }
        j.close(']');
        j.fin();
        j.key("vm");
        self.jrecord(
            &mut j,
            self.snap.root.0,
            &self.snap.root.1,
            self.root_span,
            None,
        );
        j.fin();
        j.key("run");
        j.open('{');
        self.kv_str(&mut j, "state", self.run_state());
        j.key("result");
        self.jvalue(
            &mut j,
            match self.root_field("result") {
                Some(F::V(v)) => *v,
                _ => V::Imm(Value::UNDEFINED),
            },
        );
        j.fin();
        j.key("calls");
        j.open('[');
        for c in &self.calls {
            j.open('{');
            self.kv_num(&mut j, "id", c.id);
            self.kv_num(&mut j, "record", c.record);
            self.kv_str(&mut j, "function", &c.function);
            j.key("args");
            j.open('[');
            for &a in &c.args {
                self.jvalue(&mut j, a);
            }
            j.close(']');
            j.fin();
            self.kv_opt(&mut j, "promise", c.promise);
            self.jlinks(&mut j, &c.waiters);
            j.close('}');
        }
        j.close(']');
        j.fin();
        j.key("others");
        j.open('[');
        for l in &self.others {
            self.jlink(&mut j, l);
        }
        j.close(']');
        j.fin();
        j.close('}');
        j.fin();
        j.key("needs");
        j.open('{');
        for (key, pinned) in [("bytecode", true), ("interface", false)] {
            j.key(key);
            j.open('[');
            for (f, e) in self.snap.functions.iter().enumerate() {
                if e.3 == pinned {
                    j.num(f);
                }
            }
            j.close(']');
            j.fin();
        }
        j.key("builtins");
        j.open('[');
        for k in 0..self.snap.bases.len() {
            j.str(&self.builtin(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("intrinsics");
        j.open('[');
        for k in 0..self.snap.intrinsics.len() {
            j.str(&self.intrinsic(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("host");
        j.open('[');
        for k in 0..self.snap.host.len() {
            j.str(&self.host_name(k as u32));
        }
        j.close(']');
        j.fin();
        j.key("types");
        j.open('[');
        for t in &self.snap.types {
            j.str(&t.name);
        }
        j.close(']');
        j.fin();
        j.close('}');
        j.fin();
        j.close('}');
        j.out
    }

    fn kv_str(&self, j: &mut Json, k: &str, v: &str) {
        j.key(k);
        j.str(v);
        j.fin();
    }

    fn kv_num(&self, j: &mut Json, k: &str, v: impl core::fmt::Display) {
        j.key(k);
        j.num(v);
        j.fin();
    }

    fn kv_opt(&self, j: &mut Json, k: &str, v: Option<u32>) {
        j.key(k);
        match v {
            Some(v) => j.num(v),
            None => j.null(),
        }
        j.fin();
    }

    fn jlinks(&self, j: &mut Json, links: &[Link]) {
        j.key("waiters");
        j.open('[');
        for l in links {
            self.jlink(j, l);
        }
        j.close(']');
        j.fin();
    }

    fn jlink(&self, j: &mut Json, l: &Link) {
        j.open('{');
        match l {
            Link::Activation(f) => {
                self.kv_str(j, "kind", "activation");
                self.kv_num(j, "record", f.record);
                j.key("function");
                match f.function {
                    Some(k) => j.num(k),
                    None => j.null(),
                }
                j.fin();
                let name = match (f.function, f.callee) {
                    (Some(k), _) => self.fn_name(k),
                    (None, V::Base(b, _)) => self.builtin(b),
                    _ => String::new(),
                };
                self.kv_str(j, "label", &name);
                j.key("callee");
                self.jvalue(j, f.callee);
                j.fin();
                self.kv_num(j, "pc", f.pc);
                if let Some((l, c)) = f.pos {
                    self.kv_num(j, "line", l);
                    self.kv_num(j, "col", c);
                }
                j.key("this");
                self.jvalue(j, f.this);
                j.fin();
                j.key("args");
                j.open('[');
                for (n, v) in &f.args {
                    j.open('{');
                    j.key("name");
                    match n {
                        Some(n) => j.str(n),
                        None => j.null(),
                    }
                    j.fin();
                    j.key("value");
                    self.jvalue(j, *v);
                    j.fin();
                    j.close('}');
                }
                j.close(']');
                j.fin();
                for (k, list) in [("locals", &f.locals), ("captures", &f.captures)] {
                    j.key(k);
                    j.open('[');
                    for (n, v) in list {
                        j.open('{');
                        self.kv_str(j, "name", n);
                        j.key("value");
                        self.jvalue(j, *v);
                        j.fin();
                        j.close('}');
                    }
                    j.close(']');
                    j.fin();
                }
                for (k, list) in [("stack", &f.stack), ("slots", &f.slots)] {
                    j.key(k);
                    j.open('[');
                    for &v in list {
                        self.jvalue(j, v);
                    }
                    j.close(']');
                    j.fin();
                }
                self.kv_opt(j, "promise", f.promise);
                self.jlinks(j, &f.waiters);
            }
            Link::Element {
                record,
                combinator,
                index,
                values,
                remaining,
                promise,
                waiters,
            } => {
                self.kv_str(j, "kind", "element");
                self.kv_num(j, "record", *record);
                self.kv_str(j, "combinator", combinator);
                j.key("index");
                match index {
                    Some(i) => j.num(i),
                    None => j.null(),
                }
                j.fin();
                j.key("values");
                match values {
                    Some(v) => self.jvalue(j, *v),
                    None => j.null(),
                }
                j.fin();
                j.key("remaining");
                match remaining {
                    Some(i) => j.num(i),
                    None => j.null(),
                }
                j.fin();
                self.kv_opt(j, "promise", *promise);
                self.jlinks(j, waiters);
            }
            Link::Reaction {
                handler,
                promise,
                waiters,
            } => {
                self.kv_str(j, "kind", "reaction");
                j.key("handler");
                self.jvalue(j, *handler);
                j.fin();
                self.kv_opt(j, "promise", *promise);
                self.jlinks(j, waiters);
            }
            Link::End {
                promise,
                result,
                held,
            } => {
                self.kv_str(j, "kind", "end");
                self.kv_num(j, "promise", *promise);
                self.kv_str(j, "state", &self.constant(*promise, "state"));
                j.key("result");
                j.bool(*result);
                j.fin();
                j.key("heldAs");
                match held {
                    Some(h) => j.str(h),
                    None => j.null(),
                }
                j.fin();
            }
            Link::Seen { promise } => {
                self.kv_str(j, "kind", "seen");
                self.kv_num(j, "promise", *promise);
            }
        }
        j.close('}');
    }

    fn jnodes(&self, j: &mut Json, nodes: &[Node]) {
        j.open('[');
        for n in nodes {
            j.open('{');
            self.kv_str(j, "kind", n.kind);
            self.kv_str(j, "title", &n.title);
            self.kv_str(j, "note", n.note);
            if let Some((script, line, col, text)) = &n.source {
                j.key("source");
                j.open('{');
                self.kv_num(j, "script", script);
                self.kv_num(j, "line", line);
                self.kv_num(j, "col", col);
                self.kv_str(j, "text", text);
                j.close('}');
                j.fin();
            }
            for (k, list) in [("bindings", &n.bindings), ("captured", &n.captured)] {
                j.key(k);
                j.open('[');
                for b in list {
                    j.open('{');
                    self.kv_str(j, "name", &b.name);
                    j.key("value");
                    self.jvalue(j, b.value);
                    j.fin();
                    self.kv_str(j, "js", &Js::new(self).js(b.value, DEPTH, ""));
                    self.kv_opt(j, "box", b.boxed);
                    j.key("param");
                    j.bool(b.param);
                    j.fin();
                    j.close('}');
                }
                j.close(']');
                j.fin();
            }
            j.key("uninitialized");
            j.open('[');
            for u in &n.uninit {
                j.str(u);
            }
            j.close(']');
            j.fin();
            j.key("doing");
            match &n.doing {
                Some(d) => j.str(d),
                None => j.null(),
            }
            j.fin();
            self.kv_opt(j, "record", n.record);
            j.key("detail");
            j.open('[');
            for d in &n.detail {
                j.str(d);
            }
            j.close(']');
            j.fin();
            j.key("children");
            self.jnodes(j, &n.children);
            j.fin();
            j.close('}');
        }
        j.close(']');
    }

    /// Record `r` as JavaScript: its shape and its parts as values (record references
    /// kept, so that a page can show one object reached twice as one).
    fn jview(&self, j: &mut Json, r: u32) {
        const MAX: usize = 200;
        let ty = self.type_name(r).to_string();
        j.open('{');
        let vals = |s: &Self, j: &mut Json, k: &str, vs: &[V]| {
            j.key(k);
            j.open('[');
            for &v in vs.iter().take(MAX) {
                s.jvalue(j, v);
            }
            j.close(']');
            j.fin();
            if vs.len() > MAX {
                s.kv_num(j, "more", vs.len() - MAX);
            }
        };
        let props = |s: &Self, j: &mut Json| {
            j.key("props");
            j.open('[');
            let js = Js::new(s);
            if let Some(F::Props(ps)) = s.field(r, "props") {
                for p in ps.iter().take(MAX) {
                    j.open('{');
                    s.kv_str(j, "key", &js.key(p.key));
                    if p.bits & PROP_ACCESSOR != 0 {
                        j.key("get");
                        s.jvalue(j, p.a);
                        j.fin();
                        j.key("set");
                        s.jvalue(j, p.b);
                        j.fin();
                    } else {
                        j.key("value");
                        s.jvalue(j, p.a);
                        j.fin();
                    }
                    j.close('}');
                }
            }
            j.close(']');
            j.fin();
        };
        match ty.as_str() {
            BOX => {
                self.kv_str(j, "shape", "box");
                j.key("value");
                self.jvalue(
                    j,
                    self.value(r, "VALUE").unwrap_or(V::Imm(Value::UNDEFINED)),
                );
                j.fin();
            }
            PROMISE => {
                self.kv_str(j, "shape", "promise");
                self.kv_str(j, "state", &self.constant(r, "state"));
                j.key("value");
                self.jvalue(
                    j,
                    self.value(r, "result").unwrap_or(V::Imm(Value::UNDEFINED)),
                );
                j.fin();
            }
            FUNCTION => {
                self.kv_str(j, "shape", "function");
                let f = match self.field(r, "template") {
                    Some(F::Fn(f)) if (*f as usize) < self.functions.len() => Some(*f as usize),
                    _ => None,
                };
                self.kv_str(j, "name", &f.map_or_else(String::new, |f| self.fn_name(f)));
                j.key("function");
                match f {
                    Some(f) => j.num(f),
                    None => j.null(),
                }
                j.fin();
                j.key("captures");
                j.open('[');
                let names = f
                    .map(|f| self.functions[f].captures.clone())
                    .unwrap_or_default();
                if let Some(F::Vs(cs)) = self.field(r, "captures") {
                    for (k, &c) in cs.iter().enumerate() {
                        j.open('{');
                        self.kv_str(j, "name", names.get(k).map_or("", |s| s.as_str()));
                        j.key("value");
                        self.jvalue(j, c);
                        j.fin();
                        j.close('}');
                    }
                }
                j.close(']');
                j.fin();
            }
            GENERATOR => {
                self.kv_str(j, "shape", "activation");
                self.kv_str(j, "state", &self.constant(r, "state"));
                self.kv_str(j, "kind", &self.constant(r, "kind"));
            }
            "Object.Array" => {
                self.kv_str(j, "shape", "array");
                let el = match self.field(r, "elements") {
                    Some(F::Vs(v)) => v.clone(),
                    _ => Vec::new(),
                };
                vals(self, j, "items", &el);
                props(self, j);
            }
            "Object.Map" | "Object.Set" => {
                let map = ty == "Object.Map";
                self.kv_str(j, "shape", if map { "map" } else { "set" });
                let entries = self
                    .value(r, "entries")
                    .map(|e| self.list(e))
                    .unwrap_or_default();
                let live: Vec<&[V]> = entries
                    .get(2..)
                    .unwrap_or(&[])
                    .chunks(2)
                    .filter(|kv| !matches!(kv[0], V::Imm(x) if x.is_hole()))
                    .collect();
                self.kv_num(j, "size", live.len());
                j.key("entries");
                j.open('[');
                for kv in live.iter().take(MAX) {
                    if map {
                        j.open('[');
                        self.jvalue(j, kv[0]);
                        self.jvalue(j, kv.get(1).copied().unwrap_or(V::Imm(Value::UNDEFINED)));
                        j.close(']');
                    } else {
                        self.jvalue(j, kv[0]);
                    }
                }
                j.close(']');
                j.fin();
            }
            "ValueArray" => {
                self.kv_str(j, "shape", "list");
                vals(self, j, "items", &self.list(V::Rec(r, true)));
            }
            t if t.starts_with("Object.") || t.starts_with("Patch.") => {
                self.kv_str(j, "shape", "object");
                let class = t
                    .strip_prefix("Object.")
                    .map(|c| c.strip_prefix("Record.").unwrap_or(c));
                self.kv_str(
                    j,
                    "class",
                    match class {
                        Some("Ordinary") => "",
                        Some(c) => c,
                        None => "",
                    },
                );
                if let Some(F::Vs(el)) = self.field(r, "elements") {
                    vals(self, j, "elements", el);
                }
                props(self, j);
                self.jslots(j, r, true);
            }
            _ => {
                self.kv_str(j, "shape", "record");
                self.jslots(j, r, false);
            }
        }
        j.close('}');
    }

    /// A record's fields as `slots` [{name, value}]: an object's internal slots, or all
    /// fields of another record (values only; the rest as text).
    fn jslots(&self, j: &mut Json, r: u32, object: bool) {
        let Some((t, fs)) = self.snap.records.get(r as usize) else {
            return;
        };
        let ty = &self.snap.types[*t as usize];
        j.key("slots");
        j.open('[');
        for ((n, _), f) in ty.fields.iter().zip(fs) {
            if object
                && matches!(
                    n.as_str(),
                    "proto" | "header" | "props" | "elements" | "deleted"
                )
            {
                continue;
            }
            let v = match f {
                F::V(v) => *v,
                F::U(u) => V::Imm(Value::number(*u as f64)),
                F::I(x) => V::Imm(Value::number(*x as f64)),
                F::Const(_) => {
                    j.open('{');
                    self.kv_str(j, "name", n);
                    self.kv_str(j, "text", &self.constant(r, n));
                    j.close('}');
                    continue;
                }
                _ => continue,
            };
            j.open('{');
            self.kv_str(j, "name", n);
            j.key("value");
            self.jvalue(j, v);
            j.fin();
            j.close('}');
        }
        j.close(']');
        j.fin();
    }

    fn jrecord(
        &self,
        j: &mut Json,
        t: u32,
        fs: &[F],
        span: (usize, usize),
        held: Option<(u32, &Vec<u32>)>,
    ) {
        let ty = &self.snap.types[t as usize];
        j.open('{');
        self.kv_str(j, "type", &ty.name);
        self.kv_num(j, "start", span.0);
        self.kv_num(j, "end", span.1);
        if let Some((r, _)) = held {
            self.kv_str(j, "label", &self.label(r));
            self.kv_str(j, "note", &self.note(r));
            self.kv_str(j, "js", &Js::new(self).js(V::Rec(r, false), 2, ""));
            j.key("view");
            self.jview(j, r);
            j.fin();
        }
        j.key("fields");
        j.open('{');
        for ((n, fty), f) in ty.fields.iter().zip(fs) {
            j.key(n);
            self.jfield(j, fty, f);
            j.fin();
        }
        j.close('}');
        j.fin();
        if let Some((_, h)) = held {
            j.key("heldBy");
            j.open('[');
            for &r in h {
                if r == u32::MAX {
                    j.str("vm");
                } else {
                    j.num(r);
                }
            }
            j.close(']');
            j.fin();
        }
        j.close('}');
    }

    fn jfield(&self, j: &mut Json, ty: &Ty, f: &F) {
        match f {
            F::V(v) => self.jvalue(j, *v),
            F::U(u) | F::Buckets(u) => j.num(u),
            F::I(i) => j.num(i),
            F::F64(x) => jnum(j, *x),
            F::Vs(vs) => {
                j.open('[');
                for &v in vs {
                    self.jvalue(j, v);
                }
                j.close(']');
            }
            F::Props(ps) => {
                j.open('[');
                for p in ps {
                    j.open('{');
                    j.key("key");
                    self.jvalue(j, p.key);
                    j.fin();
                    j.key("flags");
                    j.open('[');
                    for (bit, n) in [(1, "writable"), (2, "enumerable"), (4, "configurable")] {
                        if p.bits & bit != 0 {
                            j.str(n);
                        }
                    }
                    j.close(']');
                    j.fin();
                    if p.bits & PROP_ACCESSOR != 0 {
                        j.key("get");
                        self.jvalue(j, p.a);
                        j.fin();
                        j.key("set");
                        self.jvalue(j, p.b);
                        j.fin();
                    } else {
                        j.key("value");
                        self.jvalue(j, p.a);
                        j.fin();
                    }
                    j.close('}');
                }
                j.close(']');
            }
            F::Tail(rows) => {
                let elem = match ty {
                    Ty::Tail(e) => e.as_slice(),
                    _ => &[],
                };
                j.open('[');
                for row in rows {
                    if elem.len() == 1 && elem[0].0.is_empty() {
                        self.jfield(j, &elem[0].1, &row[0]);
                        continue;
                    }
                    j.open('{');
                    for ((n, t), f) in elem.iter().zip(row) {
                        j.key(n);
                        self.jfield(j, t, f);
                        j.fin();
                    }
                    j.close('}');
                }
                j.close(']');
            }
            F::Fn(k) => {
                j.open('{');
                self.kv_str(j, "t", "function");
                self.kv_num(j, "v", k);
                j.close('}');
            }
            F::Intr(k) => j.str(&self.intrinsic(*k)),
            F::HostFn(k) => j.str(&self.host_name(*k)),
            F::Flags(bits) | F::Const(bits) => {
                let names = match ty {
                    Ty::Flags(n) | Ty::Bits(_, n) => n.as_slice(),
                    Ty::Enum(_, n) => {
                        j.str(n.get(*bits as usize).map_or("?", |s| s.as_str()));
                        return;
                    }
                    _ => &[],
                };
                j.open('[');
                for (k, n) in names.iter().enumerate() {
                    if bits & (1 << k) != 0 {
                        j.str(n);
                    }
                }
                j.close(']');
            }
            F::None => j.null(),
        }
    }

    /// A value: `{"t": kind, "v": ...}`.
    fn jvalue(&self, j: &mut Json, v: V) {
        j.open('{');
        match v {
            V::Imm(x) => {
                let t = if x.is_undefined() {
                    "undefined"
                } else if x.is_null() {
                    "null"
                } else if x.is_hole() {
                    "hole"
                } else if x.is_uninitialized() {
                    "uninitialized"
                } else if x.as_bool().is_some() {
                    "bool"
                } else if x.as_int().is_some() {
                    "int"
                } else {
                    "number"
                };
                self.kv_str(j, "t", t);
                if let Some(b) = x.as_bool() {
                    j.key("v");
                    j.bool(b);
                    j.fin();
                } else if let Some(n) = x.as_number() {
                    j.key("v");
                    jnum(j, n);
                    j.fin();
                }
            }
            V::Str(s) => {
                self.kv_str(j, "t", "string");
                self.kv_str(j, "v", &self.text_of(s));
            }
            V::Rec(r, internal) => {
                self.kv_str(j, "t", "record");
                self.kv_num(j, "v", r);
                if internal {
                    j.key("internal");
                    j.bool(true);
                    j.fin();
                }
            }
            V::Base(b, internal) => {
                self.kv_str(j, "t", "builtin");
                self.kv_str(j, "v", &self.builtin(b));
                if internal {
                    j.key("internal");
                    j.bool(true);
                    j.fin();
                }
            }
            V::Fn(f) => {
                self.kv_str(j, "t", "function");
                self.kv_num(j, "v", f);
            }
            V::Script(s) => {
                self.kv_str(j, "t", "script");
                self.kv_num(j, "v", s);
            }
            V::Host(h) => {
                self.kv_str(j, "t", "host");
                self.kv_str(j, "v", &self.host_name(h));
            }
        }
        j.close('}');
    }
}

/// A number in JSON: the shortest round-trip form, non-finite ones and -0 as strings.
fn jnum(j: &mut Json, x: f64) {
    if x.is_finite() && !(x == 0.0 && x.is_sign_negative()) {
        j.sep();
        j.out.push_str(crate::num::to_string(x).as_str());
    } else {
        j.str(crate::num::to_string(x).as_str().trim_start_matches('+'));
    }
}

/// A field type as the snapshot names it.
fn ty_name(t: &Ty) -> String {
    match t {
        Ty::Value => "value".into(),
        Ty::Ref => "ref".into(),
        Ty::Uint => "uint".into(),
        Ty::Int => "int".into(),
        Ty::F64 => "f64".into(),
        Ty::Values => "values".into(),
        Ty::Props => "props".into(),
        Ty::Function => "function".into(),
        Ty::Intrinsic => "intrinsic".into(),
        Ty::HostFn => "hostfn".into(),
        Ty::Buckets => "buckets".into(),
        Ty::Program => "program".into(),
        Ty::Flags(_) => "flags".into(),
        Ty::Enum(s, _) => format!("enum:{s}"),
        Ty::Bits(s, _) => format!("bits:{s}"),
        Ty::Tail(e) if e.len() == 1 && e[0].0.is_empty() => format!("tail({})", ty_name(&e[0].1)),
        Ty::Tail(e) => {
            let parts: Vec<String> = e
                .iter()
                .map(|(n, t)| format!("{n}:{}", ty_name(t)))
                .collect();
            format!("tail(record({}))", parts.join(","))
        }
    }
}
