//! `call_intrinsic id, argc` (docs/compiler.md 7.3, D1270): the leaf and reentrant
//! intrinsics of `builtins/intrinsics.d.ts` in Rust, and calls of the JS builtins that
//! the prelude registered with `%InstallIntrinsic`.
//!
//! Arguments are on the stack (`a1 .. an`, like `call` without `f` and `this`); the
//! intrinsic replaces them with its result. Reentrant ones (D1173) finish in Rust or
//! push exactly one call frame whose result completes them.

use super::*;
use crate::builtins::{
    array, collection, date, eval, function, number, object, string, typed, uri,
};
use crate::compiler::DynamicKind;

/// Fuel per code unit of compiled source (eval, the Function constructors): compiling
/// one unit costs about as much as a few simple instructions (D1591).
const COMPILE_FUEL_PER_UNIT: i64 = 4;
use crate::bytecode::intrinsics::{INTRINSICS, IntrinsicKind, intrinsic, intrinsic_id};
use crate::heap::cells::flags::{OBJ_BUILTIN, OBJ_BUILTIN_CTOR};
use crate::heap::{ObjectClass, atoms};

/// The id of `name`, at compile time (a typo fails the build).
pub(super) const fn id(name: &str) -> u16 {
    let mut k = 0;
    while k < INTRINSICS.len() {
        let a = INTRINSICS[k].name.as_bytes();
        let b = name.as_bytes();
        if a.len() == b.len() {
            let mut j = 0;
            while j < a.len() && a[j] == b[j] {
                j += 1;
            }
            if j == a.len() {
                return k as u16;
            }
        }
        k += 1;
    }
    panic!("unknown intrinsic")
}

const IS_OBJECT: u16 = id("IsObject");
const IS_CALLABLE: u16 = id("IsCallable");
const IS_CONSTRUCTOR: u16 = id("IsConstructor");
const IS_ARRAY: u16 = id("IsArray");
const IS_STRING: u16 = id("IsString");
const IS_NUMBER: u16 = id("IsNumber");
const IS_SYMBOL: u16 = id("IsSymbol");
const IS_BOOLEAN: u16 = id("IsBoolean");
const AS_OBJ: u16 = id("AsObj");
const AS_FN: u16 = id("AsFn");
const AS_STR: u16 = id("AsStr");
const AS_NUMBER: u16 = id("AsNumber");
const AS_SYM: u16 = id("AsSym");
const AS_BOOL: u16 = id("AsBool");
const TO_BOOLEAN: u16 = id("ToBoolean");
const TO_NUMBER_PRIMITIVE: u16 = id("ToNumberPrimitive");
const TO_STRING_PRIMITIVE: u16 = id("ToStringPrimitive");
const TO_OBJECT: u16 = id("ToObject");
const TO_INTEGER_OR_INFINITY_NUMBER: u16 = id("ToIntegerOrInfinityNumber");
const F64_TO_U32: u16 = id("F64ToU32");
const F64_TO_I32: u16 = id("F64ToI32");
const STR_LENGTH: u16 = id("StrLength");
const GET: u16 = id("Get");
const GET_V: u16 = id("GetV");
const SET: u16 = id("Set");
const HAS_PROPERTY: u16 = id("HasProperty");
const CREATE_DATA_PROPERTY_OR_THROW: u16 = id("CreateDataPropertyOrThrow");
const DELETE_PROPERTY_OR_THROW: u16 = id("DeletePropertyOrThrow");
const CALL: u16 = id("Call");
const CONSTRUCT: u16 = id("Construct");
const OBJECT_CREATE: u16 = id("ObjectCreate");
const ARRAY_CREATE: u16 = id("ArrayCreate");
const NEW_TYPE_ERROR: u16 = id("NewTypeError");
const NEW_NOT_CONSTRUCTOR_ERROR: u16 = id("NewNotConstructorError");
const NEW_RANGE_ERROR: u16 = id("NewRangeError");
const NEW_REFERENCE_ERROR: u16 = id("NewReferenceError");
const NEW_SYNTAX_ERROR: u16 = id("NewSyntaxError");
const WELL_KNOWN_SYMBOL: u16 = id("WellKnownSymbol");
const REALM_INTRINSIC: u16 = id("RealmIntrinsic");
const ERROR_CREATE: u16 = id("ErrorCreate");
const CAPTURE_STACK: u16 = id("CaptureStack");
const DEFINE_DATA_PROPERTY_OR_THROW: u16 = id("DefineDataPropertyOrThrow");
const DEFINE_OWN_PROPERTY_FROM_RECORD: u16 = id("DefineOwnPropertyFromRecord");
const GET_OWN_PROPERTY_DESCRIPTOR_OBJECT: u16 = id("GetOwnPropertyDescriptorObject");
const HAS_OWN_PROPERTY: u16 = id("HasOwnProperty");
const OWN_PROPERTY_ATTRIBUTES: u16 = id("OwnPropertyAttributes");
const OWN_PROPERTY_KEYS: u16 = id("OwnPropertyKeys");
const CALL_WITH_ARRAY: u16 = id("CallWithArray");
const ARRAY_BUFFER_CREATE: u16 = id("ArrayBufferCreate");
const BUFFER_INFO: u16 = id("BufferInfo");
const BUFFER_DETACH: u16 = id("BufferDetach");
const BUFFER_RESIZE: u16 = id("BufferResize");
const BUFFER_TRANSFER: u16 = id("BufferTransfer");
const BUFFER_COPY: u16 = id("BufferCopy");
const IS_TYPED_ARRAY: u16 = id("IsTypedArray");
const IS_DATA_VIEW: u16 = id("IsDataView");
const DATA_VIEW_CREATE: u16 = id("DataViewCreate");
const DATA_VIEW_FIELD: u16 = id("DataViewField");
const DATA_VIEW_GET: u16 = id("DataViewGet");
const DATA_VIEW_SET: u16 = id("DataViewSet");
const SORT_NUMBERS: u16 = id("SortNumbers");
const REALM_CONSTRUCTOR: u16 = id("RealmConstructor");
const TYPED_ARRAY_CREATE: u16 = id("TypedArrayCreate");
const TYPED_ARRAY_FIELD: u16 = id("TypedArrayField");
const TYPED_ARRAY_GET: u16 = id("TypedArrayGet");
const TYPED_ARRAY_SET: u16 = id("TypedArraySet");
const TYPED_ARRAY_COPY: u16 = id("TypedArrayCopy");
const TYPED_ARRAY_KIND_PROTO: u16 = id("TypedArrayKindProto");
const BASE64_ENCODE: u16 = id("Base64Encode");
const HEX_ENCODE: u16 = id("HexEncode");
const BASE64_DECODE: u16 = id("Base64Decode");
const HEX_DECODE: u16 = id("HexDecode");

const DATE_NOW: u16 = id("DateNow");
const DATE_CREATE: u16 = id("DateCreate");
const DATE_VALUE: u16 = id("DateValue");
const DATE_SET_VALUE: u16 = id("DateSetValue");
const DATE_FROM_FIELDS: u16 = id("DateFromFields");
const DATE_GET_FIELD: u16 = id("DateGetField");
const DATE_SET_FIELDS: u16 = id("DateSetFields");
const DATE_FORMAT: u16 = id("DateFormat");
const DATE_PARSE: u16 = id("DateParse");
const DATE_TIMEZONE_OFFSET: u16 = id("DateTimezoneOffset");

const COLLECTION_CREATE: u16 = id("CollectionCreate");
const COLLECTION_CLASS: u16 = id("CollectionClass");
const COLLECTION_GET: u16 = id("CollectionGet");
const COLLECTION_HAS: u16 = id("CollectionHas");
const COLLECTION_SET: u16 = id("CollectionSet");
const COLLECTION_DELETE: u16 = id("CollectionDelete");
const COLLECTION_CLEAR: u16 = id("CollectionClear");
const COLLECTION_SIZE: u16 = id("CollectionSize");
const COLLECTION_ITERATOR: u16 = id("CollectionIterator");
const COLLECTION_ITERATOR_NEXT: u16 = id("CollectionIteratorNext");
const COLLECTION_ITERATOR_STEP: u16 = id("CollectionIteratorStep");
const COLLECTION_ITERATOR_KEY: u16 = id("CollectionIteratorKey");
const COLLECTION_ITERATOR_VALUE: u16 = id("CollectionIteratorValue");
const CAN_BE_HELD_WEAKLY: u16 = id("CanBeHeldWeakly");

const CONSTRUCT_WITH_ARRAY: u16 = id("ConstructWithArray");
const GET_WITH_RECEIVER: u16 = id("GetWithReceiver");
const SET_WITH_RECEIVER: u16 = id("SetWithReceiver");
const DELETE_PROPERTY: u16 = id("DeleteProperty");
const BOUND_FUNCTION_CREATE: u16 = id("BoundFunctionCreate");
const FUNCTION_TO_STRING: u16 = id("FunctionToString");
const NUMBER_POW: u16 = id("NumberPow");
const SYMBOL_DESCRIPTIVE_STRING: u16 = id("SymbolDescriptiveString");
const GET_PROTOTYPE_OF: u16 = id("GetPrototypeOf");
const SET_PROTOTYPE_OF: u16 = id("SetPrototypeOf");
const PREVENT_EXTENSIONS: u16 = id("PreventExtensions");
const IS_EXTENSIBLE: u16 = id("IsExtensible");
const SET_INTEGRITY_LEVEL: u16 = id("SetIntegrityLevel");
const TEST_INTEGRITY_LEVEL: u16 = id("TestIntegrityLevel");
const SAME_VALUE: u16 = id("SameValue");
const OBJECT_CLASS_TAG: u16 = id("ObjectClassTag");
const WRAP_PRIMITIVE: u16 = id("WrapPrimitive");
const PRIMITIVE_VALUE: u16 = id("PrimitiveValue");
const BOUND_TARGET_FUNCTION: u16 = id("BoundTargetFunction");
const NEW_SYMBOL: u16 = id("NewSymbol");
const SYMBOL_DESCRIPTION: u16 = id("SymbolDescription");
const SYMBOL_FOR: u16 = id("SymbolFor");
const SYMBOL_KEY_FOR: u16 = id("SymbolKeyFor");
const NUMBER_TO_STRING: u16 = id("NumberToString");
const NUMBER_TO_FIXED: u16 = id("NumberToFixed");
const NUMBER_TO_EXPONENTIAL: u16 = id("NumberToExponential");
const NUMBER_TO_PRECISION: u16 = id("NumberToPrecision");
const STRING_PARSE_FLOAT: u16 = id("StringParseFloat");
const STRING_PARSE_INT: u16 = id("StringParseInt");
const URI_ENCODE: u16 = id("UriEncode");
const COMPILE_EVAL: u16 = id("CompileEval");
const COMPILE_FUNCTION: u16 = id("CompileFunction");
const GLOBAL_OBJECT: u16 = id("GlobalObject");
const ENQUEUE_MICROTASK: u16 = id("EnqueueMicrotask");
const MICROTASK_JOB: u16 = id("MicrotaskJob");
const URI_DECODE: u16 = id("UriDecode");
const STR_ESCAPE: u16 = id("StrEscape");
const STR_UNESCAPE: u16 = id("StrUnescape");
const NUMBER_TO_INT32: u16 = id("NumberToInt32");
const NUMBER_TO_UINT32: u16 = id("NumberToUint32");
const MATH_RANDOM: u16 = id("MathRandom");
const CREATE_ARRAY_ITERATOR: u16 = id("CreateArrayIterator");
const ARRAY_ITERATOR_TARGET: u16 = id("ArrayIteratorTarget");
const ARRAY_ITERATOR_INDEX: u16 = id("ArrayIteratorIndex");
const ARRAY_ITERATOR_SET_INDEX: u16 = id("ArrayIteratorSetIndex");
const ARRAY_ITERATOR_FINISH: u16 = id("ArrayIteratorFinish");
const ARRAY_ITERATOR_KIND: u16 = id("ArrayIteratorKind");
const CREATE_ITER_RESULT_OBJECT: u16 = id("CreateIterResultObject");
const CREATE_ARRAY_FROM_PAIR: u16 = id("CreateArrayFromPair");
const F64_ABS: u16 = id("F64Abs");
const F64_ACOS: u16 = id("F64Acos");
const F64_ACOSH: u16 = id("F64Acosh");
const F64_ASIN: u16 = id("F64Asin");
const F64_ASINH: u16 = id("F64Asinh");
const F64_ATAN: u16 = id("F64Atan");
const F64_ATANH: u16 = id("F64Atanh");
const F64_ATAN2: u16 = id("F64Atan2");
const F64_CBRT: u16 = id("F64Cbrt");
const F64_HYPOT: u16 = id("F64Hypot");
const F64_CEIL: u16 = id("F64Ceil");
const F64_CLZ32: u16 = id("F64Clz32");
const F64_COS: u16 = id("F64Cos");
const F64_COSH: u16 = id("F64Cosh");
const F64_EXP: u16 = id("F64Exp");
const F64_EXPM1: u16 = id("F64Expm1");
const F64_FLOOR: u16 = id("F64Floor");
const F64_FROUND: u16 = id("F64Fround");
const F64_F16ROUND: u16 = id("F64F16round");
const F64_SUM_PRECISE: u16 = id("F64SumPrecise");
const DOM_EXCEPTION_CREATE: u16 = id("DOMExceptionCreate");
const DOM_EXCEPTION_FIELD: u16 = id("DOMExceptionField");
const DOM_EXCEPTION_CODE: u16 = id("DOMExceptionCode");
const DOM_EXCEPTION_CONSTANT: u16 = id("DOMExceptionConstant");
const BTOA: u16 = id("Btoa");
const ATOB: u16 = id("Atob");
const F64_IMUL: u16 = id("F64Imul");
const F64_LOG: u16 = id("F64Log");
const F64_LOG1P: u16 = id("F64Log1p");
const F64_LOG10: u16 = id("F64Log10");
const F64_LOG2: u16 = id("F64Log2");
const F64_ROUND: u16 = id("F64Round");
const F64_SIGN: u16 = id("F64Sign");
const F64_SIN: u16 = id("F64Sin");
const F64_SINH: u16 = id("F64Sinh");
const F64_SQRT: u16 = id("F64Sqrt");
const F64_TAN: u16 = id("F64Tan");
const F64_TANH: u16 = id("F64Tanh");
const F64_TRUNC: u16 = id("F64Trunc");
const COMPARE_STRINGS: u16 = id("CompareStrings");
const STRING_CODE_POINTS: u16 = id("StringCodePoints");
const SORT_PRIMITIVES: u16 = id("SortPrimitives");
const STR_LOCALE_COMPARE: u16 = id("StrLocaleCompare");
const STR_CHAR_CODE_AT: u16 = id("StrCharCodeAt");
const STR_CODE_POINT_AT: u16 = id("StrCodePointAt");
const STR_SUBSTRING: u16 = id("StrSubstring");
const STR_INDEX_OF: u16 = id("StrIndexOf");
const STR_LAST_INDEX_OF: u16 = id("StrLastIndexOf");
const STR_TO_CASE: u16 = id("StrToCase");
const STR_TRIM: u16 = id("StrTrim");
const STR_IS_WELL_FORMED: u16 = id("StrIsWellFormed");
const STR_TO_WELL_FORMED: u16 = id("StrToWellFormed");
const STR_REPEAT: u16 = id("StrRepeat");
const STR_FROM_CODE_UNITS: u16 = id("StrFromCodeUnits");
const STR_FROM_CODE_POINTS: u16 = id("StrFromCodePoints");
const STR_NORMALIZE: u16 = id("StrNormalize");

