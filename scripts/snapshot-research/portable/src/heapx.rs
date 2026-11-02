//! Heap helpers shared by the exporter, the importer and the measurements: reading
//! templates, frames and values through djs-core's public heap API. Everything here
//! must compile against the older djs tree too (README.md).

use std::collections::{HashMap, VecDeque};

use djs_core::bytecode::decode::decode;
use djs_core::bytecode::{HandlerTable, bytes_of, lookup_line};
use djs_core::heap::cells::{FuncTemplate, Object, RefArray, Roots, Script, U32Array, ValueArray};
use djs_core::heap::{Atom, Heap, ObjectClass, Property, Ref};
use djs_core::interp::{Intrinsic, intrinsic};
use djs_core::value::{Obj, Str, TAG_PREFIX, Value, tag};

pub const HEADER: u32 = 6;

// Generator slots (interp/run/suspend.rs), unchanged since D1375.
pub const G_STATE: u32 = 0;
pub const G_FRAME: u32 = 1;
pub const G_PC: u32 = 2;
pub const G_HP: u32 = 3;
pub const G_PROMISE: u32 = 4;
pub const G_KIND: u32 = 5;
pub const AWAITING: i32 = 4;
pub const KIND_ASYNC: i32 = 1;

// Promise slots (interp/run/promise.rs), unchanged since D1371.
pub const P_STATE: u32 = 0;
pub const P_RESULT: u32 = 1;
pub const P_REACTIONS: u32 = 2;
pub const P_FLAGS: u32 = 3;

pub fn obj_at(off: u32) -> Obj {
    Value::from_bits(TAG_PREFIX | (tag::OBJECT as u64) << 32 | off as u64)
        .and_then(Value::as_object)
        .expect("an object offset")
}

pub fn str_at(off: u32) -> Str {
    Value::from_bits(TAG_PREFIX | (tag::STRING as u64) << 32 | off as u64)
        .and_then(Value::as_string)
        .expect("a string offset")
}

pub fn has_field(heap: &Heap, cell: u32, field_offset: u32) -> bool {
    heap.cell_size(cell) >= field_offset + 4
}

/// The template of a JS closure object.
pub fn closure_template(heap: &Heap, f: Obj) -> Option<u32> {
    if heap.object_class(f) != ObjectClass::Function {
        return None;
    }
    heap.object_slot(f, 0).as_internal()
}

pub struct TemplateInfo {
    pub off: u32,
    pub locals: u32,
    pub params: u32,
    pub captures: u32,
    pub names: Vec<String>,
    pub span: (u32, u32),
    pub script: String,
    pub code: Vec<u8>,
    pub lines: Vec<u8>,
    pub handlers: Vec<djs_core::bytecode::Handler>,
    pub children: Vec<u32>,
}

