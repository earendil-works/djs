//! JSON leaf intrinsics over `crate::json` (D1331): the parse without a reviver, the
//! stringify fast path, and QuoteJSONString. builtins/json.ts does the rest.

use alloc::string::String;

use crate::heap::{Heap, HeapError};
use crate::interp::conv::Fault;
use crate::interp::{ErrorKind, Intrinsic, realm};
use crate::json::{self, ParseError, ParseOptions, StringifyError};
use crate::value::{Str, Value};

/// `%JsonParse(text)`: the parsed value, or the SyntaxError (qjs's message and
/// position) for the caller to throw.
pub(crate) fn parse(heap: &mut Heap, text: Str) -> Result<Result<Value, json::SyntaxError>, Fault> {
    let opts = ParseOptions {
        object_proto: Some(realm::intrinsic_obj(heap, Intrinsic::ObjectProto)),
        array_proto: Some(realm::intrinsic_obj(heap, Intrinsic::ArrayProto)),
        ..Default::default()
    };
    match json::parse(heap, text, &opts) {
        Ok(v) => Ok(Ok(v)),
        Err(ParseError::Syntax(e)) => Ok(Err(e)),
        Err(ParseError::Heap(e)) => Err(Fault::heap(e)),
    }
}

/// `%JsonStringifyFast(value, gap)`: the result string, `undefined` for an undefined
/// result, or `null` when the value needs the full algorithm (nothing observable has
/// happened then).
pub(crate) fn stringify_fast(heap: &mut Heap, v: Value, gap: Str) -> Result<Value, Fault> {
    let gap = heap.str_to_utf16(gap);
    match json::stringify_fast(heap, v, &gap) {
        Ok(Some(s)) => Ok(Value::string(s)),
        Ok(None) => Ok(Value::UNDEFINED),
        // The full algorithm throws for a BigInt (after a replacer had its say).
        Err(StringifyError::Slow | StringifyError::BigInt) => Ok(Value::NULL),
        Err(StringifyError::Heap(e)) => Err(Fault::heap(e)),
        Err(StringifyError::Circular) => Err(Fault::error(
            ErrorKind::TypeError,
            String::from("circular reference"),
        )),
        Err(StringifyError::StackOverflow) => Err(Fault::error(
            ErrorKind::RangeError,
            String::from(crate::interp::STACK_OVERFLOW),
        )),
    }
}

/// `%JsonQuote(s)`: QuoteJSONString ( value ), 25.5.2.3.
pub(crate) fn quote(heap: &mut Heap, s: Str) -> Result<Str, Fault> {
    // A string is plain data: the fast path always handles it.
    match json::stringify_fast(heap, Value::string(s), &[]) {
        Ok(Some(q)) => Ok(q),
        Err(StringifyError::Heap(e)) => Err(Fault::heap(e)),
        _ => Err(Fault::heap(HeapError::OutOfMemory)),
    }
}
