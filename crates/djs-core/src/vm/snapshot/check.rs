//! Import validation (D3702): an imported heap is checked before anything runs, so that
//! malformed snapshot bytes refuse instead of trapping or corrupting the VM later. The
//! bytes are trusted host input (the host wrote them); the checks are for robustness, not
//! a security boundary.
//!
//! Three layers:
//! - the decoder: every count, index and tag is bounds-checked, numbers stay numbers;
//! - [`object`]: each object's internal slots against what its class's code relies on
//!   (kinds of the referenced cells, ints in range), suspended frames against their code
//!   (the pc on an instruction, the registers within the frame, the operand count the
//!   resume point expects), closures' captures against their template;
//! - [`Heap::verify`] over the whole heap: every cell's layout, every reference's target
//!   kind, prototype chains.

use alloc::format;
use alloc::string::String;

use crate::bytecode::{decode, handlers::HandlerTable, stack, template};
use crate::heap::cells::{FuncTemplate, ValueArray};
use crate::heap::{CellKind, Heap, ObjectClass};
use crate::interp::frame;
use crate::value::{Obj, Value};

type Check = Result<(), String>;

fn int(v: Value, lo: i32, hi: i32, what: &str) -> Check {
    match v.as_int() {
        Some(i) if (lo..=hi).contains(&i) => Ok(()),
        _ => Err(format!("{what} is not an int in {lo}..={hi}")),
    }
}

fn internal_of(h: &Heap, v: Value, k: CellKind) -> Option<u32> {
    v.as_internal().filter(|&c| h.cell_kind(c) == k)
}

fn object_of(h: &Heap, v: Value, c: ObjectClass) -> Option<Obj> {
    v.as_object().filter(|&o| h.object_class(o) == c)
}