pub fn template_info(heap: &Heap, t: u32) -> TemplateInfo {
    let a = heap.arena();
    let names = if has_field(heap, t, FuncTemplate::NAMES.offset) {
        let r: Ref = a.get(t, FuncTemplate::NAMES);
        match r.get() {
            Some(n) => {
                let len: u32 = a.get(n, U32Array::LEN);
                (0..len)
                    .map(|i| heap.atom_name(Atom::from_id(a.read::<u32>(U32Array::DATA.at(n, i)))))
                    .collect()
            }
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let span = if has_field(heap, t, FuncTemplate::SRC_END.offset) {
        (
            a.get(t, FuncTemplate::SRC_START),
            a.get(t, FuncTemplate::SRC_END),
        )
    } else {
        (0, 0)
    };
    let script = if has_field(heap, t, FuncTemplate::SCRIPT.offset) {
        let s: Ref = a.get(t, FuncTemplate::SCRIPT);
        s.get()
            .and_then(|s| a.get::<Ref>(s, Script::NAME).get())
            .map(|n| heap.str_to_string(str_at(n)))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let lines = if has_field(heap, t, FuncTemplate::LINES.offset) {
        bytes_of(heap, a.get(t, FuncTemplate::LINES)).to_vec()
    } else {
        Vec::new()
    };
    let hb = bytes_of(heap, a.get(t, FuncTemplate::HANDLERS)).to_vec();
    let handlers = {
        let table = HandlerTable::new(&hb).expect("handler table");
        (0..table.len()).filter_map(|i| table.get(i)).collect()
    };
    let children = {
        let r: Ref = a.get(t, FuncTemplate::CHILDREN);
        match r.get() {
            Some(c) => {
                let n: u32 = a.get(c, RefArray::LEN);
                (0..n)
                    .filter_map(|i| a.read::<Ref>(RefArray::REFS.at(c, i)).get())
                    .collect()
            }
            None => Vec::new(),
        }
    };
    TemplateInfo {
        off: t,
        locals: a.get::<u16>(t, FuncTemplate::LOCALS) as u32,
        params: a.get::<u16>(t, FuncTemplate::PARAMS) as u32,
        captures: a.get::<u16>(t, FuncTemplate::CAPTURES) as u32,
        names,
        span,
        script,
        code: bytes_of(heap, a.get(t, FuncTemplate::CODE)).to_vec(),
        lines,
        handlers,
        children,
    }
}

impl TemplateInfo {
    /// Every `await` of the code: (pc of the await, its line:col, the pc after it).
    pub fn awaits(&self) -> Vec<(u32, (u32, u32), u32)> {
        let mut out = Vec::new();
        let mut pc = 0u32;
        while (pc as usize) < self.code.len() {
            let insn = decode(&self.code, pc).expect("decodes");
            if insn.info().name == "await" {
                let lc = lookup_line(&self.lines, pc).unwrap_or((0, 0));
                out.push((pc, lc, insn.next_pc()));
            }
            pc = insn.next_pc();
        }
        out
    }
}

/// A suspended frame, read from a Generator object.
pub struct Frame {
    pub callee: Obj,
    pub template: u32,
    pub pc: u32,
    pub hp: u32,
    pub values: Vec<Value>,
}

pub fn frame_of(heap: &Heap, g: Obj) -> Option<Frame> {
    let arr = heap.object_slot(g, G_FRAME).as_internal()?;
    let n = va_len(heap, arr);
    let values: Vec<Value> = (0..n).map(|i| va_get(heap, arr, i)).collect();
    let callee = values.first()?.as_object()?;
    let template = closure_template(heap, callee)?;
    Some(Frame {
        callee,
        template,
        pc: heap.object_slot(g, G_PC).as_int()? as u32,
        hp: heap.object_slot(g, G_HP).as_int()? as u32,
        values,
    })
}

impl Frame {
    pub fn argc(&self) -> u32 {
        self.values[(self.hp + 3) as usize].as_int().unwrap_or(0) as u32
    }
    pub fn locals<'a>(&'a self, t: &TemplateInfo) -> &'a [Value] {
        let lp = (self.hp + HEADER) as usize;
        &self.values[lp..lp + t.locals as usize]
    }
    pub fn operands<'a>(&'a self, t: &TemplateInfo) -> &'a [Value] {
        let sb = (self.hp + HEADER + t.locals) as usize;
        &self.values[sb..]
    }
}

/// Canonical names of the realm's objects: the shortest property path from the global
/// object (ties broken by key), plus the realm intrinsics that no path reaches, by name.
/// Both sides of a transfer compute them on their own realm; the importer resolves a
/// name by walking it, so a new builtin elsewhere does not change an old name's meaning.
pub fn builtin_paths(heap: &Heap) -> HashMap<u32, String> {
    builtin_paths_in(heap, None)
}

/// [`builtin_paths`] of a session's heap, keeping only the objects that a fresh realm
/// (`reference`: path -> class) has at the same path: what a script or a prelude added to
/// the global object is state, not a builtin.
pub fn builtin_paths_in(
    heap: &Heap,
    reference: Option<&HashMap<String, ObjectClass>>,
) -> HashMap<u32, String> {
    let mut paths: HashMap<u32, String> = HashMap::new();
    let mut queue = VecDeque::new();
    let global = intrinsic(heap, Intrinsic::Global).as_object().unwrap();
    paths.insert(global.offset(), "globalThis".into());
    queue.push_back((global, String::from("globalThis"), 0));
    // The realm's intrinsics by name first: %ArrayIteratorProto%/next is reachable
    // from no global.
    for (i, name) in REALM_INTRINSICS {
        if let Some(o) = intrinsic(heap, *i).as_object()
            && !paths.contains_key(&o.offset())
        {
            let p = format!("%{name}%");
            paths.insert(o.offset(), p.clone());
            queue.push_back((o, p, 0));
        }
    }
    while let Some((o, path, depth)) = queue.pop_front() {
        if depth >= 4 {
            continue;
        }
        let mut edges: Vec<(String, Value)> = Vec::new();
        for k in heap.own_keys(o) {
            if heap.atom_symbol(k).is_some() {
                continue;
            }
            let name = heap.atom_name(k);
            match heap.get_own_property(o, k) {
                Some(Property::Data { value, .. }) => edges.push((name, value)),
                Some(Property::Accessor { getter, setter, .. }) => {
                    edges.push((format!("get {name}"), getter));
                    edges.push((format!("set {name}"), setter));
                }
                None => {}
            }
        }
        if let Some(p) = heap.get_prototype_of(o) {
            edges.push(("<proto>".into(), Value::object(p)));
        }
        edges.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in edges {
            let Some(t) = v.as_object() else { continue };
            if paths.contains_key(&t.offset()) {
                continue;
            }
            let p = format!("{path}/{k}");
            if let Some(r) = reference
                && r.get(&p) != Some(&heap.object_class(t))
            {
                continue;
            }
            paths.insert(t.offset(), p.clone());
            queue.push_back((t, p, depth + 1));
        }
    }
    paths
}

