//! Function leaf intrinsics: bound functions and `Function.prototype.toString`.

use alloc::string::String;

use crate::bytecode::template::func_flags;
use crate::heap::cells::flags::OBJ_BUILTIN;
use crate::heap::cells::{FuncTemplate, Script};
use crate::heap::{Heap, ObjectClass, Ref, atoms};
use crate::interp::conv::{Fault, type_error};
use crate::value::{Obj, Str, Value};

/// BoundFunctionCreate ( targetFunction, boundThis, boundArgs ): `args` is an array
/// that the bound function keeps (the caller made it). `name` and `length` are set by
/// the builtin (`Function.prototype.bind`), since reading the target's may run getters.
pub(crate) fn bound_function_create(
    heap: &mut Heap,
    target: Obj,
    this: Value,
    args: Obj,
) -> Result<Value, Fault> {
    let proto = heap.get_prototype_of(target);
    let f = heap.alloc_object(ObjectClass::BoundFunction, proto, 3)?;
    heap.set_object_slot(f, 0, Value::object(target));
    heap.set_object_slot(f, 1, this);
    heap.set_object_slot(f, 2, Value::object(args));
    Ok(Value::object(f))
}

/// The source text of a function, as `Function.prototype.toString` returns it: the
/// source slice for JS functions (D009), `function name() {\n    [native code]\n}` (as
/// qjs prints it) for builtins, host and bound functions.
pub(crate) fn to_string(heap: &mut Heap, f: Value) -> Result<Str, Fault> {
    let Some(o) = f.as_object().filter(|&o| heap.is_callable(o)) else {
        return type_error("not a function");
    };
    if heap.object_class(o) == ObjectClass::Function
        && heap.cell_flags(o.offset()) & OBJ_BUILTIN == 0
        && let Some(src) = source_slice(heap, o)
    {
        return Ok(heap.alloc_str(&src)?);
    }
    let name = heap
        .get_own_property(o, atoms::NAME)
        .and_then(|p| p.value())
        .and_then(|v| v.as_string())
        .map(|s| heap.str_to_string(s))
        .unwrap_or_default();
    Ok(heap.alloc_str(&crate::text::cat!(
        "function ",
        name,
        "() {\n    [native code]\n}"
    ))?)
}

fn source_slice(heap: &Heap, f: Obj) -> Option<String> {
    let t = heap.object_slot(f, 0).as_internal()?;
    let a = heap.arena();
    let debug = crate::bytecode::template::debug_info(heap, t);
    let source: Ref = a.get(debug.script.get()?, Script::SOURCE);
    let (start, end) = (debug.src_start as usize, debug.src_end as usize);
    let kind = func_flags::kind(a.get(t, FuncTemplate::FLAGS));
    if matches!(kind, func_flags::KIND_SCRIPT | func_flags::KIND_ASYNC_BODY) {
        return None;
    }
    let text = heap.str_to_string(Str(source.get()?));
    text.get(start..end).map(String::from)
}
