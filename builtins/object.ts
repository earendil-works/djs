// Object (ECMA-262 20.1) and property descriptors (6.2.6).

/**
 * Object ( [ value ] ), 20.1.1.1.
 * @constructor Object
 * @length 1
 */
function ObjectConstructor(value?: Value): Obj {
  if (value === undefined || value === null) {
    return ObjectCreate(GetPrototypeFromConstructor(new.target, "ObjectPrototype"));
  }
  return ToObject(value);
}

/**
 * ToPropertyDescriptor ( Obj ), 6.2.6.5. The result is a descriptor record: an object
 * without a prototype whose own data properties are the present fields.
 */
function ToPropertyDescriptor(attributes: Value): Obj {
  if (!IsObject(attributes)) {
    throw NewTypeError("not an object");
  }
  const o: Obj = AsObj(attributes);
  const desc: Obj = ObjectCreate(null);
  if (HasProperty(o, "enumerable")) {
    CreateDataPropertyOrThrow(desc, "enumerable", ToBoolean(Get(o, "enumerable")));
  }
  if (HasProperty(o, "configurable")) {
    CreateDataPropertyOrThrow(desc, "configurable", ToBoolean(Get(o, "configurable")));
  }
  const hasValue: boolean = HasProperty(o, "value");
  if (hasValue) {
    CreateDataPropertyOrThrow(desc, "value", Get(o, "value"));
  }
  const hasWritable: boolean = HasProperty(o, "writable");
  if (hasWritable) {
    CreateDataPropertyOrThrow(desc, "writable", ToBoolean(Get(o, "writable")));
  }
  const hasGet: boolean = HasProperty(o, "get");
  if (hasGet) {
    const getter: Value = Get(o, "get");
    if (!IsCallable(getter) && getter !== undefined) {
      throw NewTypeError("invalid getter");
    }
    CreateDataPropertyOrThrow(desc, "get", getter);
  }
  const hasSet: boolean = HasProperty(o, "set");
  if (hasSet) {
    const setter: Value = Get(o, "set");
    if (!IsCallable(setter) && setter !== undefined) {
      throw NewTypeError("invalid setter");
    }
    CreateDataPropertyOrThrow(desc, "set", setter);
  }
  if ((hasGet || hasSet) && (hasValue || hasWritable)) {
    throw NewTypeError("cannot have setter/getter and value or writable");
  }
  return desc;
}

/**
 * DefinePropertyOrThrow ( O, P, desc ), 7.3.8, with a descriptor record. An array's
 * `length` value goes through ArraySetLength's conversions first (10.4.2.4).
 */
function DefinePropertyOrThrow(o: Obj, key: Value, desc: Obj): void {
  if (IsArray(o) && key === "length" && HasOwnProperty(desc, "value")) {
    const v: Value = Get(desc, "value");
    const newLen: f64 = ToNumber(v);
    const numberLen: f64 = ToNumber(v);
    if (!(newLen >= 0 && newLen <= 4294967295 && ToIntegerOrInfinityNumber(newLen) === newLen)) {
      throw NewRangeError("invalid array length");
    }
    if (newLen !== numberLen) {
      throw NewRangeError("invalid array length");
    }
    CreateDataPropertyOrThrow(desc, "value", newLen);
  }
  if (!DefineOwnPropertyFromRecord(o, key, desc)) {
    if (OwnPropertyAttributes(o, key) < 0) {
      throw NewTypeError("object is not extensible");
    }
    throw NewTypeError("property is not configurable");
  }
}

/**
 * Object.defineProperty ( O, P, Attributes ), 20.1.2.4.
 * @builtin Object.defineProperty
 */
function ObjectDefineProperty(o: Value, p: Value, attributes: Value): Value {
  if (!IsObject(o)) {
    throw NewTypeError("not an object");
  }
  const key: Value = ToPropertyKey(p);
  const desc: Obj = ToPropertyDescriptor(attributes);
  DefinePropertyOrThrow(AsObj(o), key, desc);
  return o;
}

/**
 * Object.getOwnPropertyDescriptor ( O, P ), 20.1.2.8.
 * @builtin Object.getOwnPropertyDescriptor
 */
function ObjectGetOwnPropertyDescriptor(o: Value, p: Value): Value {
  const obj: Obj = ToObject(o);
  const key: Value = ToPropertyKey(p);
  return GetOwnPropertyDescriptorObject(obj, key);
}

/**
 * Object.getOwnPropertyNames ( O ), 20.1.2.10.
 * @builtin Object.getOwnPropertyNames
 */
function ObjectGetOwnPropertyNames(o: Value): Obj {
  return OwnPropertyKeys(ToObject(o), 0);
}

/**
 * Object.prototype.hasOwnProperty ( V ), 20.1.3.2.
 * @builtin Object.prototype.hasOwnProperty
 */
function ObjectPrototypeHasOwnProperty(this: Value, v: Value): boolean {
  const key: Value = ToPropertyKey(v);
  return HasOwnProperty(ToObject(this), key);
}

/**
 * Object.prototype.propertyIsEnumerable ( V ), 20.1.3.4.
 * @builtin Object.prototype.propertyIsEnumerable
 */
function ObjectPrototypePropertyIsEnumerable(this: Value, v: Value): boolean {
  const key: Value = ToPropertyKey(v);
  const attrs: i32 = OwnPropertyAttributes(ToObject(this), key);
  return attrs >= 0 && (attrs & 2) !== 0;
}
