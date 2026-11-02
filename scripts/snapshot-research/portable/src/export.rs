//! The exporter: a waiting VM as an engine-independent object graph (the format sketched
//! in docs/portable-snapshots.md, as JSON). Roots are the pending tool calls; code is
//! named by source span, suspension points by the await's source position, locals by
//! name, operands by role, builtins by path, builtin closures by intrinsic name and
//! their records by a named schema. Anything it cannot name refuses the export.

use std::collections::{HashMap, VecDeque};

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::heap::{Heap, ObjectClass, Property};
use djs_core::value::{Kind, Obj, Value};
use serde_json::{Value as J, json};

use crate::heapx::*;
use crate::roles::labels_at;
use crate::session::Session;

pub struct Exporter<'a> {
    heap: &'a Heap,
    builtins: HashMap<u32, String>,
    ids: HashMap<u32, usize>,
    nodes: Vec<J>,
    queue: VecDeque<(u32, usize)>,
    /// Record cells by the schema their referrer implies.
    record_schema: HashMap<u32, &'static str>,
    pub errors: Vec<String>,
}

/// Field names of the builtin records a codemode session holds. These are djs's own
/// layouts per build (builtins/promise.ts); the format names the fields.
pub fn record_fields(schema: &str) -> &'static [&'static str] {
    match schema {
        // Before Promise.allKeyed (D1972) the element record had no `keys`.
        "PromiseElement" if cfg!(feature = "old") => {
            &["alreadyCalled", "index", "values", "capability", "remaining"]
        }
        "PromiseElement" => &["alreadyCalled", "index", "values", "capability", "remaining", "keys"],
        "Counter" => &["count"],
        _ => &[],
    }
}

fn num(n: f64) -> J {
    if n.is_nan() {
        json!({"num": "NaN"})
    } else if n.is_infinite() {
        json!({"num": if n > 0.0 { "Infinity" } else { "-Infinity" }})
    } else if n == 0.0 && n.is_sign_negative() {
        json!({"num": "-0"})
    } else {
        json!(n)
    }
}

