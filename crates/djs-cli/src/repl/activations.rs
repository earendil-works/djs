//! Suspended activations, read from the heap for `.pending` and `.bt` (0174): async
//! functions and generators parked at an `await` or `yield`, where they stopped and what
//! their named bindings hold, and which of them wait on a host call.

use djs_core::bytecode::template::{bytes_of, debug_info};
use djs_core::heap::cells::{BoxCell, FuncTemplate, HostCall, U32Array, ValueArray};
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass};
use djs_core::interp::frame::HEADER;
use djs_core::value::{Obj, Value};

use super::inspect;

/// Generator object slots (interp/run/suspend.rs).
const STATE: u32 = 0;
const FRAME: u32 = 1;
const PC: u32 = 2;
const HP: u32 = 3;
const PROMISE: u32 = 4;
const KIND: u32 = 5;

/// One suspended activation.
pub struct Activation {
    /// An async function's result promise (to tell which input it is).
    pub promise: Value,
    /// `async function todos`, `generator g`, ...
    pub what: String,
    /// `repl:3:22`, where it stopped.
    pub at: String,
    /// The named bindings and their values, printed.
    pub bindings: Vec<(String, String)>,
}

/// Reads the activation of a suspended Generator object, `None` if it is not suspended.
pub fn activation(heap: &Heap, g: Obj) -> Option<Activation> {
    if heap.object_class(g) != ObjectClass::Generator {
        return None;
    }
    let a = heap.arena();
    let frame = heap.object_slot(g, FRAME).as_internal()?;
    let pc = heap.object_slot(g, PC).as_int().unwrap_or(0) as u32;
    let hp = heap.object_slot(g, HP).as_int().unwrap_or(0) as u32;
    let slot = |i: u32| a.read::<Value>(ValueArray::VALUES.at(frame, i));
    let len: u32 = a.get(frame, ValueArray::LEN);
    let callee = slot(0).as_object()?;
    if heap.object_class(callee) != ObjectClass::Function {
        return None;
    }
    let t = heap.closure_template(callee);
    let info = debug_info(heap, t);
    let name = Atom::from_id(a.get(t, FuncTemplate::NAME));
    let flags = u32::from(a.get::<u16>(t, FuncTemplate::FLAGS));
    use djs_core::bytecode::template::func_flags;
    let body = matches!(
        func_flags::kind(flags),
        func_flags::KIND_SCRIPT | func_flags::KIND_ASYNC_BODY
    );
    let name = if body {
        String::from("<eval>")
    } else if name.is_null() {
        String::from("<anonymous>")
    } else {
        heap.atom_name(name)
    };
    let kind = match heap.object_slot(g, KIND).as_int() {
        Some(0) => "generator",
        Some(1) if body => "async",
        Some(1) => "async function",
        Some(2) => "async generator",
        _ => "?",
    };
    let state = match heap.object_slot(g, STATE).as_int() {
        Some(0) => " (not started)",
        _ => "",
    };
    let file = info
        .script
        .get()
        .and_then(|s| {
            a.get::<djs_core::heap::Ref>(s, djs_core::heap::cells::Script::NAME)
                .get()
        })
        .and_then(|n| ref_value(djs_core::value::tag::STRING, n)?.as_string())
        .map(|n| heap.str_to_string(n))
        .unwrap_or_else(|| String::from("?"));
    let at = match djs_core::bytecode::lookup_line(bytes_of(heap, info.lines), pc.saturating_sub(1))
    {
        Some((l, c)) => format!("{file}:{l}:{c}"),
        None => file,
    };
    let names: Vec<String> = info
        .names
        .get()
        .map(|n| {
            let count: u32 = a.get(n, U32Array::LEN);
            (0..count)
                .map(|i| heap.atom_name(Atom::from_id(a.read::<u32>(U32Array::DATA.at(n, i)))))
                .collect()
        })
        .unwrap_or_default();
    let locals = u32::from(a.get::<u16>(t, FuncTemplate::LOCALS));
    let mut bindings = Vec::new();
    // Parameters are arguments (slots 2..), named from the function's source text.
    let source = info.script.get().and_then(|s| {
        let src = a
            .get::<djs_core::heap::Ref>(s, djs_core::heap::cells::Script::SOURCE)
            .get()?;
        let text = heap.str_to_string(ref_value(djs_core::value::tag::STRING, src)?.as_string()?);
        text.get(info.src_start as usize..info.src_end as usize)
            .map(String::from)
    });
    let params = u32::from(a.get::<u16>(t, FuncTemplate::PARAMS));
    let names_of_params = source.as_deref().map(param_names).unwrap_or_default();
    // A parameter copied into a local (a non-simple parameter list) shows once, as the
    // local; destructured ones show as the names they bind.
    let local_names = &names[..(locals as usize).min(names.len())];
    for i in 0..params.min(hp.saturating_sub(2)) {
        if let Some(Some(n)) = names_of_params.get(i as usize)
            && !local_names.contains(n)
        {
            bindings.push((n.clone(), short(heap, slot(2 + i))));
        }
    }
    for j in 0..locals {
        let at = hp + HEADER + j;
        let Some(n) = names.get(j as usize) else {
            break;
        };
        if at >= len || n.is_empty() || n.starts_with('<') {
            continue;
        }
        let mut v = slot(at);
        // A captured variable lives in a box.
        if let Some(b) = v.as_internal()
            && heap.cell_kind(b) == CellKind::BoxCell
        {
            v = a.get(b, BoxCell::VALUE);
        }
        bindings.push((n.clone(), short(heap, v)));
    }
    Some(Activation {
        promise: heap.object_slot(g, PROMISE),
        what: format!("{kind} {name}{state}"),
        at,
        bindings,
    })
}