const ARRAY_SPLICE_DENSE: u16 = id("ArraySpliceDense");
const ARRAY_CAN_POP_DENSE: u16 = id("ArrayCanPopDense");
const ARRAY_POP_DENSE: u16 = id("ArrayPopDense");
const ARRAY_SEARCH_DENSE: u16 = id("ArraySearchDense");
const ARRAY_SLICE_DENSE: u16 = id("ArraySliceDense");
const ARRAY_JOIN_DENSE: u16 = id("ArrayJoinDense");
const ITERATOR_FAST_READY: u16 = id("IteratorFastReady");
const ITERATOR_FAST_VALUE: u16 = id("IteratorFastValue");
const INSTALL_CONSTRUCTOR: u16 = id("InstallConstructor");
const INSTALL_ACCESSOR: u16 = id("InstallAccessor");
const INSTALL_VALUE: u16 = id("InstallValue");
const INSTALL_METHOD: u16 = id("InstallMethod");
const INSTALL_INTRINSIC: u16 = id("InstallIntrinsic");
const MARK_NATIVE: u16 = id("MarkNative");
/// The installed `%ToPrimitive`, used by the interpreter's slow paths.
pub(super) const TO_PRIMITIVE: u16 = id("ToPrimitive");
/// The installed `%ToNumber`, for typed array element writes (0039).
pub(super) const TO_NUMBER: u16 = id("ToNumber");
pub(super) const TO_BIG_INT: u16 = id("ToBigInt");
/// The installed `%InstanceofOperator`, for `instanceof` with a custom Symbol.hasInstance.
pub(super) const INSTANCEOF_OPERATOR: u16 = id("InstanceofOperator");
/// The installed `%ArraySpread`, for `array_spread` over a general iterable.
pub(super) const ARRAY_SPREAD: u16 = id("ArraySpread");
/// The installed `%CopyDataProperties`, for `copy_data_props` with getters or a string.
pub(super) const COPY_DATA_PROPERTIES: u16 = id("CopyDataProperties");
/// The original Array.prototype.values and %ArrayIteratorPrototype%.next: an array
/// spread skips the iterator protocol while they are in place.
pub(super) const ARRAY_PROTOTYPE_VALUES: u16 = id("ArrayPrototypeValues");
pub(super) const THROW_TYPE_ERROR: u16 = id("ThrowTypeError");
pub(super) const ARRAY_SET_LENGTH: u16 = id("ArraySetLength");
pub(super) const ARRAY_ITERATOR_NEXT: u16 = id("ArrayIteratorPrototypeNext");
pub(super) const STRING_PROTOTYPE_ITERATOR: u16 = id("StringPrototypeIterator");
pub(super) const STRING_ITERATOR_NEXT: u16 = id("StringIteratorPrototypeNext");
const CREATE_STRING_ITERATOR: u16 = id("CreateStringIterator");
const STRING_ITERATOR_TARGET: u16 = id("StringIteratorTarget");
const STR_CODE_POINT_STRING: u16 = id("StrCodePointString");
const HAS_ORIGINAL_ITERATION: u16 = id("HasOriginalIteration");
const PROMISE_CREATE: u16 = id("PromiseCreate");
const IS_PROMISE: u16 = id("IsPromise");
const INTRINSIC_PROMISE: u16 = id("IntrinsicPromise");
const PROMISE_STATE: u16 = id("PromiseState");
const PROMISE_RESULT: u16 = id("PromiseResult");
const FULFILL_PROMISE: u16 = id("FulfillPromise");
const REJECT_PROMISE: u16 = id("RejectPromise");
const PROMISE_CLAIM_RESOLVE: u16 = id("PromiseClaimResolve");
const PERFORM_PROMISE_THEN: u16 = id("PerformPromiseThen");
const ENQUEUE_PROMISE_RESOLVE_THENABLE_JOB: u16 = id("EnqueuePromiseResolveThenableJob");
const NEW_RECORD: u16 = id("NewRecord");
const RECORD_GET: u16 = id("RecordGet");
const RECORD_SET: u16 = id("RecordSet");
const NEW_BUILTIN_CLOSURE: u16 = id("NewBuiltinClosure");
const GENERATOR_STATE: u16 = id("GeneratorState");
const GENERATOR_SET_STATE: u16 = id("GeneratorSetState");
const GENERATOR_RESUME: u16 = id("GeneratorResume");
const IS_ASYNC_FUNCTION_STATE: u16 = id("IsAsyncFunctionState");
const ASYNC_FUNCTION_PROMISE: u16 = id("AsyncFunctionPromise");
const ASYNC_GENERATOR_STATE: u16 = id("AsyncGeneratorState");
const ASYNC_GENERATOR_ENQUEUE: u16 = id("AsyncGeneratorEnqueue");
const ASYNC_GENERATOR_FIRST_REQUEST: u16 = id("AsyncGeneratorFirstRequest");
const ASYNC_GENERATOR_DEQUEUE: u16 = id("AsyncGeneratorDequeue");
const CREATE_ASYNC_FROM_SYNC_ITERATOR: u16 = id("CreateAsyncFromSyncIterator");
pub(super) const PROMISE_CONSTRUCTOR: u16 = id("PromiseConstructor");
const PROMISE_RESOLVE_THENABLE_JOB: u16 = id("PromiseResolveThenableJob");

/// The classes whose slots `%RecordGet`/`%RecordSet` reach: Records, and the objects
/// whose builtins keep all their state in slots (0051). Their builtins check the brand.
fn is_slot_object(class: ObjectClass) -> bool {
    matches!(
        class,
        ObjectClass::Record
            | ObjectClass::IteratorHelper
            | ObjectClass::DisposableStack
            | ObjectClass::FinalizationRegistry
    )
}

/// The builtins a `%NewBuiltinClosure(kind, ..)` calls, by kind (D1372; the list in
/// builtins/intrinsics.d.ts).
const BUILTIN_CLOSURES: [u16; 25] = [
    id("PromiseResolveFunction"),
    id("PromiseRejectFunction"),
    id("GetCapabilitiesExecutor"),
    id("PromiseAllResolveElement"),
    id("PromiseAllSettledResolveElement"),
    id("PromiseAllSettledRejectElement"),
    id("PromiseAnyRejectElement"),
    id("PromiseThenFinally"),
    id("PromiseCatchFinally"),
    id("PromiseValueThunk"),
    id("PromiseThrower"),
    id("AsyncGeneratorAwaitReturnFulfilled"),
    id("AsyncGeneratorAwaitReturnRejected"),
    id("AsyncFromSyncIteratorUnwrap"),
    id("AsyncFromSyncIteratorCloseThrow"),
    id("ProxyRevokeFunction"),
    id("FromAsyncFulfilled"),
    id("FromAsyncRejected"),
    id("AsyncDisposeFulfilled"),
    id("AsyncDisposeRejected"),
    id("DecoratorAddInitializer"),
    id("DecoratorAccessGet"),
    id("DecoratorAccessSet"),
    id("DecoratorAccessHas"),
    id("DecoratorParamCall"),
];

/// The function-kind constructors, wired to their prototypes when installed (D1420).
const FUNCTION_KIND_CONSTRUCTORS: [(u16, Intrinsic, &str); 3] = [
    (
        id("GeneratorFunctionConstructor"),
        Intrinsic::GeneratorFunctionProto,
        "GeneratorFunction",
    ),
    (
        id("AsyncGeneratorFunctionConstructor"),
        Intrinsic::AsyncGeneratorFunctionProto,
        "AsyncGeneratorFunction",
    ),
    (
        id("AsyncFunctionConstructor"),
        Intrinsic::AsyncFunctionProto,
        "AsyncFunction",
    ),
];

/// What `ordinary_set` leaves to its caller.
enum SetStep {
    Done(bool),
    /// Call this setter with the receiver.
    Setter(Value),
    /// `%ProxySet(p, key, v, receiver)`: a proxy on the chain.
    Proxy(Obj),
    /// `%ProxySetOnReceiver(receiver, key, v)`: the receiver is a proxy.
    Receiver(Obj),
    /// `%ArraySetLength(array, v, false)`: an array's `length` set to an object.
    ArrayLength(Obj),
}

/// The libm intrinsics (D1150): `F64Sin(x)` and friends.
fn math_leaf(id: u16, x: f64, y: f64) -> Option<f64> {
    Some(match id {
        F64_ABS => libm::fabs(x),
        F64_ACOS => libm::acos(x),
        F64_ACOSH => libm::acosh(x),
        F64_ASIN => libm::asin(x),
        F64_ASINH => libm::asinh(x),
        F64_ATAN => libm::atan(x),
        F64_ATANH => libm::atanh(x),
        F64_ATAN2 => libm::atan2(x, y),
        F64_CBRT => libm::cbrt(x),
        F64_HYPOT => libm::hypot(x, y),
        F64_CEIL => libm::ceil(x),
        F64_CLZ32 => number::to_uint32(x).leading_zeros() as f64,
        F64_COS => libm::cos(x),
        F64_COSH => libm::cosh(x),
        F64_EXP => libm::exp(x),
        F64_EXPM1 => libm::expm1(x),
        F64_FLOOR => libm::floor(x),
        F64_FROUND => crate::heap::typed::f32_bits_to_f64(crate::heap::typed::f64_to_f32_bits(x)),
        F64_F16ROUND => crate::builtins::math::f16_round(x),
        F64_IMUL => (number::to_uint32(x).wrapping_mul(number::to_uint32(y)) as i32) as f64,
        F64_LOG => libm::log(x),
        F64_LOG1P => libm::log1p(x),
        F64_LOG10 => libm::log10(x),
        F64_LOG2 => libm::log2(x),
        F64_ROUND => number::round(x),
        F64_SIGN => number::sign(x),
        F64_SIN => libm::sin(x),
        F64_SINH => libm::sinh(x),
        F64_SQRT => libm::sqrt(x),
        F64_TAN => libm::tan(x),
        F64_TANH => libm::tanh(x),
        F64_TRUNC => libm::trunc(x),
        _ => return None,
    })
}

/// The Array Iterator kind of String Iterators (kinds 0-2 are keys/values/entries).
const STRING_ITERATOR_KIND: i32 = 3;

/// An array index key (a non-negative integral Number below 2^31) for the element fast
/// paths.
fn index_key(k: Value) -> Option<u32> {
    if let Some(i) = k.as_int() {
        return u32::try_from(i).ok();
    }
    k.as_number()
        .filter(|&n| (0.0..2147483648.0).contains(&n) && (n as u32) as f64 == n)
        .map(|n| n as u32)
}