pub const REALM_INTRINSICS: &[(Intrinsic, &str)] = &[
    (Intrinsic::ObjectProto, "ObjectProto"),
    (Intrinsic::FunctionProto, "FunctionProto"),
    (Intrinsic::ArrayProto, "ArrayProto"),
    (Intrinsic::ErrorProto, "ErrorProto"),
    (Intrinsic::ArrayIteratorProto, "ArrayIteratorProto"),
    (Intrinsic::AsyncFunctionProto, "AsyncFunctionProto"),
    (Intrinsic::PromiseProto, "PromiseProto"),
];

/// Resolves a canonical builtin name on this realm.
pub fn resolve_builtin(heap: &mut Heap, path: &str) -> Option<Value> {
    let mut parts = path.split('/');
    let first = parts.next()?;
    let mut cur = if first == "globalThis" {
        intrinsic(heap, Intrinsic::Global)
    } else {
        let name = first.strip_prefix('%')?.strip_suffix('%')?;
        let (i, _) = REALM_INTRINSICS.iter().find(|(_, n)| *n == name)?;
        intrinsic(heap, *i)
    };
    for part in parts {
        let o = cur.as_object()?;
        if part == "<proto>" {
            cur = Value::object(heap.get_prototype_of(o)?);
            continue;
        }
        let (which, key) = match part.split_once(' ') {
            Some(("get", k)) => (1, k),
            Some(("set", k)) => (2, k),
            _ => (0, part),
        };
        let atom = heap.atom_str(key).ok()?;
        cur = match (heap.get_own_property(o, atom)?, which) {
            (Property::Data { value, .. }, 0) => value,
            (Property::Accessor { getter, .. }, 1) => getter,
            (Property::Accessor { setter, .. }, 2) => setter,
            _ => return None,
        };
    }
    Some(cur)
}

/// The pending async host calls: (call id, its HostCall cell's promise), oldest first.
pub fn pending_promises(heap: &Heap) -> Vec<(u32, Obj)> {
    use djs_core::heap::cells::HostCall;
    let a = heap.arena();
    let table: Ref = a.get(0, Roots::HOST_CALLS);
    let Some(t) = table.get() else { return vec![] };
    let n: u32 = a.get(t, ValueArray::LEN);
    (0..n)
        .filter_map(|i| {
            let c = va_get(heap, t, i).as_internal()?;
            let p: Value = a.get(c, HostCall::PROMISE);
            Some((a.get(c, HostCall::ID), p.as_object()?))
        })
        .collect()
}

#[allow(dead_code)]
pub fn object_slot_count(heap: &Heap, o: Obj) -> u32 {
    heap.object_slot_count(o)
}

pub fn va_len(heap: &Heap, arr: u32) -> u32 {
    heap.arena().get(arr, ValueArray::LEN)
}

pub fn va_get(heap: &Heap, arr: u32, i: u32) -> Value {
    heap.arena().read(ValueArray::VALUES.at(arr, i))
}

/// A new ValueArray holding `values`.
pub fn va_new(heap: &mut Heap, values: &[Value]) -> u32 {
    let n = values.len() as u32;
    let size = ValueArray::VALUES.cell_size(n.max(1)).expect("size");
    let arr = heap
        .alloc_cell(djs_core::heap::CellKind::ValueArray, size)
        .expect("oom");
    for (i, v) in values.iter().enumerate() {
        heap.arena_mut()
            .write(ValueArray::VALUES.at(arr, i as u32), *v);
    }
    heap.arena_mut().set(arr, ValueArray::LEN, n);
    arr
}

/// The dense elements store of an object (0 if none).
pub fn elements_of(heap: &Heap, o: Obj) -> u32 {
    let r: Ref = heap.arena().get(o.offset(), Object::ELEMENTS);
    r.get().unwrap_or(0)
}