/// The checks of one object of class `c` (cells are allocated and filled; every reference
/// points at the start of a cell of this heap).
pub(super) fn object(h: &Heap, o: Obj) -> Check {
    let c = h.object_class(o);
    let n = h.object_slot_count(o);
    if n < c.slots() {
        return Err(format!("{} slots, the class has {}", n, c.slots()));
    }
    let s = |name: &str| h.object_slot(o, c.slot(name));
    // Header flags only where they mean something: the lazy `length` and `name` are a
    // closure's or a builtin closure's (read from their template or kind), `hostAsync` a
    // host function's, `builtin` a prelude function's (never above the base).
    {
        use crate::heap::cells::flags::*;
        let f = h.snap_obj_flags(o);
        let lazy = matches!(c, ObjectClass::Function | ObjectClass::BuiltinClosure);
        if (f & (OBJ_LAZY_LENGTH | OBJ_LAZY_NAME) != 0 && !lazy)
            || (f & OBJ_HOST_ASYNC != 0 && c != ObjectClass::HostFunction)
            || f & (OBJ_BUILTIN | OBJ_BUILTIN_CTOR) != 0
            || (f & OBJ_LENGTH_READONLY != 0 && c != ObjectClass::Array)
        {
            return Err(format!("header flags {f:#x} on a {c:?}"));
        }
        // A lazy name is the template's name, which must be a string (or none).
        if f & OBJ_LAZY_NAME != 0 && c == ObjectClass::Function {
            let t = h.closure_template(o);
            if t == 0 || h.cell_kind(t) != CellKind::FuncTemplate {
                return Err(String::from("a closure without a template"));
            }
            let name = crate::heap::Atom::from_id(h.arena().get(t, FuncTemplate::NAME));
            if !name.is_null() && h.atom_string_of(name).is_none() {
                return Err(String::from("a lazy name that is not a string"));
            }
        }
    }
    let el = h.snap_elements(o);
    if el != 0
        && !matches!(
            c,
            ObjectClass::Array | ObjectClass::Arguments | ObjectClass::Function
        )
    {
        return Err(String::from("dense elements on an object that has none"));
    }
    match c {
        ObjectClass::Array => {
            let len = s("length").as_number().unwrap_or(-1.0);
            let dense = if el == 0 { 0 } else { h.value_array_len(el) };
            if !(len >= dense as f64 && len <= f64::from(u32::MAX) && len == (len as u32) as f64) {
                return Err(String::from("an array length below its elements"));
            }
        }
        ObjectClass::Function => {
            let t = h.closure_template(o);
            if t == 0 || h.cell_kind(t) != CellKind::FuncTemplate {
                return Err(String::from("a closure without a template"));
            }
            let a = h.arena();
            let caps = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
            let home =
                u32::from(a.get::<u16>(t, FuncTemplate::FLAGS)) & template::func_flags::HOME != 0;
            if n != caps + u32::from(home) {
                return Err(format!(
                    "a closure with {n} slots, its template needs {caps}"
                ));
            }
            for i in 0..caps {
                if internal_of(h, h.object_slot(o, i), CellKind::BoxCell).is_none() {
                    return Err(format!("capture {i} is not a Box"));
                }
            }
        }
        ObjectClass::HostFunction => int(s("id"), 0, i32::MAX, "the host function id")?,
        ObjectClass::ArrayIterator => {
            int(s("kind"), 0, 3, "the kind")?;
            if !s("index").is_number() {
                return Err(String::from("the index is not a number"));
            }
        }
        ObjectClass::BoundFunction => {
            if !s("target").as_object().is_some_and(|t| h.is_callable(t)) {
                return Err(String::from("the target is not callable"));
            }
            if object_of(h, s("args"), ObjectClass::Array).is_none() {
                return Err(String::from("the arguments are not an array"));
            }
        }
        ObjectClass::Map | ObjectClass::Set | ObjectClass::WeakMap | ObjectClass::WeakSet => {
            let weak = matches!(c, ObjectClass::WeakMap | ObjectClass::WeakSet);
            let kind = if weak {
                CellKind::EphemeronArray
            } else {
                CellKind::ValueArray
            };
            let e = internal_of(h, s("entries"), kind).ok_or("the entries are not an array")?;
            let base = if weak { 0 } else { 2 };
            let len = h.value_array_len(e);
            if len < base || !(len - base).is_multiple_of(2) {
                return Err(String::from("the entries are not pairs"));
            }
            int(s("live"), 0, i32::MAX, "the live count")?;
            if !weak {
                let fwd = h.value_array_get(e, 0);
                if !fwd.is_undefined() && internal_of(h, fwd, CellKind::ValueArray).is_none() {
                    return Err(String::from("the forward entries are not an array"));
                }
                let removed = h.value_array_get(e, 1);
                if !removed.is_undefined() && internal_of(h, removed, CellKind::U32Array).is_none()
                {
                    return Err(String::from("the removed entries are not a list"));
                }
            }
        }
        ObjectClass::CollectionIterator => {
            let e = s("entries");
            if !e.is_undefined() && internal_of(h, e, CellKind::ValueArray).is_none() {
                return Err(String::from("the entries are not an array"));
            }
            int(s("next"), 0, i32::MAX, "the next entry")?;
            int(s("kind"), 0, 2, "the kind")?;
            int(s("isSet"), 0, 1, "isSet")?;
        }
        ObjectClass::Promise => {
            int(s("state"), 0, 2, "the state")?;
            int(s("flags"), 0, i32::MAX, "the flags")?;
            let r = s("reactions");
            if !r.is_undefined() {
                let a = internal_of(h, r, CellKind::ValueArray)
                    .ok_or("the reactions are not an array")?;
                if !h.value_array_len(a).is_multiple_of(3) {
                    return Err(String::from("the reactions are not triples"));
                }
            }
        }
        ObjectClass::BuiltinClosure => {
            let id = s("builtin").as_int().unwrap_or(-1);
            if id < 0 || crate::interp::realm::installed_intrinsic(h, id as u16).is_undefined() {
                return Err(String::from("not an installed builtin closure kind"));
            }
        }
        ObjectClass::Date => {
            if !s("time").is_number() {
                return Err(String::from("the time is not a number"));
            }
        }
        ObjectClass::Generator => generator(h, o)?,
        ObjectClass::RegExp => {
            if s("source").as_string().is_none() {
                return Err(String::from("the source is not a string"));
            }
            int(s("flags"), 0, 0xFF, "the flags")?;
        }
        ObjectClass::RegExpStringIterator => {
            if s("regexp").as_object().is_none() {
                return Err(String::from("not a regexp"));
            }
            if s("string").as_string().is_none() {
                return Err(String::from("the string is not a string"));
            }
            int(s("flags"), 0, 7, "the flags")?;
        }
        ObjectClass::Proxy => {
            for k in ["target", "handler"] {
                let v = s(k);
                if !v.is_null() && v.as_object().is_none() {
                    return Err(format!("the {k} is not an object"));
                }
            }
            int(s("flags"), 0, 3, "the flags")?;
        }
        ObjectClass::Boolean => {
            s("primitive").as_bool().ok_or("not a boolean")?;
        }
        ObjectClass::Number => {
            if !s("primitive").is_number() {
                return Err(String::from("not a number"));
            }
        }
        ObjectClass::String => {
            s("primitive").as_string().ok_or("not a string")?;
        }
        ObjectClass::Symbol => {
            s("primitive").as_symbol().ok_or("not a symbol")?;
        }
        ObjectClass::BigInt => {
            s("primitive").as_bigint().ok_or("not a BigInt")?;
        }
        ObjectClass::ArrayBuffer => {
            let b = s("bytes");
            if !b.is_undefined() && internal_of(h, b, CellKind::Bytes).is_none() {
                return Err(String::from("the bytes are not bytes"));
            }
        }
        ObjectClass::TypedArray | ObjectClass::DataView => {
            if object_of(h, s("buffer"), ObjectClass::ArrayBuffer).is_none() {
                return Err(String::from("the buffer is not an ArrayBuffer"));
            }
            for k in ["offset", "length"] {
                if !s(k).is_number() {
                    return Err(format!("the {k} is not a number"));
                }
            }
            if c == ObjectClass::TypedArray {
                int(s("kind"), 0, 11, "the element kind")?;
            }
        }
        ObjectClass::DisposableStack => {
            int(s("state"), 0, 1, "the state")?;
            if object_of(h, s("resources"), ObjectClass::Array).is_none() {
                return Err(String::from("the resources are not an array"));
            }
        }
        ObjectClass::CallSite => {
            int(s("line"), 0, i32::MAX, "the line")?;
            int(s("column"), 0, i32::MAX, "the column")?;
        }
        ObjectClass::TextCodec => {
            s("flags").as_int().ok_or("the flags are not an int")?;
            s("state").as_int().ok_or("the state is not an int")?;
        }
        _ => {}
    }
    Ok(())
}