pub(super) fn bad_argument<T>(name: &str) -> Result<T, Fault> {
    Err(Fault::error(
        ErrorKind::InternalError,
        format!("bad argument to %{name}"),
    ))
}

impl Exec<'_> {
    /// Executes `call_intrinsic id, argc`; `next` is the following instruction.
    pub(super) fn call_intrinsic(&mut self, id: u16, argc: u32, next: u32) -> Step {
        let Some(info) = intrinsic(id) else {
            return Err(Fault::error(
                ErrorKind::InternalError,
                format!("bad bytecode: intrinsic {id}"),
            ));
        };
        let base = self.sp - argc;
        let arg = |ex: &Self, i: u32| {
            if i < argc {
                ex.get(base + i)
            } else {
                Value::UNDEFINED
            }
        };
        if info.kind == IntrinsicKind::Builtin {
            if let Some(v) = self.iter_intrinsic_fast(id, base, argc)? {
                self.sp = base;
                self.push(v);
                self.pc = next;
                return Ok(None);
            }
            return self.call_installed(id, base, argc, next);
        }
        if let Some(step) = self.proxy_intrinsic(id, base, argc, next) {
            return step;
        }
        let a0 = arg(self, 0);
        let v = match id {
            IS_OBJECT => Value::bool(a0.as_object().is_some()),
            IS_CALLABLE => Value::bool(self.is_callable(a0)),
            IS_CONSTRUCTOR => Value::bool(a0.as_object().is_some_and(|o| self.is_constructor(o))),
            IS_ARRAY => match a0.as_object().map(|o| self.heap.is_array(o)) {
                None => Value::FALSE,
                Some(Some(b)) => Value::bool(b),
                Some(None) => return type_error("revoked proxy"),
            },
            IS_STRING => Value::bool(a0.as_string().is_some()),
            IS_NUMBER => Value::bool(a0.is_number()),
            IS_SYMBOL => Value::bool(a0.as_symbol().is_some()),
            IS_BOOLEAN => Value::bool(a0.as_bool().is_some()),
            AS_OBJ if a0.as_object().is_some() => a0,
            AS_FN if self.is_callable(a0) => a0,
            AS_STR if a0.as_string().is_some() => a0,
            AS_NUMBER if a0.is_number() => a0,
            AS_SYM if a0.as_symbol().is_some() => a0,
            AS_BOOL if a0.as_bool().is_some() => a0,
            AS_OBJ | AS_FN | AS_STR | AS_NUMBER | AS_SYM | AS_BOOL => {
                return bad_argument(info.name);
            }
            TO_BOOLEAN => Value::bool(conv::to_boolean(self.heap, a0)),
            TO_NUMBER_PRIMITIVE => {
                if a0.as_object().is_some() {
                    return bad_argument(info.name);
                }
                conv::number(conv::to_number(self.heap, a0)?)
            }
            TO_STRING_PRIMITIVE => {
                if a0.as_object().is_some() {
                    return bad_argument(info.name);
                }
                Value::string(conv::to_string(self.heap, a0)?)
            }
            TO_OBJECT => {
                if a0.as_object().is_some() {
                    a0
                } else {
                    match realm::wrap_primitive(self.heap, a0, None)? {
                        Some(o) => Value::object(o),
                        None => return type_error("cannot convert to object"),
                    }
                }
            }
            TO_INTEGER_OR_INFINITY_NUMBER => {
                let n = a0.as_number().unwrap_or(f64::NAN);
                let t = if n.is_nan() { 0.0 } else { libm::trunc(n) };
                // -0 becomes +0.
                conv::number(t + 0.0)
            }
            F64_TO_U32 | F64_TO_I32 => {
                let n = a0.as_number().unwrap_or(f64::NAN);
                let ok = if id == F64_TO_U32 {
                    (n as u32) as f64 == n
                } else {
                    (n as i32) as f64 == n
                };
                if !ok {
                    return bad_argument(info.name);
                }
                conv::number(n)
            }
            STR_LENGTH => match a0.as_string() {
                Some(s) => Value::int(self.heap.str_len(s) as i32),
                None => return bad_argument(info.name),
            },
            GET | GET_V => {
                // Dense elements first: builtins read arrays element by element.
                if let (Some(o), Some(i)) = (a0.as_object(), index_key(arg(self, 1)))
                    && let Some(v) = self.heap.get_element_fast(o, i)
                {
                    self.sp = base;
                    self.push(v);
                    self.pc = next;
                    return Ok(None);
                }
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                match self.lookup(a0, key)? {
                    Got::Value(v) => v,
                    Got::Getter(getter) => {
                        self.sp = base;
                        self.pc = next;
                        return self.call_accessor(getter, a0, None);
                    }
                    Got::Proxy(p) => {
                        self.sp = base;
                        self.pc = next;
                        return self.call_proxy_get(p, key, a0, Continuation::PushResult);
                    }
                }
            }
            SET => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                let v = arg(self, 2);
                // The result (undefined) goes below a setter's frame, if one is called.
                self.sp = base;
                self.push(Value::UNDEFINED);
                self.pc = next;
                return self.set_property(o, key, v, 0);
            }
            HAS_PROPERTY => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                match self.heap.has_property_or_proxy(o, key) {
                    Ok(r) => Value::bool(r),
                    Err(p) => {
                        self.sp = base;
                        self.pc = next;
                        let k = self.heap.atom_to_value(key)?;
                        return self.call_prelude(
                            proxy::PROXY_HAS,
                            &[Value::object(p), k],
                            Continuation::PushResult,
                        );
                    }
                }
            }
            CREATE_DATA_PROPERTY_OR_THROW => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                // An Array's dense element (or the next one) takes the fast store: for
                // those, defining a default data property and [[Set]] are the same.
                if let Some(i) = index_key(arg(self, 1))
                    && self.heap.object_class(o) == ObjectClass::Array
                    && self.heap.set_element_fast(o, i, arg(self, 2))?
                {
                    self.sp = base;
                    self.push(Value::UNDEFINED);
                    self.pc = next;
                    return Ok(None);
                }
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                if !self.heap.create_data_property(o, key, arg(self, 2))? {
                    return type_error("could not define property");
                }
                Value::UNDEFINED
            }
            DELETE_PROPERTY_OR_THROW => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                if !self.heap.delete_own_property(o, key) {
                    return type_error("could not delete property");
                }
                Value::UNDEFINED
            }
            // `f this a1 .. an` is already a call's stack layout.
            CALL => return self.call(base, argc - 2, Continuation::PushResult, next),
            // `f a1 .. an` is a construct's.
            CONSTRUCT => return self.construct(argc - 1, next),
            OBJECT_CREATE => {
                let proto = match (a0.as_object(), a0.is_null()) {
                    (Some(p), _) => Some(p),
                    (None, true) => None,
                    _ => return type_error("not an object"),
                };
                Value::object(self.heap.new_object(proto)?)
            }
            ARRAY_CREATE => {
                let n = a0.as_number().unwrap_or(f64::NAN);
                if !(n >= 0.0 && n <= u32::MAX as f64 && (n as u32) as f64 == n) {
                    return Err(Fault::error(
                        ErrorKind::RangeError,
                        String::from("invalid array length"),
                    ));
                }
                let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                let a = self.heap.new_array(Some(proto), 0)?;
                let d = PropertyDescriptor {
                    value: Some(conv::number(n)),
                    ..Default::default()
                };
                self.heap.define_own_property(a, atoms::LENGTH, &d)?;
                Value::object(a)
            }
            NEW_TYPE_ERROR | NEW_RANGE_ERROR | NEW_SYNTAX_ERROR | NEW_REFERENCE_ERROR => {
                let msg = match a0.as_string() {
                    Some(s) => self.heap.str_to_string(s),
                    None => return bad_argument(info.name),
                };
                let kind = match id {
                    NEW_TYPE_ERROR => ErrorKind::TypeError,
                    NEW_SYNTAX_ERROR => ErrorKind::SyntaxError,
                    NEW_REFERENCE_ERROR => ErrorKind::ReferenceError,
                    _ => ErrorKind::RangeError,
                };
                self.new_error_here(kind, &msg)?
            }
            NEW_NOT_CONSTRUCTOR_ERROR => {
                let msg = self.not_a_constructor_message(a0);
                self.new_error_here(ErrorKind::TypeError, &msg)?
            }
            WELL_KNOWN_SYMBOL => {
                // The names are the prelude's one-byte string constants: compare their
                // bytes in place instead of building a String on every call (0056).
                let name: &[u8] = match a0.as_string().and_then(|s| self.heap.str_view(s)) {
                    Some(crate::heap::StrView::Latin1(b)) => b,
                    _ => b"",
                };
                let atom = match name {
                    b"iterator" => atoms::SYM_ITERATOR,
                    b"asyncIterator" => atoms::SYM_ASYNC_ITERATOR,
                    b"hasInstance" => atoms::SYM_HAS_INSTANCE,
                    b"toPrimitive" => atoms::SYM_TO_PRIMITIVE,
                    b"toStringTag" => atoms::SYM_TO_STRING_TAG,
                    b"species" => atoms::SYM_SPECIES,
                    b"isConcatSpreadable" => atoms::SYM_IS_CONCAT_SPREADABLE,
                    b"unscopables" => atoms::SYM_UNSCOPABLES,
                    b"match" => atoms::SYM_MATCH,
                    b"matchAll" => atoms::SYM_MATCH_ALL,
                    b"replace" => atoms::SYM_REPLACE,
                    b"search" => atoms::SYM_SEARCH,
                    b"split" => atoms::SYM_SPLIT,
                    b"dispose" => atoms::SYM_DISPOSE,
                    b"asyncDispose" => atoms::SYM_ASYNC_DISPOSE,
                    b"metadata" => atoms::SYM_METADATA,
                    _ => return bad_argument(info.name),
                };
                Value::symbol(self.heap.atom_symbol(atom).expect("well-known symbol"))
            }
            GET_PROTOTYPE_OF | PREVENT_EXTENSIONS | IS_EXTENSIBLE | OBJECT_CLASS_TAG => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                match id {
                    GET_PROTOTYPE_OF => self
                        .heap
                        .get_prototype_of(o)
                        .map_or(Value::NULL, Value::object),
                    PREVENT_EXTENSIONS => Value::bool(self.heap.prevent_extensions(o)),
                    IS_EXTENSIBLE => Value::bool(self.heap.is_extensible(o)),
                    _ => {
                        let tag = match self.heap.is_array(o) {
                            None => return type_error("revoked proxy"),
                            Some(true) => "Array",
                            Some(false) if self.heap.is_proxy(o) && self.heap.is_callable(o) => {
                                "Function"
                            }
                            Some(false) => object::class_tag(self.heap, o),
                        };
                        Value::string(self.heap.alloc_str(tag)?)
                    }
                }
            }
            SET_PROTOTYPE_OF => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let proto = arg(self, 1);
                if proto.as_object().is_none() && !proto.is_null() {
                    return bad_argument(info.name);
                }
                // Object.prototype is an immutable prototype exotic object (10.4.7).
                if o == intrinsic_obj(self.heap, Intrinsic::ObjectProto)
                    && self.heap.get_prototype_of(o) != proto.as_object()
                {
                    Value::FALSE
                } else {
                    Value::bool(self.heap.set_prototype_of(o, proto.as_object()))
                }
            }
            SET_INTEGRITY_LEVEL | TEST_INTEGRITY_LEVEL => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let frozen = arg(self, 1).as_bool() == Some(true);
                Value::bool(if id == SET_INTEGRITY_LEVEL {
                    object::set_integrity_level(self.heap, o, frozen)?
                } else {
                    object::test_integrity_level(self.heap, o, frozen)
                })
            }
            SAME_VALUE => Value::bool(self.heap.same_value(a0, arg(self, 1))),
            WRAP_PRIMITIVE => {
                let proto = arg(self, 1).as_object();
                match realm::wrap_primitive(self.heap, a0, proto)? {
                    Some(o) if proto.is_some() => Value::object(o),
                    _ => return bad_argument(info.name),
                }
            }
            PRIMITIVE_VALUE => match a0.as_object() {
                None => a0,
                Some(o) => match self.heap.object_class(o) {
                    ObjectClass::Boolean
                    | ObjectClass::Number
                    | ObjectClass::String
                    | ObjectClass::Symbol
                    | ObjectClass::BigInt => self.heap.object_slot(o, 0),
                    _ => Value::UNDEFINED,
                },
            },
            BOUND_TARGET_FUNCTION => match a0.as_object() {
                Some(o) if self.heap.object_class(o) == ObjectClass::BoundFunction => {
                    self.heap.object_slot(o, 0)
                }
                _ => Value::UNDEFINED,
            },
            NEW_SYMBOL => {
                if !a0.is_undefined() && a0.as_string().is_none() {
                    return bad_argument(info.name);
                }
                Value::symbol(self.heap.new_symbol(a0)?)
            }
            SYMBOL_DESCRIPTION => match a0.as_symbol() {
                Some(sym) => self.heap.symbol_description(sym),
                None => return bad_argument(info.name),
            },
            SYMBOL_FOR => {
                let Some(key) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let registry = intrinsic_obj(self.heap, Intrinsic::SymbolRegistry);
                let atom = self.heap.intern(key)?;
                match self
                    .heap
                    .get_own_property(registry, atom)
                    .and_then(|p| p.value())
                {
                    Some(sym) => sym,
                    None => {
                        let sym = Value::symbol(self.heap.new_symbol(a0)?);
                        realm::define_hidden(self.heap, registry, atom, sym)?;
                        sym
                    }
                }
            }
            SYMBOL_KEY_FOR => {
                let Some(sym) = a0.as_symbol() else {
                    return bad_argument(info.name);
                };
                // Registered symbols are described by their key.
                let desc = self.heap.symbol_description(sym);
                match desc.as_string() {
                    Some(d) => {
                        let registry = intrinsic_obj(self.heap, Intrinsic::SymbolRegistry);
                        let atom = self.heap.intern(d)?;
                        let found = self
                            .heap
                            .get_own_property(registry, atom)
                            .and_then(|p| p.value());
                        if found == Some(a0) {
                            desc
                        } else {
                            Value::UNDEFINED
                        }
                    }
                    None => Value::UNDEFINED,
                }
            }
            NUMBER_TO_STRING | NUMBER_TO_FIXED | NUMBER_TO_EXPONENTIAL | NUMBER_TO_PRECISION => {
                let x = a0.as_number().unwrap_or(f64::NAN);
                let op = match id {
                    NUMBER_TO_STRING => 0,
                    NUMBER_TO_FIXED => 1,
                    NUMBER_TO_EXPONENTIAL => 2,
                    _ => 3,
                };
                Value::string(number::format(self.heap, op, x, arg(self, 1))?)
            }
            STRING_PARSE_FLOAT | STRING_PARSE_INT => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let radix = (id == STRING_PARSE_INT).then(|| arg(self, 1).as_int().unwrap_or(0));
                conv::number(number::parse(self.heap, s, radix)?)
            }
            COMPILE_EVAL | COMPILE_FUNCTION => {
                let (src, kind, body) = if id == COMPILE_EVAL {
                    (a0, None, Value::UNDEFINED)
                } else {
                    let kind = match a0.as_int() {
                        Some(0) => DynamicKind::Normal,
                        Some(1) => DynamicKind::Generator,
                        Some(2) => DynamicKind::Async,
                        Some(3) => DynamicKind::AsyncGenerator,
                        _ => return bad_argument(info.name),
                    };
                    (arg(self, 1), Some(kind), arg(self, 2))
                };
                let Some(s) = src.as_string() else {
                    return bad_argument(info.name);
                };
                // The compile is bounded by the source (D1591).
                let len =
                    self.heap.str_len(s) + body.as_string().map_or(0, |b| self.heap.str_len(b));
                self.fuel -= COMPILE_FUEL_PER_UNIT * i64::from(len);
                match kind {
                    None => eval::compile_eval(self.heap, s)?,
                    Some(kind) => {
                        let Some(b) = body.as_string() else {
                            return bad_argument(info.name);
                        };
                        eval::compile_function(self.heap, kind, s, b)?
                    }
                }
            }
            GLOBAL_OBJECT => realm::intrinsic(self.heap, Intrinsic::Global),
            ENQUEUE_MICROTASK => {
                let job = realm::installed_intrinsic(self.heap, MICROTASK_JOB);
                let args = [a0, Value::UNDEFINED, Value::UNDEFINED, Value::UNDEFINED];
                promise::enqueue(self.heap, job, args)?;
                Value::UNDEFINED
            }
            URI_ENCODE | URI_DECODE | STR_ESCAPE | STR_UNESCAPE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                // Linear work: charge for it like the other string scans.
                self.fuel -= i64::from(self.heap.str_len(s) / 8);
                let component = arg(self, 1).as_bool() == Some(true);
                Value::string(match id {
                    URI_ENCODE => uri::encode(self.heap, s, component)?,
                    URI_DECODE => uri::decode(self.heap, s, component)?,
                    STR_ESCAPE => uri::escape(self.heap, s)?,
                    _ => uri::unescape(self.heap, s)?,
                })
            }
            NUMBER_TO_INT32 => Value::int(conv::to_int32(a0.as_number().unwrap_or(f64::NAN))),
            NUMBER_TO_UINT32 => {
                conv::number(number::to_uint32(a0.as_number().unwrap_or(f64::NAN)) as f64)
            }
            MATH_RANDOM => Value::number(number::random(self.heap)),
            // An Array Iterator's slots: 0 the target (undefined once exhausted), 1 the
            // next index (a Number), 2 the kind (0 keys, 1 values, 2 entries).
            CREATE_ARRAY_ITERATOR => {
                let (Some(o), Some(kind)) = (a0.as_object(), arg(self, 1).as_int()) else {
                    return bad_argument(info.name);
                };
                let proto = realm::intrinsic_obj(self.heap, Intrinsic::ArrayIteratorProto);
                let it = self.heap.alloc_object(
                    ObjectClass::ArrayIterator,
                    Some(proto),
                    ObjectClass::ArrayIterator.slots(),
                )?;
                self.heap.set_object_slot(it, 0, Value::object(o));
                self.heap.set_object_slot(it, 1, Value::int(0));
                self.heap.set_object_slot(it, 2, Value::int(kind));
                Value::object(it)
            }
            ARRAY_ITERATOR_TARGET => {
                // Kind 3 is a String Iterator (0034): not an Array Iterator for `next`.
                let it = self.array_iterator(a0)?;
                if self.heap.object_slot(it, 2).as_int() == Some(STRING_ITERATOR_KIND) {
                    return type_error("Array Iterator object expected");
                }
                self.heap.object_slot(it, 0)
            }
            CREATE_STRING_ITERATOR => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let proto = realm::intrinsic_obj(self.heap, Intrinsic::StringIteratorProto);
                let it = self.heap.alloc_object(
                    ObjectClass::ArrayIterator,
                    Some(proto),
                    ObjectClass::ArrayIterator.slots(),
                )?;
                self.heap.set_object_slot(it, 0, Value::string(s));
                self.heap.set_object_slot(it, 1, Value::int(0));
                self.heap
                    .set_object_slot(it, 2, Value::int(STRING_ITERATOR_KIND));
                Value::object(it)
            }
            STRING_ITERATOR_TARGET => match a0.as_object() {
                Some(o)
                    if self.heap.object_class(o) == ObjectClass::ArrayIterator
                        && self.heap.object_slot(o, 2).as_int() == Some(STRING_ITERATOR_KIND) =>
                {
                    self.heap.object_slot(o, 0)
                }
                _ => return type_error("String Iterator object expected"),
            },
            STR_CODE_POINT_STRING => {
                let (Some(s), Some(i)) = (a0.as_string(), arg(self, 1).as_number()) else {
                    return bad_argument(info.name);
                };
                let s = self.heap.flatten(s)?;
                let i = i as u32;
                let n = match string::code_point_at(self.heap, s, i) {
                    Some(c) if c >= 65536.0 => 2,
                    Some(_) => 1,
                    None => return bad_argument(info.name),
                };
                Value::string(string::substring(self.heap, s, i, i + n)?)
            }
            HAS_ORIGINAL_ITERATION => Value::bool(self.iteration_unmodified(a0)),
            ARRAY_ITERATOR_INDEX => self.heap.object_slot(self.array_iterator(a0)?, 1),
            ARRAY_ITERATOR_SET_INDEX => {
                let it = self.array_iterator(a0)?;
                let Some(i) = arg(self, 1).as_number() else {
                    return bad_argument(info.name);
                };
                self.heap.set_object_slot(it, 1, conv::number(i));
                Value::UNDEFINED
            }
            ARRAY_ITERATOR_FINISH => {
                let it = self.array_iterator(a0)?;
                self.heap.set_object_slot(it, 0, Value::UNDEFINED);
                Value::UNDEFINED
            }
            ARRAY_ITERATOR_KIND => self.heap.object_slot(self.array_iterator(a0)?, 2),
            CREATE_ITER_RESULT_OBJECT => {
                let Some(done) = arg(self, 1).as_bool() else {
                    return bad_argument(info.name);
                };
                let proto = realm::intrinsic_obj(self.heap, Intrinsic::ObjectProto);
                let o = self.heap.new_object_from_entries(
                    Some(proto),
                    &[atoms::VALUE, atoms::DONE],
                    &[a0, Value::bool(done)],
                )?;
                Value::object(o)
            }
            CREATE_ARRAY_FROM_PAIR => {
                let proto = realm::intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                let pair = [a0, arg(self, 1)];
                Value::object(self.heap.new_array_from_values(Some(proto), &pair)?)
            }
            COMPARE_STRINGS => match (a0.as_string(), arg(self, 1).as_string()) {
                (Some(a), Some(b)) => conv::number(string::compare(self.heap, a, b)),
                _ => return bad_argument(info.name),
            },
            STRING_CODE_POINTS => match a0.as_string() {
                Some(s) => Value::object(string::code_points(self.heap, s)?),
                None => return bad_argument(info.name),
            },
            ARRAY_CAN_POP_DENSE => Value::bool(
                a0.as_object()
                    .is_some_and(|o| self.heap.array_can_pop_dense(o)),
            ),
            ARRAY_POP_DENSE => match a0.as_object().and_then(|o| self.heap.array_pop_dense(o)) {
                Some(v) => v,
                None => return bad_argument(info.name),
            },
            ARRAY_SEARCH_DENSE => {
                let (Some(o), Some(from), Some(len), Some(mode)) = (
                    a0.as_object(),
                    arg(self, 2).as_number(),
                    arg(self, 3).as_number(),
                    arg(self, 4).as_int(),
                ) else {
                    return bad_argument(info.name);
                };
                conv::number(self.array_search_dense(o, arg(self, 1), from, len, mode))
            }
            ARRAY_SLICE_DENSE => {
                let (Some(o), Some(k), Some(fin), Some(a)) = (
                    a0.as_object(),
                    arg(self, 1).as_number(),
                    arg(self, 2).as_number(),
                    arg(self, 3).as_object(),
                ) else {
                    return bad_argument(info.name);
                };
                Value::bool(self.array_slice_dense(o, k, fin, a)?)
            }
            ARRAY_JOIN_DENSE => {
                let (Some(o), Some(len), Some(sep)) = (
                    a0.as_object(),
                    arg(self, 1).as_number(),
                    arg(self, 2).as_string(),
                ) else {
                    return bad_argument(info.name);
                };
                self.array_join_dense(o, len, sep)?
            }
            ITERATOR_FAST_READY => Value::bool(self.iter_fast_ready(a0, arg(self, 1))),
            ITERATOR_FAST_VALUE => match self.iter_fast_next(a0, arg(self, 1))? {
                super::iter::FastStep::Value(v) => v,
                _ => return bad_argument(info.name),
            },
            ARRAY_SPLICE_DENSE => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let as_u32 = |v: Value| {
                    v.as_number()
                        .filter(|&n| (n as u32) as f64 == n)
                        .map(|n| n as u32)
                };
                match (
                    as_u32(arg(self, 1)),
                    as_u32(arg(self, 2)),
                    as_u32(arg(self, 3)),
                ) {
                    (Some(s), Some(d), Some(i)) => {
                        Value::bool(self.heap.array_splice_dense(o, s, d, i)?)
                    }
                    _ => Value::FALSE,
                }
            }
            STR_CHAR_CODE_AT | STR_CODE_POINT_AT | STR_SUBSTRING | STR_INDEX_OF
            | STR_LAST_INDEX_OF | STR_TO_CASE | STR_TRIM | STR_IS_WELL_FORMED
            | STR_TO_WELL_FORMED | STR_REPEAT | STR_NORMALIZE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                // A rope is flattened once (the result is cached in the rope), so a loop
                // of charCodeAt over a built-up string is not quadratic in its depth.
                let s = self.heap.flatten(s)?;
                // Integral, non-negative Number arguments (the builtins clamp them).
                let idx = |v: Value| {
                    v.as_number()
                        .filter(|n| *n >= 0.0)
                        .map_or(0, |n| n.min(u32::MAX as f64) as u32)
                };
                let a1 = arg(self, 1);
                match id {
                    STR_CHAR_CODE_AT => match a1.as_number() {
                        Some(n) if n >= 0.0 && n < self.heap.str_len(s) as f64 => {
                            Value::int(self.heap.char_code_at(s, n as u32).unwrap_or(0) as i32)
                        }
                        _ => Value::NAN,
                    },
                    STR_CODE_POINT_AT => match a1.as_number() {
                        Some(n) if n >= 0.0 && n < self.heap.str_len(s) as f64 => {
                            string::code_point_at(self.heap, s, n as u32)
                                .map_or(Value::UNDEFINED, conv::number)
                        }
                        _ => Value::UNDEFINED,
                    },
                    STR_SUBSTRING => {
                        Value::string(string::substring(self.heap, s, idx(a1), idx(arg(self, 2)))?)
                    }
                    STR_INDEX_OF | STR_LAST_INDEX_OF => {
                        let Some(search) = a1.as_string() else {
                            return bad_argument(info.name);
                        };
                        let from = idx(arg(self, 2));
                        conv::number(string::index_of(
                            self.heap,
                            s,
                            search,
                            from,
                            id == STR_LAST_INDEX_OF,
                        )?)
                    }
                    STR_TO_CASE => {
                        Value::string(string::to_case(self.heap, s, a1.as_bool() == Some(true))?)
                    }
                    STR_TRIM => {
                        Value::string(string::trim(self.heap, s, a1.as_int().unwrap_or(0))?)
                    }
                    STR_IS_WELL_FORMED => string::well_formed(self.heap, s, false)?,
                    STR_TO_WELL_FORMED => string::well_formed(self.heap, s, true)?,
                    STR_REPEAT => Value::string(string::repeat(self.heap, s, idx(a1))?),
                    _ => match string::normalize(self.heap, s, a1.as_int().unwrap_or(0) as u8)? {
                        Some(r) => Value::string(r),
                        None => Value::UNDEFINED,
                    },
                }
            }
            STR_LOCALE_COMPARE => match (a0.as_string(), arg(self, 1).as_string()) {
                (Some(a), Some(b)) => conv::number(string::locale_compare(self.heap, a, b)?),
                _ => return bad_argument(info.name),
            },
            STR_FROM_CODE_UNITS | STR_FROM_CODE_POINTS => match a0.as_object() {
                Some(list) => Value::string(string::from_codes(
                    self.heap,
                    list,
                    id == STR_FROM_CODE_POINTS,
                )?),
                None => return bad_argument(info.name),
            },
            SORT_PRIMITIVES => match a0.as_object() {
                Some(o) => Value::bool(array::sort_primitives(self.heap, o)?),
                None => return bad_argument(info.name),
            },
            REALM_INTRINSIC => {
                let name = a0
                    .as_string()
                    .map(|s| self.heap.str_to_string(s))
                    .unwrap_or_default();
                match Intrinsic::by_name(&name) {
                    Some(i) => realm::intrinsic(self.heap, i),
                    None => return bad_argument(info.name),
                }
            }
            ERROR_CREATE => {
                let Some(proto) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                Value::object(self.heap.alloc_object(ObjectClass::Error, Some(proto), 0)?)
            }
            CAPTURE_STACK => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let trace = self.stack_trace_from(self.pc, true);
                let s = self.heap.alloc_str(&trace)?;
                realm::define_hidden(self.heap, o, atoms::STACK, Value::string(s))?;
                Value::UNDEFINED
            }
            DEFINE_DATA_PROPERTY_OR_THROW => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = object::key(self.heap, arg(self, 1))?;
                let flags = object::flags(arg(self, 3));
                object::define_data_or_throw(self.heap, o, key, arg(self, 2), flags)?;
                Value::UNDEFINED
            }
            DEFINE_OWN_PROPERTY_FROM_RECORD => {
                let (Some(o), Some(rec)) = (a0.as_object(), arg(self, 2).as_object()) else {
                    return bad_argument(info.name);
                };
                let key = object::key(self.heap, arg(self, 1))?;
                Value::bool(object::define_from_record(self.heap, o, key, rec)?)
            }
            GET_OWN_PROPERTY_DESCRIPTOR_OBJECT | HAS_OWN_PROPERTY | OWN_PROPERTY_ATTRIBUTES => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = object::key(self.heap, arg(self, 1))?;
                match id {
                    HAS_OWN_PROPERTY => Value::bool(self.heap.has_own_property(o, key)),
                    OWN_PROPERTY_ATTRIBUTES => Value::int(object::attributes(self.heap, o, key)),
                    _ => object::descriptor_object(self.heap, o, key)?,
                }
            }
            OWN_PROPERTY_KEYS => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                object::own_keys(self.heap, o, arg(self, 1).as_int().unwrap_or(0))?
            }
            CALL_WITH_ARRAY => {
                // `f this args` becomes `f this a1 .. an`.
                let Some(args) = arg(self, 2).as_object() else {
                    return bad_argument(info.name);
                };
                let n = self.heap.array_length(args);
                self.ensure(base + 2 + n)?;
                for i in 0..n {
                    let v = self
                        .heap
                        .get_element_fast(args, i)
                        .unwrap_or(Value::UNDEFINED);
                    self.set(base + 2 + i, v);
                }
                self.sp = base + 2 + n;
                return self.call(base, n, Continuation::PushResult, next);
            }
            COLLECTION_CREATE => {
                let (Some(cls), Some(proto)) = (
                    a0.as_int().and_then(collection::class_of),
                    arg(self, 1).as_object(),
                ) else {
                    return bad_argument(info.name);
                };
                Value::object(self.heap.collection_new(cls, proto)?)
            }
            COLLECTION_CLASS => Value::int(collection::class_index(self.heap, a0)),
            COLLECTION_GET | COLLECTION_HAS | COLLECTION_SET | COLLECTION_DELETE
            | COLLECTION_CLEAR | COLLECTION_SIZE | COLLECTION_ITERATOR => {
                if collection::class_index(self.heap, a0) < 0 {
                    return bad_argument(info.name);
                }
                let m = a0.as_object().expect("collection");
                let key = arg(self, 1);
                match id {
                    COLLECTION_GET => self.heap.collection_get(m, key).unwrap_or(Value::UNDEFINED),
                    COLLECTION_HAS => Value::bool(self.heap.collection_has(m, key)),
                    COLLECTION_SET => {
                        let key = collection::stored_key(self.heap, key)?;
                        self.heap.collection_set(m, key, arg(self, 2))?;
                        Value::UNDEFINED
                    }
                    COLLECTION_DELETE => Value::bool(self.heap.collection_delete(m, key)?),
                    COLLECTION_CLEAR => {
                        self.heap.collection_clear(m)?;
                        Value::UNDEFINED
                    }
                    COLLECTION_SIZE => Value::int(self.heap.collection_size(m) as i32),
                    _ => Value::object(collection::iterator(
                        self.heap,
                        m,
                        key.as_int().unwrap_or(0),
                    )?),
                }
            }
            COLLECTION_ITERATOR_NEXT => {
                collection::iterator_next(self.heap, a0, arg(self, 1).as_bool() == Some(true))?
            }
            COLLECTION_ITERATOR_STEP | COLLECTION_ITERATOR_KEY | COLLECTION_ITERATOR_VALUE => {
                let Some(it) = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::CollectionIterator)
                else {
                    return bad_argument(info.name);
                };
                match id {
                    COLLECTION_ITERATOR_STEP => {
                        Value::bool(self.heap.collection_iterator_next(it).is_some())
                    }
                    COLLECTION_ITERATOR_KEY => self.heap.object_slot(it, 4),
                    _ => self.heap.object_slot(it, 5),
                }
            }
            CAN_BE_HELD_WEAKLY => Value::bool(collection::can_be_held_weakly(self.heap, a0)?),
            DATE_NOW => realm::intrinsic(self.heap, Intrinsic::Clock),
            DATE_CREATE => {
                let Some(proto) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let o = self.heap.alloc_object(ObjectClass::Date, Some(proto), 1)?;
                let t = arg(self, 1).as_number().unwrap_or(f64::NAN);
                self.heap.set_object_slot(o, 0, Value::number(t));
                Value::object(o)
            }
            DATE_VALUE | DATE_SET_VALUE => {
                let Some(o) = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Date)
                else {
                    return type_error("not a Date object");
                };
                if id == DATE_SET_VALUE {
                    let t = arg(self, 1).as_number().unwrap_or(f64::NAN);
                    self.heap.set_object_slot(o, 0, Value::number(t));
                }
                conv::number(self.heap.object_slot(o, 0).as_number().unwrap_or(f64::NAN))
            }
            DATE_FROM_FIELDS | DATE_SET_FIELDS => {
                let offset = self.time_zone_offset();
                let (list, local) = if id == DATE_FROM_FIELDS {
                    (a0.as_object(), arg(self, 1).as_bool() == Some(true))
                } else {
                    (
                        arg(self, 2).as_object(),
                        arg(self, 3).as_bool() == Some(true),
                    )
                };
                let Some(list) = list else {
                    return bad_argument(info.name);
                };
                let n = self.heap.array_length(list).min(7);
                let nums: alloc::vec::Vec<f64> = (0..n)
                    .map(|i| {
                        self.heap
                            .get_element_fast(list, i)
                            .and_then(|v| v.as_number())
                            .unwrap_or(f64::NAN)
                    })
                    .collect();
                let t = if id == DATE_FROM_FIELDS {
                    let mut f = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
                    f[..nums.len()].copy_from_slice(&nums);
                    date::from_fields_checked(&mut f, local, offset)
                } else {
                    let t = a0.as_number().unwrap_or(f64::NAN);
                    let first = arg(self, 1).as_int().unwrap_or(0).clamp(0, 6) as usize;
                    let n = nums.len().min(7 - first);
                    date::set_fields(t, first, &nums[..n], local, offset)
                };
                conv::number(t)
            }
            DATE_GET_FIELD => {
                let t = a0.as_number().unwrap_or(f64::NAN);
                let field = arg(self, 1).as_int().unwrap_or(0).clamp(0, 7) as usize;
                let local = arg(self, 2).as_bool() == Some(true);
                match date::fields(t, local, self.time_zone_offset(), false) {
                    Some(f) => conv::number(f[field]),
                    None => Value::NAN,
                }
            }
            DATE_FORMAT => {
                let t = a0.as_number().unwrap_or(f64::NAN);
                let fmt = arg(self, 1).as_int().unwrap_or(1) as u8;
                let part = arg(self, 2).as_int().unwrap_or(3) as u8;
                match date::format(t, fmt, part, self.time_zone_offset()) {
                    Some(s) => Value::string(self.heap.alloc_str(&s)?),
                    None if fmt == 2 => {
                        return Err(Fault::error(
                            ErrorKind::RangeError,
                            String::from("Date value is NaN"),
                        ));
                    }
                    None => Value::string(self.heap.alloc_str("Invalid Date")?),
                }
            }
            DATE_PARSE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let offset = self.time_zone_offset();
                conv::number(date::parse(self.heap.str_units(s), offset))
            }
            DATE_TIMEZONE_OFFSET => {
                let t = a0.as_number().unwrap_or(f64::NAN);
                if t.is_nan() {
                    Value::NAN
                } else {
                    conv::number(0.0 - self.time_zone_offset() as f64)
                }
            }
            ARRAY_BUFFER_CREATE => {
                let Some(proto) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let len = arg(self, 1).as_number().unwrap_or(f64::NAN);
                let max = arg(self, 2).as_number();
                Value::object(typed::buffer_create(self.heap, proto, len, max)?)
            }
            BUFFER_INFO => {
                let b = typed::as_buffer(self.heap, a0)?;
                typed::buffer_info(self.heap, b, arg(self, 1).as_int().unwrap_or(0))
            }
            BUFFER_DETACH | BUFFER_RESIZE | BUFFER_TRANSFER => {
                let b = typed::as_buffer(self.heap, a0)?;
                let n = arg(self, 1).as_number().unwrap_or(0.0);
                match id {
                    BUFFER_DETACH => {
                        typed::buffer_detach(self.heap, b);
                        Value::UNDEFINED
                    }
                    BUFFER_RESIZE => {
                        typed::buffer_resize(self.heap, b, n as u32)?;
                        Value::UNDEFINED
                    }
                    _ => Value::object(typed::buffer_transfer(
                        self.heap,
                        b,
                        n,
                        arg(self, 2).as_number(),
                    )?),
                }
            }
            BUFFER_COPY => {
                let src = typed::as_buffer(self.heap, a0)?;
                let dst = typed::as_buffer(self.heap, arg(self, 2))?;
                let num = |v: Value| v.as_number().unwrap_or(0.0) as u32;
                let (from, to, count) = (num(arg(self, 1)), num(arg(self, 3)), num(arg(self, 4)));
                if from as u64 + count as u64 > self.heap.buffer_len(src) as u64
                    || to as u64 + count as u64 > self.heap.buffer_len(dst) as u64
                {
                    return bad_argument(info.name);
                }
                typed::buffer_copy(self.heap, src, from, dst, to, count);
                Value::UNDEFINED
            }
            SORT_NUMBERS => match a0.as_object() {
                Some(o) => {
                    array::sort_numbers(self.heap, o)?;
                    Value::UNDEFINED
                }
                None => return bad_argument(info.name),
            },
            DATA_VIEW_CREATE => {
                let (Some(proto), Some(buffer)) = (a0.as_object(), arg(self, 1).as_object()) else {
                    return bad_argument(info.name);
                };
                let off = arg(self, 2).as_number().unwrap_or(0.0);
                let len = arg(self, 3).as_number().unwrap_or(-1.0);
                Value::object(typed::data_view_create(self.heap, proto, buffer, off, len)?)
            }
            DATA_VIEW_FIELD => {
                let o = typed::as_data_view(self.heap, a0)?;
                let field = arg(self, 1).as_int().unwrap_or(0).clamp(0, 2);
                self.heap.object_slot(o, field as u32)
            }
            DATA_VIEW_GET | DATA_VIEW_SET => {
                let o = typed::as_data_view(self.heap, a0)?;
                let at = arg(self, 1).as_number().unwrap_or(-1.0);
                let Some(kind) = arg(self, 2)
                    .as_int()
                    .and_then(crate::heap::typed::Kind::from_i32)
                else {
                    return bad_argument(info.name);
                };
                if id == DATA_VIEW_GET {
                    let little = arg(self, 3) == Value::TRUE;
                    let e = typed::view_get(self.heap, o, at, kind, little);
                    match e {
                        crate::heap::typed::Elem::Number(n) => Value::number(n),
                        _ => self.heap.elem_value(kind, e)?,
                    }
                } else {
                    let Some(e) = self.heap.element_of(kind, arg(self, 3)) else {
                        return bad_argument(info.name);
                    };
                    let little = arg(self, 4) == Value::TRUE;
                    typed::view_set(self.heap, o, at, kind, e, little);
                    Value::UNDEFINED
                }
            }
            IS_DATA_VIEW => Value::bool(
                a0.as_object()
                    .is_some_and(|o| self.heap.object_class(o) == ObjectClass::DataView),
            ),
            REALM_CONSTRUCTOR => {
                let Some(k) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let ctors = intrinsic_obj(self.heap, Intrinsic::Constructors);
                let key = self.heap.intern(k)?;
                match self
                    .heap
                    .get_own_property(ctors, key)
                    .and_then(|p| p.value())
                {
                    Some(f) => f,
                    None => return bad_argument(info.name),
                }
            }
            IS_TYPED_ARRAY => {
                Value::bool(a0.as_object().is_some_and(|o| self.heap.is_typed_array(o)))
            }
            TYPED_ARRAY_CREATE => {
                let (Some(proto), Some(kind), Some(buffer)) = (
                    a0.as_object(),
                    arg(self, 1)
                        .as_int()
                        .and_then(crate::heap::typed::Kind::from_i32),
                    arg(self, 2).as_object(),
                ) else {
                    return bad_argument(info.name);
                };
                let off = arg(self, 3).as_number().unwrap_or(0.0);
                let len = arg(self, 4).as_number().unwrap_or(-1.0);
                Value::object(typed::typed_array_create(
                    self.heap, proto, kind, buffer, off, len,
                )?)
            }
            TYPED_ARRAY_FIELD => {
                let o = typed::as_typed_array(self.heap, a0)?;
                typed::typed_array_field(self.heap, o, arg(self, 1).as_int().unwrap_or(0))
            }
            TYPED_ARRAY_GET | TYPED_ARRAY_SET => {
                let o = typed::as_typed_array(self.heap, a0)?;
                let i = arg(self, 1).as_number().unwrap_or(-1.0);
                let index =
                    (i >= 0.0 && i <= u32::MAX as f64 && i == libm::trunc(i)).then_some(i as u32);
                if id == TYPED_ARRAY_GET {
                    match index {
                        Some(i) => self.heap.ta_get_alloc(o, i)?.unwrap_or(Value::UNDEFINED),
                        None => Value::UNDEFINED,
                    }
                } else {
                    // A Number for a Number kind, a BigInt for a BigInt kind (converted).
                    // Anything else is a primitive the caller did not convert (`undefined`
                    // read past a shrunk buffer): ToBigInt's or ToNumber's TypeError.
                    let kind = self.heap.ta_kind(o);
                    let Some(e) = self.heap.element_of(kind, arg(self, 2)) else {
                        return type_error(if kind.is_bigint() {
                            "cannot convert to bigint"
                        } else {
                            "cannot convert bigint to number"
                        });
                    };
                    if let Some(i) = index {
                        self.heap.ta_set(o, i, e);
                    }
                    Value::UNDEFINED
                }
            }
            TYPED_ARRAY_COPY => {
                let src = typed::as_typed_array(self.heap, a0)?;
                let dst = typed::as_typed_array(self.heap, arg(self, 2))?;
                let num = |v: Value| v.as_number().unwrap_or(0.0) as u32;
                let (from, to, count) = (num(arg(self, 1)), num(arg(self, 3)), num(arg(self, 4)));
                let (ls, ld) = (
                    self.heap.ta_length(src).unwrap_or(0),
                    self.heap.ta_length(dst).unwrap_or(0),
                );
                if from as u64 + count as u64 > ls as u64 || to as u64 + count as u64 > ld as u64 {
                    return bad_argument(info.name);
                }
                typed::typed_array_copy(self.heap, src, from, dst, to, count);
                Value::UNDEFINED
            }
            TYPED_ARRAY_KIND_PROTO => {
                let Some(kind) = a0.as_int().and_then(crate::heap::typed::Kind::from_i32) else {
                    return bad_argument(info.name);
                };
                realm::intrinsic(self.heap, typed::kind_proto(kind))
            }
            BASE64_ENCODE | HEX_ENCODE => {
                let o = typed::as_typed_array(self.heap, a0)?;
                let Some(bytes) = typed::uint8_bytes(self.heap, o) else {
                    return type_error("ArrayBuffer is detached");
                };
                let s = if id == HEX_ENCODE {
                    typed::to_hex(bytes)
                } else {
                    typed::to_base64(
                        bytes,
                        arg(self, 1).as_bool() == Some(true),
                        arg(self, 2).as_bool() == Some(true),
                    )
                };
                Value::string(self.heap.alloc_latin1(s.as_bytes())?)
            }
            BASE64_DECODE | HEX_DECODE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let units: alloc::vec::Vec<u16> = self.heap.str_units(s).collect();
                let max_arg = if id == HEX_DECODE {
                    arg(self, 1)
                } else {
                    arg(self, 3)
                };
                let max = max_arg.as_number().unwrap_or(f64::INFINITY);
                let max = if max >= usize::MAX as f64 {
                    usize::MAX
                } else {
                    max as usize
                };
                let d = if id == HEX_DECODE {
                    typed::from_hex(&units, max)
                } else {
                    let last = arg(self, 2).as_int().unwrap_or(0) as u8;
                    typed::from_base64(&units, arg(self, 1).as_bool() == Some(true), last, max)
                };
                typed::decoded_record(self.heap, d)?
            }
            proxy::CONSTRUCT_WITH_THIS => return self.construct_with_this(base, next),
            CONSTRUCT_WITH_ARRAY => {
                // `f args newTarget` becomes `f a1 .. an`.
                let Some(args) = arg(self, 1).as_object() else {
                    return bad_argument(info.name);
                };
                let new_target = arg(self, 2);
                let n = self.heap.array_length(args);
                self.ensure(base + 1 + n)?;
                for i in 0..n {
                    let v = self
                        .heap
                        .get_element_fast(args, i)
                        .unwrap_or(Value::UNDEFINED);
                    self.set(base + 1 + i, v);
                }
                self.sp = base + 1 + n;
                return self.construct_with(n, next, Some(new_target));
            }
            GET_WITH_RECEIVER => {
                let key = object::key(self.heap, arg(self, 1))?;
                let receiver = arg(self, 2);
                match self.lookup(a0, key)? {
                    Got::Value(v) => v,
                    Got::Getter(getter) => {
                        self.sp = base;
                        self.pc = next;
                        return self.call_accessor(getter, receiver, None);
                    }
                    Got::Proxy(p) => {
                        self.sp = base;
                        self.pc = next;
                        return self.call_proxy_get(p, key, receiver, Continuation::PushResult);
                    }
                }
            }
            SET_WITH_RECEIVER => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = object::key(self.heap, arg(self, 1))?;
                let (v, receiver) = (arg(self, 2), arg(self, 3));
                match self.ordinary_set(o, key, v, receiver)? {
                    SetStep::Done(done) => Value::bool(done),
                    SetStep::Setter(setter) => {
                        // `true` goes below the setter's frame; its result is dropped.
                        self.sp = base;
                        self.push(Value::TRUE);
                        self.pc = next;
                        return self.call_accessor(setter, receiver, Some(v));
                    }
                    SetStep::Proxy(p) => {
                        self.sp = base;
                        self.pc = next;
                        let k = self.heap.atom_to_value(key)?;
                        let args = [Value::object(p), k, v, receiver];
                        return self.call_prelude(
                            proxy::PROXY_SET,
                            &args,
                            Continuation::PushResult,
                        );
                    }
                    SetStep::Receiver(r) => {
                        self.sp = base;
                        self.pc = next;
                        let k = self.heap.atom_to_value(key)?;
                        let args = [Value::object(r), k, v];
                        return self.call_prelude(
                            proxy::PROXY_SET_ON_RECEIVER,
                            &args,
                            Continuation::PushResult,
                        );
                    }
                    SetStep::ArrayLength(a) => {
                        self.sp = base;
                        self.pc = next;
                        let args = [Value::object(a), v, Value::FALSE];
                        return self.call_prelude(
                            ARRAY_SET_LENGTH,
                            &args,
                            Continuation::PushResult,
                        );
                    }
                }
            }
            DELETE_PROPERTY => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let key = object::key(self.heap, arg(self, 1))?;
                Value::bool(self.heap.delete_own_property(o, key))
            }
            BOUND_FUNCTION_CREATE => {
                let (Some(target), Some(args)) = (a0.as_object(), arg(self, 2).as_object()) else {
                    return bad_argument(info.name);
                };
                function::bound_function_create(self.heap, target, arg(self, 1), args)?
            }
            FUNCTION_TO_STRING => Value::string(function::to_string(self.heap, a0)?),
            NUMBER_POW => conv::number(conv::pow(
                a0.as_number().unwrap_or(f64::NAN),
                arg(self, 1).as_number().unwrap_or(f64::NAN),
            )),
            SYMBOL_DESCRIPTIVE_STRING => {
                let Some(sym) = a0.as_symbol() else {
                    return bad_argument(info.name);
                };
                let d = match self.heap.symbol_description(sym).as_string() {
                    Some(d) => self.heap.str_to_string(d),
                    None => String::new(),
                };
                Value::string(self.heap.alloc_str(&format!("Symbol({d})"))?)
            }
            INSTALL_CONSTRUCTOR => {
                self.install_constructor(
                    a0,
                    arg(self, 1),
                    arg(self, 2),
                    arg(self, 3),
                    arg(self, 4),
                )?;
                Value::UNDEFINED
            }
            INSTALL_ACCESSOR => {
                self.install_accessor(a0, arg(self, 1), arg(self, 2), arg(self, 3))?;
                Value::UNDEFINED
            }
            INSTALL_VALUE => {
                let Some(t) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let path = self.heap.str_to_string(t);
                let holder = self.resolve_path(&path)?;
                let (key, _) = self.install_key(arg(self, 1))?;
                let d = PropertyDescriptor::data(arg(self, 2), object::flags(arg(self, 3)));
                self.heap.define_own_property(holder, key, &d)?;
                Value::UNDEFINED
            }
            INSTALL_METHOD => {
                self.install_method(a0, arg(self, 1), arg(self, 2), arg(self, 3))?;
                Value::UNDEFINED
            }
            PROMISE_CREATE => {
                let Some(proto) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                Value::object(promise::alloc(self.heap, Some(proto))?)
            }
            IS_PROMISE => Value::bool(promise::as_promise(self.heap, a0).is_some()),
            INTRINSIC_PROMISE => realm::installed_intrinsic(self.heap, PROMISE_CONSTRUCTOR),
            PROMISE_STATE
            | PROMISE_RESULT
            | FULFILL_PROMISE
            | REJECT_PROMISE
            | PROMISE_CLAIM_RESOLVE
            | PERFORM_PROMISE_THEN
            | ENQUEUE_PROMISE_RESOLVE_THENABLE_JOB => {
                let Some(p) = promise::as_promise(self.heap, a0) else {
                    return bad_argument(info.name);
                };
                match id {
                    PROMISE_STATE => Value::int(promise::state_of(self.heap, p)),
                    PROMISE_RESULT => promise::result_of(self.heap, p),
                    FULFILL_PROMISE | REJECT_PROMISE => {
                        let v = arg(self, 1);
                        promise::settle(self.heap, p, id == FULFILL_PROMISE, v)?;
                        Value::UNDEFINED
                    }
                    PROMISE_CLAIM_RESOLVE => {
                        let f = promise::flags(self.heap, p);
                        promise::set_flags(self.heap, p, f | promise::FLAG_RESOLVED);
                        Value::bool(f & promise::FLAG_RESOLVED == 0)
                    }
                    PERFORM_PROMISE_THEN => {
                        let (f, r, cap) = (arg(self, 1), arg(self, 2), arg(self, 3));
                        promise::perform_then(self.heap, p, f, r, cap)?;
                        Value::UNDEFINED
                    }
                    _ => {
                        let job =
                            realm::installed_intrinsic(self.heap, PROMISE_RESOLVE_THENABLE_JOB);
                        let args = [a0, arg(self, 1), arg(self, 2), Value::UNDEFINED];
                        promise::enqueue(self.heap, job, args)?;
                        Value::UNDEFINED
                    }
                }
            }
            GENERATOR_STATE => match suspend::as_generator(self.heap, a0, suspend::GENERATOR) {
                Some(g) => Value::int(suspend::state_of(self.heap, g)),
                None => Value::int(-1),
            },
            GENERATOR_SET_STATE => {
                let g = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Generator);
                let s = arg(self, 1).as_int().filter(|s| (0..=6).contains(s));
                let (Some(g), Some(s)) = (g, s) else {
                    return bad_argument(info.name);
                };
                suspend::set_state(self.heap, g, s);
                Value::UNDEFINED
            }
            GENERATOR_RESUME => {
                let (value, mode) = (arg(self, 1), arg(self, 2));
                self.sp = base;
                return self.generator_resume(a0, value, mode, next);
            }
            IS_ASYNC_FUNCTION_STATE => {
                Value::bool(suspend::as_generator(self.heap, a0, suspend::ASYNC).is_some())
            }
            ASYNC_FUNCTION_PROMISE => match suspend::as_generator(self.heap, a0, suspend::ASYNC) {
                Some(a) => suspend::promise_of(self.heap, a),
                None => return bad_argument(info.name),
            },
            ASYNC_GENERATOR_STATE => {
                match suspend::as_generator(self.heap, a0, suspend::ASYNC_GENERATOR) {
                    Some(g) => Value::int(suspend::state_of(self.heap, g)),
                    None => Value::int(-1),
                }
            }
            ASYNC_GENERATOR_ENQUEUE | ASYNC_GENERATOR_FIRST_REQUEST | ASYNC_GENERATOR_DEQUEUE => {
                let Some(g) = suspend::as_generator(self.heap, a0, suspend::ASYNC_GENERATOR) else {
                    return bad_argument(info.name);
                };
                match id {
                    ASYNC_GENERATOR_ENQUEUE => {
                        suspend::enqueue_request(self.heap, g, arg(self, 1))?;
                        Value::UNDEFINED
                    }
                    ASYNC_GENERATOR_FIRST_REQUEST => suspend::first_request(self.heap, g),
                    _ => suspend::dequeue_request(self.heap, g),
                }
            }
            CREATE_ASYNC_FROM_SYNC_ITERATOR => {
                if a0.as_object().is_none() {
                    return bad_argument(info.name);
                }
                let proto = intrinsic_obj(self.heap, Intrinsic::AsyncFromSyncIteratorProto);
                let r = self
                    .heap
                    .alloc_object(ObjectClass::Record, Some(proto), 2)?;
                self.heap.set_object_slot(r, 0, a0);
                self.heap.set_object_slot(r, 1, arg(self, 1));
                Value::object(r)
            }
            NEW_RECORD => {
                let n = a0.as_int().and_then(|n| u32::try_from(n).ok());
                let Some(n) = n.filter(|&n| n <= 16) else {
                    return bad_argument(info.name);
                };
                let r = self.heap.alloc_object(ObjectClass::Record, None, n)?;
                for i in 0..n {
                    self.heap.set_object_slot(r, i, Value::UNDEFINED);
                }
                Value::object(r)
            }
            RECORD_GET | RECORD_SET => {
                let r = a0
                    .as_object()
                    .filter(|&o| is_slot_object(self.heap.object_class(o)));
                let i = arg(self, 1).as_int().and_then(|i| u32::try_from(i).ok());
                let (Some(r), Some(i)) = (r, i) else {
                    return bad_argument(info.name);
                };
                if i >= self.heap.object_slot_count(r) {
                    return bad_argument(info.name);
                }
                if id == RECORD_GET {
                    self.heap.object_slot(r, i)
                } else {
                    self.heap.set_object_slot(r, i, arg(self, 2));
                    Value::UNDEFINED
                }
            }
            NEW_BUILTIN_CLOSURE => {
                let kind = a0.as_int().and_then(|k| usize::try_from(k).ok());
                let target = kind.and_then(|k| BUILTIN_CLOSURES.get(k).copied());
                let (Some(target), Some(length)) = (target, arg(self, 2).as_int()) else {
                    return bad_argument(info.name);
                };
                let proto = intrinsic_obj(self.heap, Intrinsic::FunctionProto);
                let f = self
                    .heap
                    .alloc_object(ObjectClass::BuiltinClosure, Some(proto), 2)?;
                self.heap.set_object_slot(f, 0, Value::int(target as i32));
                self.heap.set_object_slot(f, 1, arg(self, 1));
                let ro = PropFlags::CONFIGURABLE;
                let d = PropertyDescriptor::data(Value::int(length), ro);
                self.heap.define_own_property(f, atoms::LENGTH, &d)?;
                let empty = Value::string(self.heap.atom_string(atoms::EMPTY)?);
                let d = PropertyDescriptor::data(empty, ro);
                self.heap.define_own_property(f, atoms::NAME, &d)?;
                Value::object(f)
            }
            MARK_NATIVE => {
                self.mark_native(a0, arg(self, 1))?;
                Value::UNDEFINED
            }
            INSTALL_INTRINSIC => {
                let name = a0
                    .as_string()
                    .map(|s| self.heap.str_to_string(s))
                    .unwrap_or_default();
                let f = arg(self, 1);
                match intrinsic_id(&name) {
                    Some(i) if intrinsic(i).unwrap().kind == IntrinsicKind::Builtin => {
                        realm::install_intrinsic(self.heap, i, f);
                        if let Some(&(_, proto, name)) =
                            FUNCTION_KIND_CONSTRUCTORS.iter().find(|c| c.0 == i)
                        {
                            self.wire_function_kind_constructor(f, proto, name)?;
                        }
                    }
                    _ => return bad_argument(info.name),
                }
                Value::UNDEFINED
            }
            DOM_EXCEPTION_CREATE => {
                let (Some(proto), Some(name), Some(msg)) = (
                    a0.as_object(),
                    arg(self, 1).as_string(),
                    arg(self, 2).as_string(),
                ) else {
                    return bad_argument(info.name);
                };
                // An Error with two slots: nothing else allocates one (D1840).
                let o = self.heap.alloc_object(ObjectClass::Error, Some(proto), 2)?;
                self.heap.set_object_slot(o, 0, Value::string(name));
                self.heap.set_object_slot(o, 1, Value::string(msg));
                Value::object(o)
            }
            DOM_EXCEPTION_FIELD => {
                let i = arg(self, 1).as_int().and_then(|i| u32::try_from(i).ok());
                match (a0.as_object(), i) {
                    (Some(o), Some(i))
                        if i < 2
                            && self.heap.object_class(o) == ObjectClass::Error
                            && self.heap.object_slot_count(o) == 2 =>
                    {
                        self.heap.object_slot(o, i)
                    }
                    _ => Value::UNDEFINED,
                }
            }
            DOM_EXCEPTION_CODE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                let units: alloc::vec::Vec<u16> = self.heap.str_units(s).collect();
                Value::int(crate::builtins::dom::code_of(&units))
            }
            DOM_EXCEPTION_CONSTANT => {
                let i = a0.as_int().and_then(|i| usize::try_from(i).ok());
                let Some(&(_, name)) =
                    i.and_then(|i| crate::builtins::dom::NAMES.get(i.wrapping_sub(1)))
                else {
                    return bad_argument(info.name);
                };
                Value::string(self.heap.alloc_str(name)?)
            }
            BTOA | ATOB => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(info.name);
                };
                if id == BTOA {
                    match crate::builtins::dom::btoa(self.heap.str_units(s)) {
                        Some(out) => Value::string(self.heap.alloc_str(&out)?),
                        None => Value::UNDEFINED,
                    }
                } else {
                    let units: alloc::vec::Vec<u16> = self.heap.str_units(s).collect();
                    match crate::builtins::dom::atob(&units) {
                        Some(bytes) => Value::string(self.heap.alloc_latin1(&bytes)?),
                        None => Value::UNDEFINED,
                    }
                }
            }
            F64_SUM_PRECISE => {
                let Some(list) = a0.as_object() else {
                    return bad_argument(info.name);
                };
                let n = self.heap.array_length(list);
                let heap = &*self.heap;
                let values = (0..n).map(|i| {
                    heap.get_element_fast(list, i)
                        .and_then(|v| v.as_number())
                        .unwrap_or(f64::NAN)
                });
                conv::number(crate::builtins::math::sum_precise(values))
            }
            _ if let Some(r) = self.proxy_leaf(id, a0, arg(self, 1)) => r?,
            _ => match math_leaf(
                id,
                a0.as_number().unwrap_or(f64::NAN),
                arg(self, 1).as_number().unwrap_or(f64::NAN),
            ) {
                Some(r) => conv::number(r),
                None => return self.json_regexp_intrinsic(id, info.name, base, argc, next),
            },
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }

    pub(super) fn is_callable(&self, v: Value) -> bool {
        v.as_object().is_some_and(|o| self.heap.is_callable(o))
    }

    /// A new error of `kind` with a stack from the current frames (the intrinsic's
    /// caller is the current frame).
    fn new_error_here(&mut self, kind: ErrorKind, msg: &str) -> Result<Value, Fault> {
        let e = realm::new_error(self.heap, kind, msg)?;
        self.set_stack(e, self.pc)?;
        Ok(e)
    }

    /// `%InstallMethod(target, key, f, length)` (D1172): `f` becomes the method `key` of
    /// the object at the dotted path `target` (writable, configurable, not enumerable),
    /// with `name` = key and `length`.
    fn install_method(
        &mut self,
        target: Value,
        key: Value,
        f: Value,
        length: Value,
    ) -> Result<(), Fault> {
        let (Some(t), Some(fo)) = (target.as_string(), f.as_object()) else {
            return bad_argument("InstallMethod");
        };
        let path = self.heap.str_to_string(t);
        let holder = self.resolve_path(&path)?;
        let (key, name) = self.install_key(key)?;
        realm::define_hidden(self.heap, holder, key, f)?;
        // A function installed twice (`Number.parseInt` is `parseInt`) keeps its first
        // name, as the spec's builtins do.
        if self.heap.cell_flags(fo.offset()) & OBJ_BUILTIN != 0 {
            return Ok(());
        }
        self.name_and_length(fo, name, length.as_number().unwrap_or(0.0))?;
        self.mark_builtin(fo, OBJ_BUILTIN);
        Ok(())
    }

    /// The host's local time zone offset (minutes east of UTC, D1383).
    fn time_zone_offset(&self) -> i64 {
        realm::intrinsic(self.heap, Intrinsic::TimeZoneOffset)
            .as_int()
            .unwrap_or(0) as i64
    }

    /// OrdinarySet ( O, P, V, Receiver ), 10.1.9.2, as far as it needs no user code:
    /// the result, or the setter, proxy trap or receiver step the caller must run.
    fn ordinary_set(
        &mut self,
        o: Obj,
        key: Atom,
        v: Value,
        receiver: Value,
    ) -> Result<SetStep, Fault> {
        // A typed array receiving its own numeric key writes the element (the value was
        // converted by the builtin); an invalid index elsewhere on the chain is a no-op.
        if self.heap.is_typed_array(o)
            && receiver.as_object() == Some(o)
            && self.heap.canonical_numeric_index(key).is_some()
        {
            self.heap.ta_numeric_set(o, key, v);
            return Ok(SetStep::Done(true));
        }
        let found = self.heap.lookup_property(o, key);
        if matches!(found, Lookup::Absent) && self.heap.typed_array_set_stops(o, key) {
            return Ok(SetStep::Done(true));
        }
        match found {
            // The [[Set]] of a proxy on the chain (or o itself) takes over.
            Lookup::Proxy(p) => return Ok(SetStep::Proxy(p)),
            Lookup::Found(_, Property::Accessor { setter, .. }) => {
                return Ok(if setter.is_undefined() {
                    SetStep::Done(false)
                } else {
                    SetStep::Setter(setter)
                });
            }
            Lookup::Found(_, Property::Data { flags, .. }) if !flags.writable() => {
                return Ok(SetStep::Done(false));
            }
            _ => {}
        }
        let Some(r) = receiver.as_object() else {
            return Ok(SetStep::Done(false));
        };
        if self.heap.is_proxy(r) {
            return Ok(SetStep::Receiver(r));
        }
        Ok(SetStep::Done(match self.heap.get_own_property(r, key) {
            Some(Property::Accessor { .. }) => false,
            Some(Property::Data { flags, .. }) if !flags.writable() => false,
            Some(_) => {
                let mut v = v;
                if key == atoms::LENGTH && self.heap.object_class(r) == ObjectClass::Array {
                    match self.array_length_value(v)? {
                        Some(len) => v = len,
                        None => return Ok(SetStep::ArrayLength(r)),
                    }
                }
                let d = PropertyDescriptor {
                    value: Some(v),
                    ..Default::default()
                };
                self.heap.define_own_property(r, key, &d)?
            }
            None => self.heap.create_data_property(r, key, v)?,
        }))
    }

    /// The property key of an install call (a string or a symbol) and the function name
    /// it gives (`[description]` for a symbol).
    fn install_key(&mut self, key: Value) -> Result<(Atom, Value), Fault> {
        if let Some(k) = key.as_string() {
            return Ok((self.heap.intern(k)?, Value::string(k)));
        }
        let Some(sym) = key.as_symbol() else {
            return bad_argument("Install*");
        };
        let d = match self.heap.symbol_description(sym).as_string() {
            Some(d) => self.heap.str_to_string(d),
            None => String::new(),
        };
        let name = self.heap.alloc_str(&format!("[{d}]"))?;
        Ok((self.heap.symbol_atom(sym), Value::string(name)))
    }

    /// Flags an installed prelude function; non-constructors lose the `prototype` that
    /// every ordinary function gets (D1310).
    /// %GeneratorFunction% and friends (D1420): a builtin constructor named `name` with
    /// [[Prototype]] %Function% and `prototype` = the realm's `proto` (whose
    /// `constructor` it becomes), both configurable but not writable. Not globals.
    fn wire_function_kind_constructor(
        &mut self,
        f: Value,
        proto: Intrinsic,
        name: &str,
    ) -> Result<(), Fault> {
        let Some(fo) = f.as_object() else {
            return bad_argument("InstallIntrinsic");
        };
        let p = intrinsic_obj(self.heap, proto);
        let function = self.heap.atom_str("Function")?;
        let global = intrinsic_obj(self.heap, Intrinsic::Global);
        let ctor = self
            .heap
            .get_own_property(global, function)
            .and_then(|p| p.value())
            .and_then(|v| v.as_object());
        self.heap.set_prototype_of(fo, ctor);
        let n = self.heap.alloc_str(name)?;
        self.name_and_length(fo, Value::string(n), 1.0)?;
        self.heap
            .remove_own_property_unchecked(fo, atoms::PROTOTYPE);
        let d = PropertyDescriptor::data(Value::object(p), PropFlags::NONE);
        self.heap.define_own_property(fo, atoms::PROTOTYPE, &d)?;
        let d = PropertyDescriptor::data(f, PropFlags::CONFIGURABLE);
        self.heap.define_own_property(p, atoms::CONSTRUCTOR, &d)?;
        self.mark_builtin(fo, OBJ_BUILTIN | OBJ_BUILTIN_CTOR);
        Ok(())
    }

    fn mark_builtin(&mut self, fo: Obj, flag: u16) {
        let flags = self.heap.cell_flags(fo.offset());
        self.heap.set_cell_flags(fo.offset(), flags | flag);
        if flag & OBJ_BUILTIN_CTOR == 0 {
            self.heap
                .remove_own_property_unchecked(fo, atoms::PROTOTYPE);
        }
    }

    /// `name` and `length` of an installed builtin (configurable only).
    /// `length` then `name` (qjs's and the spec's order of a builtin's own keys).
    fn name_and_length(&mut self, fo: Obj, name: Value, length: f64) -> Result<(), Fault> {
        let ro = PropFlags::CONFIGURABLE;
        let d = PropertyDescriptor::data(conv::number(length), ro);
        self.heap.define_own_property(fo, atoms::LENGTH, &d)?;
        let d = PropertyDescriptor::data(name, ro);
        self.heap.define_own_property(fo, atoms::NAME, &d)?;
        Ok(())
    }

    /// `%InstallConstructor(target, key, f, length, parent)` (D1310): `f` becomes the
    /// constructor `key` with its prototype object (the realm's, or a new one).
    fn install_constructor(
        &mut self,
        target: Value,
        key: Value,
        f: Value,
        length: Value,
        parent: Value,
    ) -> Result<(), Fault> {
        let (Some(t), Some(k), Some(fo)) = (target.as_string(), key.as_string(), f.as_object())
        else {
            return bad_argument("InstallConstructor");
        };
        let path = self.heap.str_to_string(t);
        let holder = self.resolve_path(&path)?;
        let name = self.heap.str_to_string(k);
        let parent = match parent.as_string() {
            Some(p) => {
                let p = self.heap.str_to_string(p);
                Some(self.resolve_path(&p)?)
            }
            None => None,
        };
        let proto = match Intrinsic::prototype_of(&name) {
            Some(i) => intrinsic_obj(self.heap, i),
            None => {
                let base = match parent {
                    Some(p) => self
                        .heap
                        .get_own_property(p, atoms::PROTOTYPE)
                        .and_then(|p| p.value())
                        .and_then(|v| v.as_object()),
                    None => None,
                };
                let base = base.unwrap_or_else(|| intrinsic_obj(self.heap, Intrinsic::ObjectProto));
                self.heap.new_object(Some(base))?
            }
        };
        // WebIDL: DOMException.prototype inherits from Error.prototype, but the
        // constructor itself from Function.prototype.
        if let Some(p) = parent
            && name != "DOMException"
        {
            self.heap.set_prototype_of(fo, Some(p));
        }
        let ctors = intrinsic_obj(self.heap, Intrinsic::Constructors);
        let ckey = self.heap.intern(k)?;
        realm::define_hidden(self.heap, ctors, ckey, f)?;
        if name == "%TypedArray%" {
            // A constructor that is a realm intrinsic, not a global (D1391).
            realm::set_intrinsic(self.heap, Intrinsic::TypedArrayCtor, f);
        } else {
            let key = self.heap.intern(k)?;
            realm::define_hidden(self.heap, holder, key, f)?;
        }
        let fname = if name.starts_with('%') {
            Value::string(self.heap.alloc_str(name.trim_matches('%'))?)
        } else {
            Value::string(k)
        };
        self.name_and_length(fo, fname, length.as_number().unwrap_or(0.0))?;
        // The Proxy constructor has no `prototype` (28.2.2): proxies take theirs from
        // the target.
        if name != "Proxy" {
            let d = PropertyDescriptor::data(Value::object(proto), PropFlags::NONE);
            self.heap.define_own_property(fo, atoms::PROTOTYPE, &d)?;
            realm::define_hidden(self.heap, proto, atoms::CONSTRUCTOR, f)?;
        }
        self.mark_builtin(fo, OBJ_BUILTIN | OBJ_BUILTIN_CTOR);
        Ok(())
    }

    /// `%InstallAccessor(target, key, f, isSetter)` (D1310): the getter or setter half
    /// of an accessor (configurable, not enumerable), named `get key` / `set key`.
    fn install_accessor(
        &mut self,
        target: Value,
        key: Value,
        f: Value,
        is_setter: Value,
    ) -> Result<(), Fault> {
        let (Some(t), Some(fo)) = (target.as_string(), f.as_object()) else {
            return bad_argument("InstallAccessor");
        };
        let path = self.heap.str_to_string(t);
        let holder = self.resolve_path(&path)?;
        let setter = is_setter.as_bool() == Some(true);
        let (atom, key_name) = self.install_key(key)?;
        let d = PropertyDescriptor {
            get: (!setter).then_some(f),
            set: setter.then_some(f),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        self.heap.define_own_property(holder, atom, &d)?;
        // Installed before (%ThrowTypeError% is every half of two accessors): keep the
        // first name.
        if self.heap.cell_flags(fo.offset()) & OBJ_BUILTIN != 0 {
            return Ok(());
        }
        let key_name = self.heap.str_to_string(key_name.as_string().expect("name"));
        if path == "Function.prototype" && (key_name == "caller" || key_name == "arguments") {
            // %ThrowTypeError% (10.2.4.1): an unnamed, frozen function shared by the
            // restricted accessors.
            let empty = Value::string(self.heap.atom_string(atoms::EMPTY)?);
            for (k, v) in [(atoms::LENGTH, Value::int(0)), (atoms::NAME, empty)] {
                let d = PropertyDescriptor::data(v, PropFlags::NONE);
                self.heap.define_own_property(fo, k, &d)?;
            }
            self.mark_builtin(fo, OBJ_BUILTIN);
            self.heap.prevent_extensions(fo);
            return Ok(());
        }
        let name = format!("{} {key_name}", if setter { "set" } else { "get" });
        let name = Value::string(self.heap.alloc_str(&name)?);
        self.name_and_length(fo, name, if setter { 1.0 } else { 0.0 })?;
        self.mark_builtin(fo, OBJ_BUILTIN);
        Ok(())
    }

    /// The object at a dotted path from the global object. A first segment `%Name%`
    /// names a realm intrinsic object (`%ArrayIteratorPrototype%`). The intrinsic
    /// prototypes are reachable as `X.prototype` even before their constructors exist.
    fn resolve_path(&mut self, path: &str) -> Result<Obj, Fault> {
        let unknown = || {
            Err(Fault::error(
                ErrorKind::InternalError,
                format!("%InstallMethod: no object at {path}"),
            ))
        };
        let realm_object = |name: &str| match name {
            "%IteratorPrototype%" => Some(Intrinsic::IteratorProto),
            "%ArrayIteratorPrototype%" => Some(Intrinsic::ArrayIteratorProto),
            "%StringIteratorPrototype%" => Some(Intrinsic::StringIteratorProto),
            "%GeneratorPrototype%" => Some(Intrinsic::GeneratorProto),
            "%AsyncIteratorPrototype%" => Some(Intrinsic::AsyncIteratorProto),
            "%AsyncGeneratorPrototype%" => Some(Intrinsic::AsyncGeneratorProto),
            "%AsyncFromSyncIteratorPrototype%" => Some(Intrinsic::AsyncFromSyncIteratorProto),
            "%MapIteratorPrototype%" => Some(Intrinsic::MapIteratorProto),
            "%SetIteratorPrototype%" => Some(Intrinsic::SetIteratorProto),
            "%RegExpStringIteratorPrototype%" => Some(Intrinsic::RegExpStringIteratorProto),
            "%IteratorHelperPrototype%" => Some(Intrinsic::IteratorHelperProto),
            "%WrapForValidIteratorPrototype%" => Some(Intrinsic::WrapForValidIteratorProto),
            "%TypedArray%" => Some(Intrinsic::TypedArrayCtor),
            // `X.prototype` of a constructor the prelude has not installed yet.
            _ => Intrinsic::prototype_of(name.strip_suffix(".prototype")?),
        };
        let mut parts = path.split('.').peekable();
        let mut o = intrinsic_obj(self.heap, Intrinsic::Global);
        if let Some(first) = parts.peek().copied()
            && first.starts_with('%')
        {
            let Some(i) = realm_object(first) else {
                return unknown();
            };
            o = intrinsic_obj(self.heap, i);
            parts.next();
        }
        for part in parts {
            let key = self.heap.atom_str(part)?;
            let next = self
                .heap
                .get_own_property(o, key)
                .and_then(|p| p.value())
                .and_then(|v| v.as_object());
            match next {
                Some(n) => o = n,
                None => match realm_object(path) {
                    Some(i) => return Ok(intrinsic_obj(self.heap, i)),
                    None => return unknown(),
                },
            }
        }
        Ok(o)
    }
}

