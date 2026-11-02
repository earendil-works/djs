//! The importer: rebuilds an exported graph in a fresh VM of *this* build. The script is
//! compiled again from the source the snapshot carries; functions are found by source
//! span, the resume point of each activation by its await's source position in this
//! build's code, locals by name in this build's frame layout, operands by role.
//! Builtins are resolved by path, builtin closures by intrinsic name, records by schema.

use std::collections::HashMap;

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::compiler::CompileOptions;
use djs_core::heap::cells::{BoxCell, Object};
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass, PropFlags, PropertyDescriptor, Ref};
use djs_core::interp::{Intrinsic, intrinsic};
use djs_core::value::{Obj, Value};
use serde_json::Value as J;

use crate::export::record_fields;
use crate::heapx::*;
use crate::roles::labels_at;
use crate::session::{Session, new_vm};

type FnKey = (String, u32, u32);

pub struct Importer {
    templates: HashMap<FnKey, TemplateInfo>,
    cells: Vec<u32>,
    kinds: Vec<String>,
}

fn collect_templates(heap: &Heap, t: u32, out: &mut HashMap<FnKey, TemplateInfo>) {
    let info = template_info(heap, t);
    let children = info.children.clone();
    out.insert((info.script.clone(), info.span.0, info.span.1), info);
    for c in children {
        collect_templates(heap, c, out);
    }
}

fn span_of(j: &J) -> FnKey {
    (
        j["script"].as_str().unwrap().to_string(),
        j["start"].as_u64().unwrap() as u32,
        j["end"].as_u64().unwrap() as u32,
    )
}

impl Importer {
    fn value(&self, heap: &mut Heap, j: &J) -> Result<Value, String> {
        Ok(match j {
            J::Null => Value::NULL,
            J::Bool(b) => Value::bool(*b),
            J::Number(n) => Value::number(n.as_f64().unwrap()),
            J::String(s) => Value::string(heap.alloc_str(s).map_err(|e| format!("{e:?}"))?),
            J::Object(m) => {
                if m.contains_key("undefined") {
                    Value::UNDEFINED
                } else if m.contains_key("hole") {
                    Value::HOLE
                } else if m.contains_key("uninitialized") {
                    Value::UNINITIALIZED
                } else if let Some(n) = m.get("num") {
                    Value::number(match n.as_str().unwrap() {
                        "NaN" => f64::NAN,
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        _ => -0.0,
                    })
                } else if let Some(p) = m.get("builtin") {
                    let p = p.as_str().unwrap();
                    resolve_builtin(heap, p).ok_or(format!("no builtin {p} in this build"))?
                } else if let Some(r) = m.get("ref") {
                    let i = r.as_u64().unwrap() as usize;
                    let c = self.cells[i];
                    if self.kinds[i] == "box" {
                        Value::internal(c)
                    } else {
                        Value::object(obj_at(c))
                    }
                } else {
                    return Err(format!("bad value {j}"));
                }
            }
            _ => return Err(format!("bad value {j}")),
        })
    }