/// The parameter names of a function's source text: `None` for a destructured one.
fn param_names(src: &str) -> Vec<Option<String>> {
    use djs_core::lexer::{Lexer, TokenKind};
    let Ok(mut lx) = Lexer::new(src.as_bytes()) else {
        return Vec::new();
    };
    let mut toks = Vec::new();
    while let Ok(t) = lx.next_token() {
        if t.kind == TokenKind::Eof || toks.len() > 4096 {
            break;
        }
        toks.push(t);
        // Enough to see the parameter list.
        if t.kind == TokenKind::Arrow || t.kind == TokenKind::LBrace && toks.len() > 1 {
            break;
        }
    }
    let text = |i: usize| &src[toks[i].start as usize..toks[i].end as usize];
    // `x => ...`, `async x => ...`
    if let Some(a) = toks.iter().position(|t| t.kind == TokenKind::Arrow)
        && a > 0
        && toks[a - 1].kind == TokenKind::Ident
        && !toks[..a].iter().any(|t| t.kind == TokenKind::LParen)
    {
        return vec![Some(text(a - 1).to_string())];
    }
    let Some(open) = toks.iter().position(|t| t.kind == TokenKind::LParen) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = true;
    let mut current: Option<Option<String>> = None;
    for (i, t) in toks.iter().enumerate().skip(open + 1) {
        let k = t.kind;
        match k {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen if depth == 0 => {
                out.extend(current.take());
                return out;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
            _ => {}
        }
        if depth == 0 && k == TokenKind::Comma {
            out.push(current.take().unwrap_or(None));
            start = true;
            continue;
        }
        if start && k != TokenKind::Ellipsis {
            current = Some((k == TokenKind::Ident).then(|| text(i).to_string()));
            start = false;
        }
    }
    out
}

/// A value printed on at most about 60 columns.
fn short(heap: &Heap, v: Value) -> String {
    let s = inspect::inspect(heap, v).replace('\n', " ");
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > 60 {
        format!("{}…", s.chars().take(59).collect::<String>())
    } else {
        s
    }
}

/// Every suspended activation in the heap (collect first: dead ones are not skipped).
pub fn all(heap: &Heap) -> Vec<Activation> {
    let mut out = Vec::new();
    for c in heap.cells() {
        if c.kind != CellKind::Object {
            continue;
        }
        let Some(o) = obj_at(c.offset) else {
            continue;
        };
        if let Some(act) = activation(heap, o) {
            out.push(act);
        }
    }
    out
}

/// The object at an arena offset (djs-core makes `Obj`s only from values).
fn obj_at(off: u32) -> Option<Obj> {
    ref_value(djs_core::value::tag::OBJECT, off)?.as_object()
}

fn ref_value(tag: u16, off: u32) -> Option<Value> {
    Value::from_bits(djs_core::value::TAG_PREFIX | (u64::from(tag) << 32) | u64::from(off))
}

/// The promise of the pending host call `call_id`.
pub fn call_promise(heap: &Heap, call_id: u32) -> Option<Obj> {
    let a = heap.arena();
    heap.cells()
        .filter(|c| c.kind == CellKind::HostCall)
        .find(|c| a.get::<u32>(c.offset, HostCall::ID) == call_id)
        .and_then(|c| a.get::<Value>(c.offset, HostCall::PROMISE).as_object())
}

/// The activations waiting on `p`, directly or through other activations and promise
/// combinators (an element of `Promise.all`, a `then`), innermost first.
pub fn waiting_on(heap: &Heap, p: Obj) -> Vec<Obj> {
    let a = heap.arena();
    let mut out = Vec::new();
    let mut seen = vec![p];
    let mut queue = vec![p];
    let push = |v: Value, seen: &mut Vec<Obj>, queue: &mut Vec<Obj>| {
        if let Some(o) = v.as_object()
            && heap.object_class(o) == ObjectClass::Promise
            && !seen.contains(&o)
        {
            seen.push(o);
            queue.push(o);
        }
    };
    let mut i = 0;
    while i < queue.len() {
        let p = queue[i];
        i += 1;
        let Some(r) = heap.object_slot(p, 2).as_internal() else {
            continue;
        };
        let n: u32 = a.get(r, ValueArray::LEN);
        for k in (0..n).step_by(3) {
            let get = |j: u32| a.read::<Value>(ValueArray::VALUES.at(r, k + j));
            let (cap, on_f) = (get(0), get(1));
            if let Some(g) = on_f.as_object() {
                match heap.object_class(g) {
                    ObjectClass::Generator => {
                        if !out.contains(&g) {
                            out.push(g);
                        }
                        push(heap.object_slot(g, PROMISE), &mut seen, &mut queue);
                    }
                    // A combinator's element function: its data record holds the
                    // combined promise's capability.
                    ObjectClass::BuiltinClosure => {
                        let data = heap.object_slot(g, 1);
                        push(data, &mut seen, &mut queue);
                        if let Some(rec) = data.as_object()
                            && heap.object_class(rec) == ObjectClass::Record
                        {
                            for s in 0..heap.object_slot_count(rec) {
                                let v = heap.object_slot(rec, s);
                                push(v, &mut seen, &mut queue);
                                push(capability_promise(heap, v), &mut seen, &mut queue);
                            }
                        }
                    }
                    _ => {}
                }
            }
            push(cap, &mut seen, &mut queue);
            push(capability_promise(heap, cap), &mut seen, &mut queue);
        }
    }
    out
}

/// The promise of a capability Record `[promise, resolve, reject]`.
fn capability_promise(heap: &Heap, v: Value) -> Value {
    match v.as_object() {
        Some(o) if heap.object_class(o) == ObjectClass::Record && heap.object_slot_count(o) > 0 => {
            heap.object_slot(o, 0)
        }
        _ => Value::UNDEFINED,
    }
}