impl Exec<'_> {
    /// `v` as an Array Iterator, for the `ArrayIterator*` leaves.
    fn array_iterator(&self, v: Value) -> Result<Obj, Fault> {
        match v.as_object() {
            Some(o) if self.heap.object_class(o) == ObjectClass::ArrayIterator => Ok(o),
            _ => type_error("Array Iterator object expected"),
        }
    }
}

impl Exec<'_> {
    /// `fast_step` for `call_intrinsic`: the cheap leaves the builtins call most (type
    /// checks, dense element access) and `%Call` of a JS closure, with the same results
    /// as `call_intrinsic`. False, with nothing changed, for anything else.
    #[inline(never)]
    pub(super) fn fast_intrinsic(&mut self) -> bool {
        let (id, argc) = (self.imm_u16(1) as u16, self.imm_u16(3));
        let base = self.sp - argc;
        let a0 = self.get(base);
        let v = match (id, argc) {
            (IS_NUMBER, 1) => Value::bool(a0.is_number()),
            (IS_OBJECT, 1) => Value::bool(a0.as_object().is_some()),
            (AS_NUMBER, 1) if a0.is_number() => a0,
            (AS_OBJ, 1) if a0.as_object().is_some() => a0,
            (GET, 2) => {
                let Some(v) = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Array)
                    .zip(index_key(self.get(base + 1)))
                    .and_then(|(o, i)| self.heap.get_element_fast(o, i))
                else {
                    return false;
                };
                v
            }
            (CREATE_DATA_PROPERTY_OR_THROW, 3) => {
                let Some((o, i)) = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Array)
                    .zip(index_key(self.get(base + 1)))
                else {
                    return false;
                };
                // Only a store that cannot fail or allocate: an existing dense element.
                if self.heap.get_element_fast(o, i).is_none() {
                    return false;
                }
                let v = self.get(base + 2);
                if !matches!(self.heap.set_element_fast(o, i, v), Ok(true)) {
                    return false;
                }
                Value::UNDEFINED
            }
            // `f this a1 .. an`, as in `call_intrinsic`.
            (CALL, 2..) => return self.fast_call_at(Op::CallIntrinsic, base, argc - 2),
            _ => return false,
        };
        let info = &OPS[Op::CallIntrinsic as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.sp = base;
        self.push(v);
        self.pc += info.size as u32;
        true
    }
}