    fn define_props(&self, heap: &mut Heap, o: Obj, props: &J) -> Result<(), String> {
        for p in props.as_array().unwrap() {
            let name = p[0].as_str().unwrap();
            let flags = PropFlags::from_bits(p[1].as_u64().unwrap() as u8);
            let atom = heap.atom_str(name).map_err(|e| format!("{e:?}"))?;
            let d = if let Some(acc) = p[2].as_object().filter(|m| m.contains_key("get")) {
                let g = self.value(heap, &acc["get"])?;
                let s = self.value(heap, &acc["set"])?;
                PropertyDescriptor::accessor(g, s, flags)
            } else {
                PropertyDescriptor::data(self.value(heap, &p[2])?, flags)
            };
            heap.define_own_property(o, atom, &d)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }

    fn proto(&self, heap: &mut Heap, j: &J) -> Result<Option<Obj>, String> {
        Ok(self.value(heap, j)?.as_object())
    }
}

/// Imports a snapshot into a new session of this build.
pub fn import(snap: &J) -> Result<Session, String> {
    let mut vm = new_vm();
    // The code: every script compiled again from its source by this build's compiler.
    // Nothing runs: the state comes from the snapshot.
    let mut templates = HashMap::new();
    let mut scripts = Vec::new();
    for sc in snap["scripts"].as_array().unwrap() {
        let name = sc["name"].as_str().unwrap();
        let source = sc["source"].as_str().unwrap();
        let opts = if name == "script.js" {
            CompileOptions::async_body(name, &[])
        } else {
            CompileOptions::script(name)
        };
        let main = vm
            .compile(source.as_bytes(), &opts)
            .map_err(|e| e.message)?;
        let root = closure_template(vm.heap(), main.as_object().unwrap()).unwrap();
        collect_templates(vm.heap(), root, &mut templates);
        scripts.push((name.to_string(), source.to_string()));
    }

    let nodes = snap["nodes"].as_array().unwrap();
    let mut imp = Importer {
        templates,
        cells: vec![0; nodes.len()],
        kinds: nodes
            .iter()
            .map(|n| n["t"].as_str().unwrap_or("?").to_string())
            .collect(),
    };

    // The pending calls are issued again: the host still owns them (same sequence
    // numbers); the VM gets fresh call ids and promises, which become the snapshot's
    // pending promise nodes.
    let mut pending = Vec::new();
    for p in snap["pending"].as_array().unwrap() {
        let fname = p["fn"].as_str().unwrap();
        let key = vm
            .heap_mut()
            .atom_str(fname)
            .map_err(|e| format!("{e:?}"))?;
        let f = vm
            .heap()
            .get_own_property(vm.global(), key)
            .and_then(|p| p.value())
            .ok_or(format!("no host function {fname}"))?;
        let texts: Vec<String> = p["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect();
        let args: Vec<Value> = texts
            .iter()
            .map(|a| Value::string(vm.heap_mut().alloc_str(a).unwrap()))
            .collect();
        vm.call(f, Value::UNDEFINED, &args)
            .map_err(|e| format!("{e:?}"))?;
        vm.run(1_000_000);
        let ids = vm.new_host_calls();
        let id = *ids.last().ok_or("no host call")?;
        let promise = pending_promises(vm.heap())
            .into_iter()
            .find(|(c, _)| *c == id)
            .unwrap()
            .1;
        let node = p["promise"]["ref"]
            .as_u64()
            .ok_or("pending promise is not a node")? as usize;
        imp.cells[node] = promise.offset();
        pending.push((
            id,
            p["seq"].as_u64().unwrap() as u32,
            fname.to_string(),
            texts,
        ));
    }
    // The embedder's functions, registered again by id.
    for (i, n) in nodes.iter().enumerate() {
        if n["t"] == "host-function" {
            let id = n["id"].as_u64().unwrap() as u32;
            let f = if n["async"].as_bool().unwrap() {
                vm.new_async_host_function(id, "")
            } else {
                vm.new_host_function(id, "")
            }
            .map_err(|e| format!("{e:?}"))?;
            imp.cells[i] = f.as_object().unwrap().offset();
        }
    }

    let heap = vm.heap_mut();
    let fproto = intrinsic(heap, Intrinsic::FunctionProto).as_object();
    // Pass 1: a cell for every node.
    for (i, n) in nodes.iter().enumerate() {
        if imp.cells[i] != 0 {
            continue;
        }
        let o = |heap: &mut Heap, class: ObjectClass, slots: u32| {
            heap.alloc_object(class, None, slots).map(|o| o.offset())
        };
        let cell = match n["t"].as_str().unwrap() {
            "object" => {
                let class = match n["class"].as_str().unwrap() {
                    "Error" => ObjectClass::Error,
                    _ => ObjectClass::Ordinary,
                };
                o(heap, class, class.slots())
            }
            "array" => heap.new_array(None, 0).map(|o| o.offset()),
            "collection" => {
                let class = if n["class"] == "Map" {
                    ObjectClass::Map
                } else {
                    ObjectClass::Set
                };
                let p = intrinsic(heap, Intrinsic::ObjectProto).as_object().unwrap();
                heap.collection_new(class, p).map(|o| o.offset())
            }
            "date" => o(heap, ObjectClass::Date, 1),
            "closure" => {
                let span = span_of(&n["fn"]);
                let t = imp
                    .templates
                    .get(&span)
                    .ok_or(format!("no function at {span:?} in this build"))?;
                o(heap, ObjectClass::Function, 1 + t.captures)
            }
            "builtin-closure" => o(heap, ObjectClass::BuiltinClosure, 2),
            "bound" => o(heap, ObjectClass::BoundFunction, 3),
            "record" => {
                let schema = n["schema"].as_str().unwrap();
                let fields = record_fields(schema);
                if fields.is_empty() {
                    return Err(format!("unknown record schema {schema}"));
                }
                o(heap, ObjectClass::Record, fields.len() as u32)
            }
            "promise" => o(heap, ObjectClass::Promise, 4),
            "array-iterator" => o(heap, ObjectClass::ArrayIterator, 3),
            "async" => o(heap, ObjectClass::Generator, ObjectClass::Generator.slots()),
            "box" => heap.alloc_cell(CellKind::BoxCell, 16),
            t => return Err(format!("cannot import a {t} node")),
        }
        .map_err(|e| format!("{e:?}"))?;
        imp.cells[i] = cell;
    }

    // Pass 2: contents.
    for (i, n) in nodes.iter().enumerate() {
        let cell = imp.cells[i];
        let t = n["t"].as_str().unwrap();
        if t == "box" {
            let v = imp.value(heap, &n["value"])?;
            heap.arena_mut().set(cell, BoxCell::VALUE, v);
            continue;
        }
        let o = obj_at(cell);
        if let Some(p) = n.get("proto") {
            let p = imp.proto(heap, p)?;
            heap.set_prototype_of(o, p);
        }
        match t {
            "object" => {
                imp.define_props(heap, o, &n["props"])?;
                if let Some(stack) = n.get("stack").and_then(|s| s.as_str()) {
                    // The trace as text: an own `stack`, which error_stack prefers.
                    let k = heap.atom_str("stack").map_err(|e| format!("{e:?}"))?;
                    let v = Value::string(heap.alloc_str(stack).map_err(|e| format!("{e:?}"))?);
                    let d = PropertyDescriptor::data(v, PropFlags::new(true, false, true));
                    heap.define_own_property(o, k, &d)
                        .map_err(|e| format!("{e:?}"))?;
                }
            }
            "host-function" => {
                // Recreated with its own `name` and `length`: replaced by the snapshot's.
                for key in ["name", "length"] {
                    let k = heap.atom_str(key).map_err(|e| format!("{e:?}"))?;
                    // (A Result here, a bool in older trees.)
                    let _ = heap.delete_own_property(o, k);
                }
                imp.define_props(heap, o, &n["props"])?;
            }
            "collection" => {
                for e in n["entries"].as_array().unwrap() {
                    let k = imp.value(heap, &e[0])?;
                    let v = match e.get(1) {
                        Some(j) => imp.value(heap, j)?,
                        None => Value::TRUE,
                    };
                    heap.collection_set(o, k, v).map_err(|e| format!("{e:?}"))?;
                }
                imp.define_props(heap, o, &n["props"])?;
            }
            "bound" => {
                for (k, f) in ["target", "this", "args"].iter().enumerate() {
                    let v = imp.value(heap, &n[*f])?;
                    heap.set_object_slot(o, k as u32, v);
                }
                imp.define_props(heap, o, &n["props"])?;
            }
            "date" => {
                let t = match &n["time"] {
                    J::Number(x) => x.as_f64().unwrap(),
                    _ => f64::NAN,
                };
                heap.set_object_slot(o, 0, Value::number(t));
                imp.define_props(heap, o, &n["props"])?;
            }
            "array" => {
                let mut values = Vec::new();
                for e in n["elements"].as_array().unwrap() {
                    values.push(imp.value(heap, e)?);
                }
                // Dense elements, written as an array of this build would hold them.
                let tmp = heap
                    .new_array_from_values(None, &values)
                    .map_err(|e| format!("{e:?}"))?;
                let el: Ref = heap.arena().get(tmp.offset(), Object::ELEMENTS);
                heap.arena_mut().set(o.offset(), Object::ELEMENTS, el);
                heap.set_object_slot(o, 0, Value::number(values.len() as f64));
                imp.define_props(heap, o, &n["props"])?;
            }
            "closure" => {
                let span = span_of(&n["fn"]);
                let tinfo = &imp.templates[&span];
                heap.set_object_slot(o, 0, Value::internal(tinfo.off));
                let caps: HashMap<&str, &J> = n["captures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| (c[0].as_str().unwrap(), &c[1]))
                    .collect();
                for k in 0..tinfo.captures {
                    let name = tinfo.names[(tinfo.locals + k) as usize].clone();
                    let j = caps.get(name.as_str()).ok_or(format!(
                        "closure {span:?}: no capture {name} in the snapshot"
                    ))?;
                    let v = imp.value(heap, j)?;
                    heap.set_object_slot(o, 1 + k, v);
                }
                if caps.len() != tinfo.captures as usize {
                    return Err(format!("closure {span:?}: captures differ"));
                }
                imp.define_props(heap, o, &n["props"])?;
            }
            "builtin-closure" => {
                let name = n["intrinsic"].as_str().unwrap();
                let id = INTRINSICS
                    .iter()
                    .position(|i| i.name == name)
                    .ok_or(format!("no intrinsic {name} in this build"))?;
                heap.set_object_slot(o, 0, Value::int(id as i32));
                if std::env::var("PORTABLE_DEBUG").is_ok() {
                    eprintln!(
                        "builtin closure {name}: exporter id {} -> here {id}",
                        n["_build"]["id"]
                    );
                }
                let d = imp.value(heap, &n["data"])?;
                heap.set_object_slot(o, 1, d);
                heap.set_prototype_of(o, fproto);
                imp.define_props(heap, o, &n["props"])?;
            }
            "record" => {
                let schema = n["schema"].as_str().unwrap();
                let fields = record_fields(schema);
                let given = n["fields"].as_object().unwrap();
                for (k, f) in fields.iter().enumerate() {
                    let v = match given.get(*f) {
                        Some(j) => imp.value(heap, j)?,
                        // A field this build added: its default.
                        None => Value::UNDEFINED,
                    };
                    heap.set_object_slot(o, k as u32, v);
                }
                for (f, j) in given {
                    if !fields.contains(&f.as_str()) && !j.get("undefined").is_some() {
                        return Err(format!("record {schema}: this build has no field {f}"));
                    }
                }
            }
            "promise" => {
                let state = n["state"].as_i64().unwrap() as i32;
                let result = imp.value(heap, &n["result"])?;
                let flags = n["handled"].as_bool().unwrap() as i32
                    | (n["alreadyResolved"].as_bool().unwrap() as i32) << 1;
                let mut triples = Vec::new();
                for r in n["reactions"].as_array().unwrap() {
                    triples.push(imp.value(heap, &r["capability"])?);
                    triples.push(imp.value(heap, &r["onFulfilled"])?);
                    triples.push(imp.value(heap, &r["onRejected"])?);
                }
                heap.set_object_slot(o, P_STATE, Value::int(state));
                heap.set_object_slot(o, P_RESULT, result);
                heap.set_object_slot(o, P_FLAGS, Value::int(flags));
                let r = if triples.is_empty() {
                    Value::UNDEFINED
                } else {
                    Value::internal(va_new(heap, &triples))
                };
                heap.set_object_slot(o, P_REACTIONS, r);
                imp.define_props(heap, o, &n["props"])?;
            }
            "array-iterator" => {
                let target = imp.value(heap, &n["target"])?;
                heap.set_object_slot(o, 0, target);
                heap.set_object_slot(o, 1, Value::number(n["index"].as_f64().unwrap()));
                heap.set_object_slot(o, 2, Value::int(n["kind"].as_i64().unwrap() as i32));
                let p = intrinsic(heap, Intrinsic::ArrayIteratorProto).as_object();
                heap.set_prototype_of(o, p);
            }
            "async" => import_activation(&imp, heap, o, n)?,
            _ => unreachable!(),
        }
        if n["extensible"] == J::Bool(false) {
            let f = heap.cell_flags(cell);
            heap.set_cell_flags(cell, f & !djs_core::heap::cells::flags::OBJ_EXTENSIBLE);
        }
    }

    let g = vm.global();
    imp.define_props(vm.heap_mut(), g, &snap["globals"])?;
    let mut s = Session::with_vm(vm);
    s.scripts = scripts;
    for (name, j) in snap["roots"].as_object().unwrap() {
        let v = imp.value(s.vm.heap_mut(), j)?;
        let h =
            s.vm.heap_mut()
                .new_handle(v)
                .map_err(|e| format!("{e:?}"))?;
        s.roots.push((name.clone(), h));
    }
    s.pending = pending;
    s.issued = snap["issued"].as_u64().unwrap() as u32;
    s.output = snap["output"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    s.finished = None;
    Ok(s)
}

fn import_activation(imp: &Importer, heap: &mut Heap, g: Obj, n: &J) -> Result<(), String> {
    let span = span_of(&n["fn"]);
    let t = imp
        .templates
        .get(&span)
        .ok_or(format!("no function at {span:?}"))?;
    let site = (
        n["site"]["line"].as_u64().unwrap() as u32,
        n["site"]["col"].as_u64().unwrap() as u32,
    );
    // The suspension point in this build's code: the await at the same source position.
    let (await_pc, _, resume_pc) = t
        .awaits()
        .into_iter()
        .find(|a| a.1 == site)
        .ok_or(format!("no await at {site:?} in this build"))?;
    let callee = imp.value(heap, &n["callee"])?;
    let this = imp.value(heap, &n["this"])?;
    let argc = n["argc"].as_u64().unwrap() as u32;
    let mut args = Vec::new();
    for a in n["args"].as_array().unwrap() {
        args.push(imp.value(heap, a)?);
    }
    while (args.len() as u32) < argc.max(t.params) {
        args.push(Value::UNDEFINED);
    }
    let hp = 2 + args.len() as u32;
    let mut frame = vec![callee, this];
    frame.extend(args);
    // The header: links are rewritten by the resume; argc and new.target are kept.
    frame.extend([
        Value::int(0),
        Value::int(0),
        Value::int(0),
        Value::int(argc as i32),
        Value::int(0),
        Value::UNDEFINED,
    ]);
    // Locals in this build's slots, by name.
    let mut bindings: HashMap<String, &J> = n["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (b[0].as_str().unwrap().to_string(), &b[1]))
        .collect();
    let mut seen: HashMap<String, u32> = HashMap::new();
    for i in 0..t.locals {
        let name = t.names.get(i as usize).cloned().unwrap_or_default();
        let v = if name == "<state>" {
            Value::object(g)
        } else if name.is_empty() || name.starts_with('<') {
            Value::UNDEFINED
        } else {
            let k = seen.entry(name.clone()).or_insert(0);
            let key = if *k == 0 {
                name.clone()
            } else {
                format!("{name}#{k}")
            };
            *k += 1;
            let j = bindings
                .remove(&key)
                .ok_or(format!("binding {key} is not in the snapshot"))?;
            imp.value(heap, j)?
        };
        frame.push(v);
    }
    if !bindings.is_empty() {
        return Err(format!(
            "bindings this build does not have: {:?}",
            bindings.keys().collect::<Vec<_>>()
        ));
    }
    // Operands by role: this build's stack map must hold the same roles.
    let roles = labels_at(heap, t, await_pc).ok_or("no stack map")?;
    let ops = n["operands"].as_array().unwrap();
    let theirs: Vec<&str> = ops.iter().map(|o| o[0].as_str().unwrap()).collect();
    if roles != theirs {
        return Err(format!(
            "operand roles differ at {site:?}: snapshot {theirs:?}, this build {roles:?}"
        ));
    }
    for o in ops {
        frame.push(imp.value(heap, &o[1])?);
    }
    if std::env::var("PORTABLE_DEBUG").is_ok() {
        eprintln!(
            "activation {}:{}-{} await at {}:{}: exporter pc {} frame {} locals {} code {} B -> here pc {} frame {} locals {} code {} B, roles {:?}",
            span.0,
            span.1,
            span.2,
            site.0,
            site.1,
            n["_build"]["pc"],
            n["_build"]["frame"],
            n["_build"]["locals"],
            n["_build"]["code"],
            resume_pc,
            frame.len(),
            t.locals,
            t.code.len(),
            roles
        );
    }
    let arr = va_new(heap, &frame);
    let promise = imp.value(heap, &n["promise"])?;
    for (slot, v) in [
        (G_STATE, Value::int(AWAITING)),
        (G_FRAME, Value::internal(arr)),
        (G_PC, Value::int(resume_pc as i32)),
        (G_HP, Value::int(hp as i32)),
        (G_PROMISE, promise),
        (G_KIND, Value::int(KIND_ASYNC)),
        (6, Value::UNDEFINED),
    ] {
        heap.set_object_slot(g, slot, v);
    }
    let _ = Atom::from_id(0);
    Ok(())
}
