//! `call_intrinsic id, argc` (docs/compiler.md 7.3, D1270): the leaf and reentrant
//! intrinsics of `builtins/intrinsics.d.ts` in Rust, and calls of the JS builtins that
//! the prelude registered with `%InstallIntrinsic`.
//!
//! Arguments are on the stack (`a1 .. an`, like `call` without `f` and `this`); the
//! intrinsic replaces them with its result. Reentrant ones (D1173) finish in Rust or
//! push exactly one call frame whose result completes them.

use super::*;
use crate::builtins::{function, number, object};
use crate::bytecode::intrinsics::{INTRINSICS, IntrinsicKind, intrinsic, intrinsic_id};
use crate::heap::cells::flags::{OBJ_BUILTIN, OBJ_BUILTIN_CTOR};
use crate::heap::{ObjectClass, atoms};

/// The id of `name`, at compile time (a typo fails the build).
const fn id(name: &str) -> u16 {
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
const NUMBER_TO_INT32: u16 = id("NumberToInt32");
const NUMBER_TO_UINT32: u16 = id("NumberToUint32");
const MATH_RANDOM: u16 = id("MathRandom");
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
const INSTALL_CONSTRUCTOR: u16 = id("InstallConstructor");
const INSTALL_ACCESSOR: u16 = id("InstallAccessor");
const INSTALL_VALUE: u16 = id("InstallValue");
const INSTALL_METHOD: u16 = id("InstallMethod");
const INSTALL_INTRINSIC: u16 = id("InstallIntrinsic");
/// The installed `%ToPrimitive`, used by the interpreter's slow paths.
pub(super) const TO_PRIMITIVE: u16 = id("ToPrimitive");

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
        F64_FROUND => x as f32 as f64,
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

fn bad_argument<T>(name: &str) -> Result<T, Fault> {
    Err(Fault::Error(
        ErrorKind::InternalError,
        format!("bad argument to %{name}"),
    ))
}

impl Exec<'_> {
    /// Executes `call_intrinsic id, argc`; `next` is the following instruction.
    pub(super) fn call_intrinsic(&mut self, id: u16, argc: u32, next: u32) -> Step {
        let Some(info) = intrinsic(id) else {
            return Err(Fault::Error(
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
            let f = realm::installed_intrinsic(self.heap, id);
            if f.is_undefined() {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("%{} is not installed", info.name),
                ));
            }
            // `a1 .. an` becomes `f undefined a1 .. an`.
            self.ensure(self.sp + 2)?;
            for i in (0..argc).rev() {
                let v = self.get(base + i);
                self.set(base + 2 + i, v);
            }
            self.set(base, f);
            self.set(base + 1, Value::UNDEFINED);
            self.sp += 2;
            return self.call(base, argc, Continuation::PushResult, next);
        }
        let a0 = arg(self, 0);
        let v = match id {
            IS_OBJECT => Value::bool(a0.as_object().is_some()),
            IS_CALLABLE => Value::bool(self.is_callable(a0)),
            IS_CONSTRUCTOR => Value::bool(a0.as_object().is_some_and(|o| self.is_constructor(o))),
            IS_ARRAY => Value::bool(
                a0.as_object()
                    .is_some_and(|o| self.heap.object_class(o) == ObjectClass::Array),
            ),
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
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                match self.lookup(a0, key)? {
                    Ok(v) => v,
                    Err(getter) => {
                        self.sp = base;
                        self.pc = next;
                        return self.call_accessor(getter, a0, None);
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
                Value::bool(self.heap.has_property(o, key))
            }
            CREATE_DATA_PROPERTY_OR_THROW => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(info.name);
                };
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
                    return Err(Fault::Error(
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
            NEW_TYPE_ERROR | NEW_RANGE_ERROR => {
                let msg = match a0.as_string() {
                    Some(s) => self.heap.str_to_string(s),
                    None => return bad_argument(info.name),
                };
                let kind = if id == NEW_TYPE_ERROR {
                    ErrorKind::TypeError
                } else {
                    ErrorKind::RangeError
                };
                self.new_error_here(kind, &msg)?
            }
            NEW_NOT_CONSTRUCTOR_ERROR => {
                let msg = self.not_a_constructor_message(a0);
                self.new_error_here(ErrorKind::TypeError, &msg)?
            }
            WELL_KNOWN_SYMBOL => {
                let name = a0
                    .as_string()
                    .map(|s| self.heap.str_to_string(s))
                    .unwrap_or_default();
                let atom = match name.as_str() {
                    "iterator" => atoms::SYM_ITERATOR,
                    "asyncIterator" => atoms::SYM_ASYNC_ITERATOR,
                    "hasInstance" => atoms::SYM_HAS_INSTANCE,
                    "toPrimitive" => atoms::SYM_TO_PRIMITIVE,
                    "toStringTag" => atoms::SYM_TO_STRING_TAG,
                    "species" => atoms::SYM_SPECIES,
                    "isConcatSpreadable" => atoms::SYM_IS_CONCAT_SPREADABLE,
                    "unscopables" => atoms::SYM_UNSCOPABLES,
                    "match" => atoms::SYM_MATCH,
                    "matchAll" => atoms::SYM_MATCH_ALL,
                    "replace" => atoms::SYM_REPLACE,
                    "search" => atoms::SYM_SEARCH,
                    "split" => atoms::SYM_SPLIT,
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
                        let tag = object::class_tag(self.heap, o);
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
                Value::bool(self.heap.set_prototype_of(o, proto.as_object()))
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
                    | ObjectClass::Symbol => self.heap.object_slot(o, 0),
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
            NUMBER_TO_INT32 => Value::int(conv::to_int32(a0.as_number().unwrap_or(f64::NAN))),
            NUMBER_TO_UINT32 => {
                conv::number(number::to_uint32(a0.as_number().unwrap_or(f64::NAN)) as f64)
            }
            MATH_RANDOM => Value::number(number::random(self.heap)),
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
            INSTALL_INTRINSIC => {
                let name = a0
                    .as_string()
                    .map(|s| self.heap.str_to_string(s))
                    .unwrap_or_default();
                let f = arg(self, 1);
                match intrinsic_id(&name) {
                    Some(i) if intrinsic(i).unwrap().kind == IntrinsicKind::Builtin => {
                        realm::install_intrinsic(self.heap, i, f);
                    }
                    _ => return bad_argument(info.name),
                }
                Value::UNDEFINED
            }
            _ => match math_leaf(
                id,
                a0.as_number().unwrap_or(f64::NAN),
                arg(self, 1).as_number().unwrap_or(f64::NAN),
            ) {
                Some(r) => conv::number(r),
                None => {
                    return Err(Fault::Error(
                        ErrorKind::InternalError,
                        format!("not supported yet: %{}", info.name),
                    ));
                }
            },
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }

    fn is_callable(&self, v: Value) -> bool {
        v.as_object()
            .is_some_and(|o| self.heap.object_class(o).is_callable())
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
    fn mark_builtin(&mut self, fo: Obj, flag: u16) {
        let flags = self.heap.cell_flags(fo.offset());
        self.heap.set_cell_flags(fo.offset(), flags | flag);
        if flag & OBJ_BUILTIN_CTOR == 0 {
            self.heap
                .remove_own_property_unchecked(fo, atoms::PROTOTYPE);
        }
    }

    /// `name` and `length` of an installed builtin (configurable only).
    fn name_and_length(&mut self, fo: Obj, name: Value, length: f64) -> Result<(), Fault> {
        let ro = PropFlags::CONFIGURABLE;
        let d = PropertyDescriptor::data(name, ro);
        self.heap.define_own_property(fo, atoms::NAME, &d)?;
        let d = PropertyDescriptor::data(conv::number(length), ro);
        self.heap.define_own_property(fo, atoms::LENGTH, &d)?;
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
        if let Some(p) = parent {
            self.heap.set_prototype_of(fo, Some(p));
        }
        let key = self.heap.intern(k)?;
        realm::define_hidden(self.heap, holder, key, f)?;
        let d = PropertyDescriptor::data(Value::object(proto), PropFlags::NONE);
        self.heap.define_own_property(fo, atoms::PROTOTYPE, &d)?;
        realm::define_hidden(self.heap, proto, atoms::CONSTRUCTOR, f)?;
        self.name_and_length(fo, Value::string(k), length.as_number().unwrap_or(0.0))?;
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
        let name = format!("{} {key_name}", if setter { "set" } else { "get" });
        let name = Value::string(self.heap.alloc_str(&name)?);
        self.name_and_length(fo, name, if setter { 1.0 } else { 0.0 })?;
        self.mark_builtin(fo, OBJ_BUILTIN);
        Ok(())
    }

    /// The object at a dotted path from the global object. The intrinsic prototypes are
    /// reachable as `X.prototype` even before their constructors exist.
    fn resolve_path(&mut self, path: &str) -> Result<Obj, Fault> {
        let intrinsic_path = match path {
            "Object.prototype" => Some(Intrinsic::ObjectProto),
            "Function.prototype" => Some(Intrinsic::FunctionProto),
            "Array.prototype" => Some(Intrinsic::ArrayProto),
            "Error.prototype" => Some(Intrinsic::ErrorProto),
            "%IteratorPrototype%" => Some(Intrinsic::IteratorProto),
            "%ArrayIteratorPrototype%" => Some(Intrinsic::ArrayIteratorProto),
            _ => None,
        };
        // `%Name%` paths name realm intrinsics directly.
        if path.starts_with('%') {
            return match intrinsic_path {
                Some(i) => Ok(intrinsic_obj(self.heap, i)),
                None => Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("%InstallMethod: unknown intrinsic object {path}"),
                )),
            };
        }
        let mut o = intrinsic_obj(self.heap, Intrinsic::Global);
        for part in path.split('.') {
            let key = self.heap.atom_str(part)?;
            match self
                .heap
                .get_own_property(o, key)
                .and_then(|p| p.value())
                .and_then(|v| v.as_object())
            {
                Some(next) => o = next,
                None => match intrinsic_path {
                    Some(i) => return Ok(intrinsic_obj(self.heap, i)),
                    None => {
                        return Err(Fault::Error(
                            ErrorKind::InternalError,
                            format!("%InstallMethod: no object at {path}"),
                        ));
                    }
                },
            }
        }
        Ok(o)
    }
}