impl<'a> Exporter<'a> {
    pub fn new(heap: &'a Heap) -> Self {
        Exporter {
            heap,
            builtins: builtin_paths(heap),
            ids: HashMap::new(),
            nodes: Vec::new(),
            queue: VecDeque::new(),
            record_schema: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn node(&mut self, cell: u32) -> J {
        if let Some(&i) = self.ids.get(&cell) {
            return json!({"ref": i});
        }
        let i = self.nodes.len();
        self.nodes.push(J::Null);
        self.ids.insert(cell, i);
        self.queue.push_back((cell, i));
        json!({"ref": i})
    }

    pub fn value(&mut self, v: Value) -> J {
        let h = self.heap;
        match v.kind() {
            Kind::Undefined => json!({"undefined": true}),
            Kind::Null => J::Null,
            Kind::Bool => json!(v.as_bool().unwrap()),
            Kind::Number => num(v.as_number().unwrap()),
            Kind::Hole => json!({"hole": true}),
            Kind::Uninitialized => json!({"uninitialized": true}),
            Kind::String => json!(h.str_to_string(v.as_string().unwrap())),
            Kind::Object => {
                let o = v.as_object().unwrap();
                if let Some(p) = self.builtins.get(&o.offset()) {
                    return json!({"builtin": p});
                }
                self.node(o.offset())
            }
            Kind::Internal => self.node(v.as_internal().unwrap()),
            k => {
                self.errors.push(format!("a {k:?} value"));
                json!({"unsupported": format!("{k:?}")})
            }
        }
    }

    fn props(&mut self, o: Obj, skip_indexes: bool) -> J {
        let h = self.heap;
        let mut out = Vec::new();
        for k in h.own_keys(o) {
            if h.atom_symbol(k).is_some() {
                self.errors.push("a symbol-keyed property".into());
                continue;
            }
            let name = h.atom_name(k);
            if skip_indexes && (name.parse::<u32>().is_ok() || name == "length") {
                continue;
            }
            match h.get_own_property(o, k) {
                Some(Property::Data { value, flags }) => {
                    let v = self.value(value);
                    out.push(json!([name, flags.bits(), v]));
                }
                Some(Property::Accessor {
                    getter,
                    setter,
                    flags,
                }) => {
                    let g = self.value(getter);
                    let s = self.value(setter);
                    out.push(json!([name, flags.bits(), {"get": g, "set": s}]));
                }
                None => {}
            }
        }
        J::Array(out)
    }

    fn proto(&mut self, o: Obj) -> J {
        match self.heap.get_prototype_of(o) {
            Some(p) => self.value(Value::object(p)),
            None => J::Null,
        }
    }

    fn function_id(&self, t: &TemplateInfo) -> J {
        json!({"start": t.span.0, "end": t.span.1})
    }

    fn record(&mut self, cell: u32) -> J {
        let h = self.heap;
        let o = obj_at(cell);
        let Some(schema) = self.record_schema.get(&cell).copied() else {
            self.errors.push("a record of unknown schema".into());
            return json!({"t": "record", "schema": "?"});
        };
        let names = record_fields(schema);
        let n = h.object_slot_count(o);
        let mut fields = serde_json::Map::new();
        for i in 0..n {
            let Some(name) = names.get(i as usize) else {
                self.errors.push(format!("record {schema}: slot {i} has no name"));
                continue;
            };
            let v = h.object_slot(o, i);
            if let Some(c) = v.as_object().map(|o| o.offset())
                && h.object_class(obj_at(c)) == ObjectClass::Record
            {
                // The element records share a counter record.
                self.record_schema.insert(c, "Counter");
            }
            fields.insert((*name).into(), self.value(v));
        }
        json!({"t": "record", "schema": schema, "fields": fields})
    }

    fn object(&mut self, cell: u32) -> J {
        let h = self.heap;
        let o = obj_at(cell);
        let class = h.object_class(o);
        let extensible = true;
        match class {
            ObjectClass::Ordinary | ObjectClass::Error => {
                let proto = self.proto(o);
                let props = self.props(o, false);
                json!({"t": "object", "class": format!("{class:?}"), "proto": proto, "props": props, "extensible": extensible})
            }
            ObjectClass::Array => {
                let proto = self.proto(o);
                let len = h.object_slot(o, 0).as_number().unwrap_or(0.0) as u32;
                let el = elements_of(h, o);
                let mut elements = Vec::new();
                for i in 0..len {
                    let v = if el != 0 && i < va_len(h, el) {
                        va_get(h, el, i)
                    } else {
                        Value::HOLE
                    };
                    elements.push(self.value(v));
                }
                let props = self.props(o, true);
                json!({"t": "array", "proto": proto, "elements": elements, "props": props})
            }
            ObjectClass::Function => {
                let t = template_info(h, closure_template(h, o).unwrap());
                if t.lines.is_empty() {
                    // Privileged code (the prelude) is part of the engine: it must be
                    // named by path, never carried.
                    self.errors.push("an unnamed builtin function".into());
                }
                let mut captures = Vec::new();
                for i in 0..t.captures {
                    let name = t
                        .names
                        .get((t.locals + i) as usize)
                        .cloned()
                        .unwrap_or_default();
                    let v = h.object_slot(o, 1 + i);
                    captures.push(json!([name, self.value(v)]));
                }
                let proto = self.proto(o);
                let props = self.props(o, false);
                json!({"t": "closure", "fn": self.function_id(&t), "captures": captures, "proto": proto, "props": props})
            }
            ObjectClass::BuiltinClosure => {
                let id = h.object_slot(o, 0).as_int().unwrap_or(-1);
                let name = INTRINSICS
                    .get(id as usize)
                    .map(|i| i.name)
                    .unwrap_or("?");
                let data = h.object_slot(o, 1);
                if let Some(d) = data.as_object()
                    && h.object_class(d) == ObjectClass::Record
                    && name.starts_with("PromiseAll")
                {
                    self.record_schema.insert(d.offset(), "PromiseElement");
                }
                let data = self.value(data);
                let props = self.props(o, false);
                json!({"t": "builtin-closure", "intrinsic": name, "data": data, "props": props})
            }
            ObjectClass::Record => self.record(cell),
            ObjectClass::Promise => {
                let state = h.object_slot(o, P_STATE).as_int().unwrap_or(0);
                let result = self.value(h.object_slot(o, P_RESULT));
                let flags = h.object_slot(o, P_FLAGS).as_int().unwrap_or(0);
                let mut reactions = Vec::new();
                if let Some(r) = h.object_slot(o, P_REACTIONS).as_internal() {
                    let n = va_len(h, r);
                    for k in 0..n / 3 {
                        let cap = va_get(h, r, 3 * k);
                        let f = va_get(h, r, 3 * k + 1);
                        let rj = va_get(h, r, 3 * k + 2);
                        let (cap, f, rj) = (self.value(cap), self.value(f), self.value(rj));
                        reactions.push(json!({"capability": cap, "onFulfilled": f, "onRejected": rj}));
                    }
                }
                let proto = self.proto(o);
                let props = self.props(o, false);
                json!({"t": "promise", "state": state, "result": result, "handled": flags & 1 != 0,
                       "alreadyResolved": flags & 2 != 0, "reactions": reactions, "proto": proto, "props": props})
            }
            ObjectClass::ArrayIterator => {
                let target = self.value(h.object_slot(o, 0));
                let index = h.object_slot(o, 1).as_number().unwrap_or(0.0);
                let kind = h.object_slot(o, 2).as_int().unwrap_or(1);
                json!({"t": "array-iterator", "target": target, "index": index, "kind": kind})
            }
            ObjectClass::Generator => self.activation(o),
            c => {
                self.errors.push(format!("an object of class {c:?}"));
                json!({"t": "unsupported", "class": format!("{c:?}")})
            }
        }
    }

    fn activation(&mut self, g: Obj) -> J {
        let h = self.heap;
        let kind = h.object_slot(g, G_KIND).as_int().unwrap_or(-1);
        let state = h.object_slot(g, G_STATE).as_int().unwrap_or(-1);
        if kind != KIND_ASYNC || state != AWAITING {
            self.errors
                .push(format!("an activation of kind {kind} in state {state}"));
            return json!({"t": "unsupported"});
        }
        let f = frame_of(h, g).expect("a saved frame");
        let t = template_info(h, f.template);
        let (await_pc, site) = t
            .awaits()
            .into_iter()
            .find(|a| a.2 == f.pc)
            .map(|a| (a.0, a.1))
            .expect("resumes after an await");
        let callee = self.value(Value::object(f.callee));
        let this = self.value(f.values[1]);
        let argc = f.argc();
        let args: Vec<J> = (0..argc.max(t.params))
            .map(|i| f.values[2 + i as usize])
            .collect::<Vec<_>>()
            .into_iter()
            .map(|v| self.value(v))
            .collect();
        let mut bindings = Vec::new();
        let mut seen: HashMap<String, u32> = HashMap::new();
        for (i, &v) in f.locals(&t).iter().enumerate() {
            let name = t.names.get(i).cloned().unwrap_or_default();
            if name == "<state>" {
                continue;
            }
            if name.is_empty() || name.starts_with('<') {
                if !v.is_undefined() {
                    self.errors.push(format!("hidden local {name:?} holds a value"));
                }
                continue;
            }
            let k = seen.entry(name.clone()).or_insert(0);
            let key = if *k == 0 { name.clone() } else { format!("{name}#{k}") };
            *k += 1;
            bindings.push(json!([key, self.value(v)]));
        }
        let labels = labels_at(h, &t, await_pc).unwrap_or_default();
        let ops = f.operands(&t).to_vec();
        if labels.len() != ops.len() {
            self.errors.push(format!(
                "operand roles: {} labels for {} operands",
                labels.len(),
                ops.len()
            ));
        }
        let operands: Vec<J> = ops
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let role = labels.get(i).cloned().unwrap_or("?".into());
                json!([role, self.value(v)])
            })
            .collect();
        let promise = self.value(h.object_slot(g, G_PROMISE));
        let fid = self.function_id(&t);
        json!({"t": "async", "fn": fid, "callee": callee, "site": {"line": site.0, "col": site.1},
               "this": this, "argc": argc, "args": args, "bindings": bindings,
               "operands": operands, "promise": promise})
    }

