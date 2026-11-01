//! Dynamic code (0059, D014, D1590): the leaves that compile source text at run time for
//! indirect `eval` and the four dynamic function constructors. They compile only. The
//! builtins (builtins/eval.ts) call the result, so running the code is an ordinary JS
//! call: fuel, suspension and snapshots work as for any other code.

use alloc::vec::Vec;

use crate::compiler::{self, CompileErrorKind, CompileOptions, DynamicKind};
use crate::heap::{Heap, HeapError, ObjectClass};
use crate::interp::conv::Fault;
use crate::interp::{ErrorKind, Intrinsic, realm};
use crate::value::{Str, Value};

/// The file name of dynamic code in stack traces, as qjs names it.
const FILE: &str = "<input>";

/// The source as UTF-8 for the compiler. Lone surrogates (which UTF-8 cannot hold)
/// become U+FFFD.
fn utf8(heap: &Heap, s: Str) -> Vec<u8> {
    heap.str_to_string(s).into_bytes()
}

fn compile_error(e: compiler::CompileError) -> Fault {
    match e.kind {
        CompileErrorKind::Syntax => Fault::Error(ErrorKind::SyntaxError, e.message),
        CompileErrorKind::OutOfMemory => Fault::Heap(HeapError::OutOfMemory),
    }
}

/// A closure for a top-level template (as `Vm::instantiate`).
fn instantiate(heap: &mut Heap, template: crate::heap::layout::Ref) -> Result<Value, Fault> {
    let proto = realm::intrinsic_obj(heap, Intrinsic::FunctionProto);
    let f = heap.alloc_object(ObjectClass::Function, Some(proto), 1)?;
    heap.set_object_slot(f, 0, Value::internal(template.0));
    Ok(Value::object(f))
}

/// PerformEval's parse (19.2.1.1) for indirect eval: the source as a Script, returned as
/// a closure that the builtin calls with `this` = the global object.
// TODO(0058): eval code is a Script for now. With 0058's completion values and an eval
// goal, this compiles with that goal instead, so that the call returns the completion
// value and top-level declarations stay local to the call (strict eval, 19.2.1.1 steps
// 16-17). The builtin needs no change.
pub(crate) fn compile_eval(heap: &mut Heap, src: Str) -> Result<Value, Fault> {
    let bytes = utf8(heap, src);
    let template =
        compiler::compile(heap, &bytes, &CompileOptions::script(FILE)).map_err(compile_error)?;
    instantiate(heap, template)
}

/// CreateDynamicFunction's parse (20.2.1.1.1, steps 6-17): builds the sourceText from the
/// parameter text (already joined with ",") and the body, and compiles it as a Script
/// whose call returns the function.
pub(crate) fn compile_function(
    heap: &mut Heap,
    kind: DynamicKind,
    params: Str,
    body: Str,
) -> Result<Value, Fault> {
    let mut src = Vec::new();
    src.extend_from_slice(kind.prefix().as_bytes());
    src.extend_from_slice(b" anonymous(");
    src.extend_from_slice(&utf8(heap, params));
    let params_close = src.len() as u32 + 1;
    src.extend_from_slice(b"\n) {\n");
    src.extend_from_slice(&utf8(heap, body));
    src.extend_from_slice(b"\n}");
    let template = compiler::compile_dynamic_function(
        heap,
        &src,
        &CompileOptions::script(FILE),
        kind,
        params_close,
    )
    .map_err(compile_error)?;
    instantiate(heap, template)
}