/// Tier 2 (0142): the objects a prelude made, named by their path from the global object
/// (`globalThis/tools/<target>/read` for a tool through pi's Proxy), skipping builtins.
pub fn prelude_names(heap: &Heap, builtins: &HashMap<u32, String>) -> HashMap<u32, String> {
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut queue = VecDeque::new();
    let global = intrinsic(heap, Intrinsic::Global).as_object().unwrap();
    queue.push_back((global, String::from("globalThis"), 0));
    while let Some((o, path, depth)) = queue.pop_front() {
        if depth >= 4 {
            continue;
        }
        let mut edges: Vec<(String, Value)> = Vec::new();
        if heap.object_class(o) == ObjectClass::Proxy {
            if let Some(t) = heap.proxy_target(o) {
                edges.push(("<target>".into(), Value::object(t)));
            }
            if let Some(t) = heap.proxy_handler(o) {
                edges.push(("<handler>".into(), Value::object(t)));
            }
        } else {
            for k in heap.own_keys(o) {
                if heap.atom_symbol(k).is_some() {
                    continue;
                }
                let name = heap.atom_name(k);
                match heap.get_own_property(o, k) {
                    Some(Property::Data { value, .. }) => edges.push((name, value)),
                    Some(Property::Accessor { getter, setter, .. }) => {
                        edges.push((format!("get {name}"), getter));
                        edges.push((format!("set {name}"), setter));
                    }
                    None => {}
                }
            }
        }
        edges.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in edges {
            let Some(t) = v.as_object() else { continue };
            if builtins.contains_key(&t.offset()) || names.contains_key(&t.offset()) {
                continue;
            }
            let p = format!("{path}/{k}");
            names.insert(t.offset(), p.clone());
            queue.push_back((t, p, depth + 1));
        }
    }
    names
}

/// Resolves a prelude path on this heap (as `resolve_builtin`, with `<target>` and
/// `<handler>` for proxies).
pub fn resolve_prelude(heap: &mut Heap, path: &str) -> Option<Value> {
    let mut parts = path.split('/');
    if parts.next() != Some("globalThis") {
        return None;
    }
    let mut cur = intrinsic(heap, Intrinsic::Global);
    for part in parts {
        let o = cur.as_object()?;
        cur = match part {
            "<target>" => Value::object(heap.proxy_target(o)?),
            "<handler>" => Value::object(heap.proxy_handler(o)?),
            _ => {
                let (which, key) = match part.split_once(' ') {
                    Some(("get", k)) => (1, k),
                    Some(("set", k)) => (2, k),
                    _ => (0, part),
                };
                let atom = heap.atom_str(key).ok()?;
                match (heap.get_own_property(o, atom)?, which) {
                    (Property::Data { value, .. }, 0) => value,
                    (Property::Accessor { getter, .. }, 1) => getter,
                    (Property::Accessor { setter, .. }, 2) => setter,
                    _ => return None,
                }
            }
        };
    }
    Some(cur)
}

/// The prelude's top-level bindings that its closures capture, by name -> Box cell:
/// reached from the closures among `roots`, through captured closures.
pub fn captured_bindings(heap: &Heap, roots: &[Value]) -> HashMap<String, u32> {
    let mut out: HashMap<String, u32> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut queue: VecDeque<Value> = roots.iter().copied().collect();
    while let Some(v) = queue.pop_front() {
        let Some(o) = v.as_object() else { continue };
        if !seen.insert(o.offset()) {
            continue;
        }
        if heap.object_class(o) == ObjectClass::Proxy {
            if let Some(t) = heap.proxy_handler(o) {
                queue.push_back(Value::object(t));
            }
            continue;
        }
        if let Some(t) = closure_template(heap, o) {
            let info = template_info(heap, t);
            for i in 0..info.captures {
                let Some(b) = heap.object_slot(o, 1 + i).as_internal() else { continue };
                let name = info.names.get((info.locals + i) as usize).cloned().unwrap_or_default();
                out.entry(name).or_insert(b);
                let inner: Value = heap.arena().get(b, djs_core::heap::cells::BoxCell::VALUE);
                queue.push_back(inner);
            }
        }
        // Methods of objects (the api, console, the guard's handler).
        for k in heap.own_keys(o) {
            if let Some(Property::Data { value, .. }) = heap.get_own_property(o, k) {
                if value.as_object().is_some_and(|f| heap.object_class(f) == ObjectClass::Function) {
                    queue.push_back(value);
                }
            }
        }
    }
    out
}
