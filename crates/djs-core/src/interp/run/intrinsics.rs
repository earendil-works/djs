//! `call_intrinsic id, argc` (docs/compiler.md 7.3, D1270): the leaf and reentrant
//! intrinsics of `builtins/intrinsics.d.ts` in Rust, and calls of the JS builtins that
//! the prelude registered with `%InstallIntrinsic`.
//!
//! Arguments are on the stack (`a1 .. an`, like `call` without `f` and `this`); the
//! intrinsic replaces them with its result. Reentrant ones (D1173) finish in Rust or
//! push exactly one call frame whose result completes them.

use super::*;
use crate::builtins::{function, object};
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
const INSTALL_CONSTRUCTOR: u16 = id("InstallConstructor");
const INSTALL_ACCESSOR: u16 = id("InstallAccessor");
const INSTALL_VALUE: u16 = id("InstallValue");
const INSTALL_METHOD: u16 = id("InstallMethod");
const INSTALL_INTRINSIC: u16 = id("InstallIntrinsic");
/// The installed `%ToPrimitive`, used by the interpreter's slow paths.
pub(super) const TO_PRIMITIVE: u16 = id("ToPrimitive");

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
                } else if a0.is_nullish() {
                    return type_error("cannot convert to object");
                } else {
                    return type_error("not supported yet: primitive wrapper objects");
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
                let (Some(t), Some(k)) = (a0.as_string(), arg(self, 1).as_string()) else {
                    return bad_argument(info.name);
                };
                let path = self.heap.str_to_string(t);
                let holder = self.resolve_path(&path)?;
                let key = self.heap.intern(k)?;
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
            _ => {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("not supported yet: %{}", info.name),
                ));
            }
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
        let (Some(t), Some(k), Some(fo)) = (target.as_string(), key.as_string(), f.as_object())
        else {
            return bad_argument("InstallMethod");
        };
        let path = self.heap.str_to_string(t);
        let holder = self.resolve_path(&path)?;
        let key = self.heap.intern(k)?;
        realm::define_hidden(self.heap, holder, key, f)?;
        let ro = PropFlags::CONFIGURABLE;
        let name = PropertyDescriptor::data(Value::string(k), ro);
        self.heap.define_own_property(fo, atoms::NAME, &name)?;
        let len = PropertyDescriptor::data(conv::number(length.as_number().unwrap_or(0.0)), ro);
        self.heap.define_own_property(fo, atoms::LENGTH, &len)?;
        self.mark_builtin(fo, OBJ_BUILTIN);
        Ok(())
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
        let (Some(t), Some(k), Some(fo)) = (target.as_string(), key.as_string(), f.as_object())
        else {
            return bad_argument("InstallAccessor");
        };
        let path = self.heap.str_to_string(t);
        let holder = self.resolve_path(&path)?;
        let setter = is_setter.as_bool() == Some(true);
        let atom = self.heap.intern(k)?;
        let d = PropertyDescriptor {
            get: (!setter).then_some(f),
            set: setter.then_some(f),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        self.heap.define_own_property(holder, atom, &d)?;
        let name = format!(
            "{} {}",
            if setter { "set" } else { "get" },
            self.heap.str_to_string(k)
        );
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
            _ => None,
        };
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