/// A suspended activation: its state, its frame against its code.
fn generator(h: &Heap, g: Obj) -> Check {
    use crate::interp::run::suspend::{ASYNC, ASYNC_GENERATOR, COMPLETED, EXECUTING, GENERATOR};
    let c = ObjectClass::Generator;
    let s = |name: &str| h.object_slot(g, c.slot(name));
    int(s("kind"), GENERATOR, ASYNC_GENERATOR, "the kind")?;
    int(s("state"), 0, 6, "the state")?;
    let state = s("state").as_int().unwrap_or(0);
    let p = s("promise");
    if !p.is_undefined() && object_of(h, p, ObjectClass::Promise).is_none() {
        return Err(String::from("the promise is not a promise"));
    }
    if s("kind").as_int() == Some(ASYNC) && p.is_undefined() {
        return Err(String::from("an async activation without its promise"));
    }
    let q = s("queue");
    if !q.is_undefined() && internal_of(h, q, CellKind::ValueArray).is_none() {
        return Err(String::from("the queue is not an array"));
    }
    let fv = s("frame");
    if state == COMPLETED || state == EXECUTING {
        // Executing: only between a resume and the next suspension, never at an idle point.
        if state == EXECUTING || !fv.is_undefined() {
            return Err(String::from(
                "a completed or running activation with a frame",
            ));
        }
        return Ok(());
    }
    let f = internal_of(h, fv, CellKind::ValueArray).ok_or("no frame")?;
    let len = h.value_array_len(f);
    let callee = h.value_array_get(f, 0);
    let callee =
        object_of(h, callee, ObjectClass::Function).ok_or("the callee is not a closure")?;
    let t = h.closure_template(callee);
    let a = h.arena();
    let params = u32::from(a.get::<u16>(t, FuncTemplate::PARAMS));
    let locals = u32::from(a.get::<u16>(t, FuncTemplate::LOCALS));
    let max_stack = u32::from(a.get::<u16>(t, FuncTemplate::STACK));
    let hp = s("hp")
        .as_int()
        .filter(|&x| x >= 0)
        .ok_or("hp is not an int")? as u32;
    let sb = hp + frame::HEADER + locals;
    if hp < 2 + params || sb > len || len - sb > max_stack {
        return Err(format!(
            "a frame of {len} slots does not fit its function (hp {hp}, {params} params, {locals} locals, stack {max_stack})"
        ));
    }
    let argc = h
        .value_array_get(f, hp + frame::ARGC)
        .as_int()
        .unwrap_or(-1);
    if argc < 0 || argc as u32 > hp - 2 {
        return Err(String::from("the frame's argument count does not fit"));
    }
    for k in [frame::PREV_BP, frame::PREV_HP, frame::RET_PC, frame::FLAGS] {
        if h.value_array_get(f, hp + k).as_int().is_none() {
            return Err(String::from("a frame header slot is not an int"));
        }
    }
    let flags = h
        .value_array_get(f, hp + frame::FLAGS)
        .as_int()
        .unwrap_or(0);
    if flags & frame::FLAG_NATIVE != 0 {
        return Err(String::from("a native frame"));
    }
    // The pc on an instruction of reachable code, where the operand stack holds what the
    // resume pushes on top of the saved values (one value, or a yield's two).
    let pc = s("pc")
        .as_int()
        .filter(|&x| x >= 0)
        .ok_or("the pc is not an int")? as u32;
    let code = template::code(h, t);
    let handlers = HandlerTable::new(template::handlers(h, t)).map_err(|_| "bad handlers")?;
    let hs: alloc::vec::Vec<_> = handlers.iter().collect();
    let depths =
        stack::stack_depths(code, &hs).map_err(|_| "the function's code does not check")?;
    let depth = *depths.get(pc as usize).ok_or("the pc is past the code")?;
    if depth == stack::UNKNOWN_DEPTH || decode::decode(code, pc).is_err() {
        return Err(format!("the pc {pc} is not on a reachable instruction"));
    }
    let saved = len - sb;
    if !(saved..=saved + 2).contains(&depth) {
        return Err(format!(
            "the frame holds {saved} operands, the code expects {depth} at pc {pc}"
        ));
    }
    let _ = ValueArray::LEN;
    Ok(())
}