    fn drain(&mut self) {
        while let Some((cell, i)) = self.queue.pop_front() {
            let h = self.heap;
            let node = match h.cell_kind(cell) {
                djs_core::heap::CellKind::Object => self.object(cell),
                djs_core::heap::CellKind::BoxCell => {
                    let v: Value = h.arena().get(cell, djs_core::heap::cells::BoxCell::VALUE);
                    json!({"t": "box", "value": self.value(v)})
                }
                k => {
                    self.errors.push(format!("a {k:?} cell"));
                    json!({"t": "unsupported"})
                }
            };
            self.nodes[i] = node;
        }
    }
}

/// Exports a waiting session. Fails on anything the format cannot name.
pub fn export(s: &mut Session, source: &str) -> Result<J, Vec<String>> {
    s.vm.collect().expect("collect");
    let build = format!("{:08x}", s.vm.heap().arena().get::<u32>(0, djs_core::heap::cells::Roots::BUILD));
    let heap = s.vm.heap();
    let mut ex = Exporter::new(heap);
    let promises = pending_promises(heap);
    let mut pending = Vec::new();
    for (id, seq, name, arg) in &s.pending {
        let p = promises
            .iter()
            .find(|(c, _)| c == id)
            .map(|(_, p)| *p)
            .expect("a pending promise");
        let pv = ex.value(Value::object(p));
        pending.push(json!({"seq": seq, "fn": "tool", "args": [name, arg], "promise": pv}));
    }
    ex.drain();
    if !ex.errors.is_empty() {
        return Err(ex.errors);
    }
    Ok(json!({
        "format": "djs-portable-prototype/1",
        "exportedBy": build,
        "source": source,
        "issued": s.issued,
        "output": s.output,
        "pending": pending,
        "nodes": ex.nodes,
    }))
}
