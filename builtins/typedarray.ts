// %TypedArray% and the typed array constructors (ECMA-262 23.2), and Uint8Array's base64
// and hex methods. Elements live in the buffer; the integer-indexed exotic behavior is in
// heap/typed.rs (D1390). Kinds: 0 Int8, 1 Uint8, 2 Uint8Clamped, 3 Int16, 4 Uint16,
// 5 Int32, 6 Uint32, 7 Float16, 8 Float32, 9 Float64, 10 BigInt64, 11 BigUint64.

/** The element size of a kind. */
function ElementSize(kind: i32): f64 {
  if (kind <= 2) {
    return 1;
  }
  if (kind <= 4 || kind === 7) {
    return 2;
  }
  if (kind <= 6 || kind === 8) {
    return 4;
  }
  return 8;
}

/** The constructor name of a kind (for the realm's %Name% defaults). */
function KindName(kind: i32): Str {
  if (kind === 0) {
    return "Int8Array";
  }
  if (kind === 1) {
    return "Uint8Array";
  }
  if (kind === 2) {
    return "Uint8ClampedArray";
  }
  if (kind === 3) {
    return "Int16Array";
  }
  if (kind === 4) {
    return "Uint16Array";
  }
  if (kind === 5) {
    return "Int32Array";
  }
  if (kind === 6) {
    return "Uint32Array";
  }
  if (kind === 7) {
    return "Float16Array";
  }
  if (kind === 8) {
    return "Float32Array";
  }
  if (kind === 9) {
    return "Float64Array";
  }
  if (kind === 10) {
    return "BigInt64Array";
  }
  return "BigUint64Array";
}

/** The kind of a typed array. */
function KindOf(o: Obj): i32 {
  return F64ToI32(AsNumber(TypedArrayField(o, 0)));
}

/** The length of a typed array (0 if out of bounds or detached). */
function TALength(o: Obj): f64 {
  return AsNumber(TypedArrayField(o, 3));
}

/** Out of bounds or detached. */
function TAOutOfBounds(o: Obj): boolean {
  return AsBool(TypedArrayField(o, 5));
}

/**
 * ValidateTypedArray ( O, order ), 23.2.4.4: the typed array, TypeError if it is not
 * one or is out of bounds/detached.
 */
function ValidateTypedArray(v: Value): Obj {
  TypedArrayField(v, 0);
  const o: Obj = AsObj(v);
  if (TAOutOfBounds(o)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  return o;
}

/** BigInt typed arrays need BigInt, which djs does not have yet. */
function RequireNumberKind(kind: i32): void {
  if (kind >= 10) {
    throw NewTypeError("not supported yet: BigInt typed arrays");
  }
}

/** AllocateTypedArray ( ..., length ) with a fresh buffer of length elements. */
function AllocateTypedArrayOfLength(kind: i32, proto: Obj, length: f64): Obj {
  const size: f64 = ElementSize(kind);
  if (length * size > 2147483648) {
    throw NewRangeError("invalid array buffer length");
  }
  const buffer: Obj = ArrayBufferCreate(RealmIntrinsic("ArrayBufferPrototype"), length * size, undefined);
  return TypedArrayCreate(proto, kind, buffer, 0, length);
}

/** InitializeTypedArrayFromArrayBuffer, 23.2.5.1.3 (qjs's messages). */
function TypedArrayFromBuffer(kind: i32, proto: Obj, buffer: Obj, byteOffset: Value, length: Value): Obj {
  const size: f64 = ElementSize(kind);
  const offset: f64 = ToIndex(byteOffset);
  if (offset % size !== 0) {
    throw NewRangeError("invalid offset");
  }
  let newLength: f64 = 0;
  if (length !== undefined) {
    newLength = ToIndex(length);
  }
  if (IsDetached(buffer)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  const bufferByteLength: f64 = BufferLength(buffer);
  if (length === undefined && AsBool(BufferInfo(buffer, 2))) {
    if (offset > bufferByteLength) {
      throw NewRangeError("invalid offset");
    }
    return TypedArrayCreate(proto, kind, buffer, offset, -1);
  }
  if (length === undefined) {
    if (bufferByteLength % size !== 0) {
      throw NewRangeError("invalid length");
    }
    if (bufferByteLength - offset < 0) {
      throw NewRangeError("invalid offset");
    }
    return TypedArrayCreate(proto, kind, buffer, offset, (bufferByteLength - offset) / size);
  }
  if (offset + newLength * size > bufferByteLength) {
    throw NewRangeError("invalid length");
  }
  return TypedArrayCreate(proto, kind, buffer, offset, newLength);
}

/** A typed array of kind with the values of a list (an array) or array-like. */
function TypedArrayFromValues(kind: i32, proto: Obj, values: Obj): Obj {
  const len: f64 = LengthOfArrayLike(values);
  const o: Obj = AllocateTypedArrayOfLength(kind, proto, len);
  for (let k: f64 = 0; k < len; k++) {
    TypedArraySet(o, k, ToNumber(Get(values, k)));
  }
  return o;
}

/** The constructors' shared steps (23.2.5.1). */
function TypedArrayConstruct(newTarget: Value, kind: i32, args: Obj): Obj {
  if (newTarget === undefined) {
    throw NewTypeError("must be called with new");
  }
  RequireNumberKind(kind);
  const proto: Obj = GetPrototypeFromConstructor(newTarget, KindName(kind) + "Prototype");
  const n: f64 = LengthOfArrayLike(args);
  const first: Value = n > 0 ? Get(args, 0) : undefined;
  if (!IsObject(first)) {
    return AllocateTypedArrayOfLength(kind, proto, ToIndex(first));
  }
  const src: Obj = AsObj(first);
  if (IsTypedArray(src)) {
    if (TAOutOfBounds(src)) {
      throw NewTypeError("ArrayBuffer is detached");
    }
    if ((KindOf(src) >= 10) !== (kind >= 10)) {
      throw NewTypeError("cannot mix BigInt and other types");
    }
    const len: f64 = TALength(src);
    const o: Obj = AllocateTypedArrayOfLength(kind, proto, len);
    TypedArrayCopy(src, 0, o, 0, len);
    return o;
  }
  if (BufferInfoOrUndefined(src)) {
    return TypedArrayFromBuffer(kind, proto, src, n > 1 ? Get(args, 1) : undefined, n > 2 ? Get(args, 2) : undefined);
  }
  const usingIterator: Value = GetMethod(src, WellKnownSymbol("iterator"));
  if (usingIterator !== undefined) {
    const list: Obj = ArrayCreate(0);
    const target: Obj = ArrayFromIterator(list, src, usingIterator, undefined, undefined);
    return TypedArrayFromValues(kind, proto, target);
  }
  return TypedArrayFromValues(kind, proto, src);
}

/** Whether o is an ArrayBuffer. */
function BufferInfoOrUndefined(o: Obj): boolean {
  try {
    BufferInfo(o, 0);
    return true;
  } catch (e) {
    return false;
  }
}

/**
 * %TypedArray% ( ), 23.2.1.1: abstract.
 * @constructor %TypedArray%
 */
function TypedArrayAbstract(): Obj {
  throw NewTypeError("cannot be called");
}

/**
 * get %TypedArray% [ %Symbol.species% ], 23.2.2.4.
 * @getter %TypedArray%[Symbol.species]
 */
function TypedArraySpecies(this: Value): Value {
  return this;
}

/** TypedArrayCreateFromConstructor with a length argument, validated (23.2.4.2). */
function TypedArrayCreateWithLength(c: Value, length: f64): Obj {
  const created: Obj = Construct(c, length);
  ValidateTypedArray(created);
  if (KindOf(created) >= 10) {
    throw NewTypeError("not supported yet: BigInt typed arrays");
  }
  if (TALength(created) < length) {
    throw NewTypeError("TypedArray length is too small");
  }
  return created;
}

/** TypedArraySpeciesCreate ( exemplar, « length » ), 23.2.4.1. */
function TypedArraySpeciesCreate(exemplar: Obj, length: f64): Obj {
  const c: Value = SpeciesConstructor(exemplar, RealmConstructor(KindName(KindOf(exemplar))));
  return TypedArrayCreateWithLength(c, length);
}

/**
 * %TypedArray%.from ( source [ , mapfn [ , thisArg ] ] ), 23.2.2.1.
 * @builtin %TypedArray%.from
 */
function TypedArrayFrom(this: Value, source: Value, mapfn?: Value, thisArg?: Value): Obj {
  if (!IsConstructor(this)) {
    throw NewTypeError("not a constructor");
  }
  const mapping: boolean = mapfn !== undefined;
  if (mapping && !IsCallable(mapfn)) {
    throw NewTypeError("not a function");
  }
  const usingIterator: Value = GetMethod(source, WellKnownSymbol("iterator"));
  let values: Obj = ArrayCreate(0);
  if (usingIterator !== undefined) {
    values = ArrayFromIterator(values, source, usingIterator, undefined, undefined);
  } else {
    values = ToObject(source);
  }
  const len: f64 = LengthOfArrayLike(values);
  const target: Obj = TypedArrayCreateWithLength(this, len);
  for (let k: f64 = 0; k < len; k++) {
    const kValue: Value = Get(values, k);
    const mapped: Value = mapping ? Call(mapfn, thisArg, kValue, k) : kValue;
    Set(target, k, mapped);
  }
  return target;
}

/**
 * %TypedArray%.of ( ...items ), 23.2.2.2.
 * @builtin %TypedArray%.of
 */
function TypedArrayOf(this: Value, ...items: Value[]): Obj {
  if (!IsConstructor(this)) {
    throw NewTypeError("not a constructor");
  }
  const len: f64 = LengthOfArrayLike(items);
  const target: Obj = TypedArrayCreateWithLength(this, len);
  for (let k: f64 = 0; k < len; k++) {
    Set(target, k, Get(items, k));
  }
  return target;
}

/**
 * get %TypedArray%.prototype.buffer, 23.2.3.2.
 * @getter %TypedArray%.prototype.buffer
 */
function TypedArrayPrototypeBuffer(this: Value): Value {
  return TypedArrayField(this, 1);
}

/**
 * get %TypedArray%.prototype.byteLength, 23.2.3.3.
 * @getter %TypedArray%.prototype.byteLength
 */
function TypedArrayPrototypeByteLength(this: Value): Value {
  return TypedArrayField(this, 4);
}

/**
 * get %TypedArray%.prototype.byteOffset, 23.2.3.4.
 * @getter %TypedArray%.prototype.byteOffset
 */
function TypedArrayPrototypeByteOffset(this: Value): Value {
  return TypedArrayField(this, 2);
}

/**
 * get %TypedArray%.prototype.length, 23.2.3.21.
 * @getter %TypedArray%.prototype.length
 */
function TypedArrayPrototypeLength(this: Value): Value {
  return TypedArrayField(this, 3);
}

/**
 * get %TypedArray%.prototype [ %Symbol.toStringTag% ], 23.2.3.38.
 * @getter %TypedArray%.prototype[Symbol.toStringTag]
 */
function TypedArrayPrototypeToStringTag(this: Value): Value {
  if (!IsTypedArray(this)) {
    return undefined;
  }
  return KindName(KindOf(AsObj(this)));
}

/**
 * %TypedArray%.prototype.set ( source [ , offset ] ), 23.2.3.26.
 * @builtin %TypedArray%.prototype.set
 */
function TypedArrayPrototypeSet(this: Value, source: Value, offset?: Value): void {
  TypedArrayField(this, 0);
  const target: Obj = AsObj(this);
  const targetOffset: f64 = ToIntegerOrInfinity(offset);
  if (targetOffset < 0) {
    throw NewRangeError("invalid array length");
  }
  if (TAOutOfBounds(target)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  const targetLength: f64 = TALength(target);
  if (IsTypedArray(source)) {
    const src: Obj = AsObj(source);
    if (TAOutOfBounds(src)) {
      throw NewTypeError("ArrayBuffer is detached");
    }
    const srcLength: f64 = TALength(src);
    if (targetOffset === 1 / 0 || srcLength + targetOffset > targetLength) {
      throw NewRangeError("invalid array length");
    }
    if ((KindOf(src) >= 10) !== (KindOf(target) >= 10)) {
      throw NewTypeError("cannot mix BigInt and other types");
    }
    TypedArrayCopy(src, 0, target, targetOffset, srcLength);
    return;
  }
  const src: Obj = ToObject(source);
  const srcLength: f64 = LengthOfArrayLike(src);
  if (targetOffset === 1 / 0 || srcLength + targetOffset > targetLength) {
    throw NewRangeError("invalid array length");
  }
  for (let k: f64 = 0; k < srcLength; k++) {
    const value: f64 = ToNumber(Get(src, k));
    TypedArraySet(target, targetOffset + k, value);
  }
}

/**
 * %TypedArray%.prototype.subarray ( start, end ), 23.2.3.31.
 * @builtin %TypedArray%.prototype.subarray
 */
function TypedArrayPrototypeSubarray(this: Value, start: Value, end: Value): Obj {
  TypedArrayField(this, 0);
  const o: Obj = AsObj(this);
  const buffer: Obj = AsObj(TypedArrayField(o, 1));
  const srcLength: f64 = TALength(o);
  const kind: i32 = KindOf(o);
  const size: f64 = ElementSize(kind);
  const beginIndex: f64 = RelativeIndex(start, srcLength, 0);
  const tracking: boolean = AsBool(TypedArrayField(o, 6));
  const beginByteOffset: f64 = AsNumber(TypedArrayField(o, 2)) + beginIndex * size;
  const c: Value = SpeciesConstructor(o, RealmConstructor(KindName(kind)));
  let created: Value = undefined;
  if (tracking && end === undefined) {
    created = Construct(c, buffer, beginByteOffset);
  } else {
    const endIndex: f64 = RelativeIndex(end, srcLength, srcLength);
    const newLength: f64 = endIndex > beginIndex ? endIndex - beginIndex : 0;
    created = Construct(c, buffer, beginByteOffset, newLength);
  }
  ValidateTypedArray(created);
  return AsObj(created);
}

/**
 * %TypedArray%.prototype.slice ( start, end ), 23.2.3.27.
 * @builtin %TypedArray%.prototype.slice
 */
function TypedArrayPrototypeSlice(this: Value, start: Value, end: Value): Obj {
  const o: Obj = ValidateTypedArray(this);
  const srcLength: f64 = TALength(o);
  const startIndex: f64 = RelativeIndex(start, srcLength, 0);
  const endIndex: f64 = RelativeIndex(end, srcLength, srcLength);
  const count: f64 = endIndex > startIndex ? endIndex - startIndex : 0;
  const a: Obj = TypedArraySpeciesCreate(o, count);
  if (count > 0) {
    if (TAOutOfBounds(o)) {
      throw NewTypeError("ArrayBuffer is detached");
    }
    const len: f64 = TALength(o);
    const fin: f64 = endIndex < len ? endIndex : len;
    const n: f64 = fin > startIndex ? fin - startIndex : 0;
    if (KindOf(a) === KindOf(o)) {
      TypedArrayCopy(o, startIndex, a, 0, n);
    } else {
      for (let k: f64 = 0; k < n; k++) {
        Set(a, k, TypedArrayGet(o, startIndex + k));
      }
    }
  }
  return a;
}

/**
 * %TypedArray%.prototype.fill ( value [ , start [ , end ] ] ), 23.2.3.9.
 * @builtin %TypedArray%.prototype.fill
 */
function TypedArrayPrototypeFill(this: Value, value: Value, start?: Value, end?: Value): Obj {
  const o: Obj = ValidateTypedArray(this);
  const len: f64 = TALength(o);
  const n: f64 = ToNumber(value);
  const k: f64 = RelativeIndex(start, len, 0);
  const fin: f64 = RelativeIndex(end, len, len);
  if (TAOutOfBounds(o)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  const len2: f64 = TALength(o);
  const last: f64 = fin < len2 ? fin : len2;
  for (let i: f64 = k; i < last; i++) {
    TypedArraySet(o, i, n);
  }
  return o;
}

// ---- Uint8Array base64 and hex (ES2026) ------------------------------------------------

/** ValidateUint8Array ( ta ). */
function ValidateUint8Array(v: Value): Obj {
  if (!IsTypedArray(v) || KindOf(AsObj(v)) !== 1) {
    throw NewTypeError("not a Uint8Array");
  }
  return AsObj(v);
}

/** GetOptionsObject ( options ). */
function GetOptionsObject(options: Value): Value {
  if (options === undefined) {
    return undefined;
  }
  if (!IsObject(options)) {
    throw NewTypeError("options must be an object");
  }
  return options;
}

/** The alphabet option: true for base64url. */
function Base64Alphabet(opts: Value): boolean {
  if (opts === undefined) {
    return false;
  }
  const alphabet: Value = Get(AsObj(opts), "alphabet");
  if (alphabet === undefined || alphabet === "base64") {
    return false;
  }
  if (alphabet === "base64url") {
    return true;
  }
  throw NewTypeError("invalid alphabet");
}

/** The lastChunkHandling option: 0 loose, 1 strict, 2 stop-before-partial. */
function LastChunkHandling(opts: Value): i32 {
  if (opts === undefined) {
    return 0;
  }
  const h: Value = Get(AsObj(opts), "lastChunkHandling");
  if (h === undefined || h === "loose") {
    return 0;
  }
  if (h === "strict") {
    return 1;
  }
  if (h === "stop-before-partial") {
    return 2;
  }
  throw NewTypeError("invalid lastChunkHandling option");
}

/** A Uint8Array holding the bytes of a decode record. */
function Uint8ArrayFromRecord(rec: Obj): Obj {
  const bytes: Obj = AsObj(Get(rec, "bytes"));
  const len: f64 = BufferLength(bytes);
  return TypedArrayCreate(RealmIntrinsic("Uint8ArrayPrototype"), 1, bytes, 0, len);
}

/**
 * Uint8Array.fromBase64 ( string [ , options ] ).
 * @builtin Uint8Array.fromBase64
 */
function Uint8ArrayFromBase64(string: Value, options?: Value): Obj {
  if (!IsString(string)) {
    throw NewTypeError("expected string");
  }
  const opts: Value = GetOptionsObject(options);
  const url: boolean = Base64Alphabet(opts);
  const last: i32 = LastChunkHandling(opts);
  const rec: Obj = Base64Decode(AsStr(string), url, last, 9007199254740991);
  if (AsBool(Get(rec, "error"))) {
    throw NewSyntaxError("invalid base64 string");
  }
  return Uint8ArrayFromRecord(rec);
}

/**
 * Uint8Array.fromHex ( string ).
 * @builtin Uint8Array.fromHex
 */
function Uint8ArrayFromHex(string: Value): Obj {
  if (!IsString(string)) {
    throw NewTypeError("expected string");
  }
  const rec: Obj = HexDecode(AsStr(string), 9007199254740991);
  if (AsBool(Get(rec, "error"))) {
    throw NewSyntaxError("invalid hex string");
  }
  return Uint8ArrayFromRecord(rec);
}

/**
 * Uint8Array.prototype.toBase64 ( [ options ] ).
 * @builtin Uint8Array.prototype.toBase64
 */
function Uint8ArrayPrototypeToBase64(this: Value, options?: Value): Str {
  const o: Obj = ValidateUint8Array(this);
  const opts: Value = GetOptionsObject(options);
  const url: boolean = Base64Alphabet(opts);
  let omit: boolean = false;
  if (opts !== undefined) {
    omit = ToBoolean(Get(AsObj(opts), "omitPadding"));
  }
  return Base64Encode(o, url, omit);
}

/**
 * Uint8Array.prototype.toHex ( ).
 * @builtin Uint8Array.prototype.toHex
 */
function Uint8ArrayPrototypeToHex(this: Value): Str {
  return HexEncode(ValidateUint8Array(this));
}

/** Writes a decode record's bytes into a Uint8Array; `{ read, written }`. */
function WriteDecoded(into: Obj, rec: Obj, isHex: boolean): Obj {
  const bytes: Obj = AsObj(Get(rec, "bytes"));
  const written: f64 = BufferLength(bytes);
  BufferCopy(bytes, 0, AsObj(TypedArrayField(into, 1)), AsNumber(TypedArrayField(into, 2)), written);
  if (AsBool(Get(rec, "error"))) {
    throw NewSyntaxError(isHex ? "invalid hex string" : "invalid base64 string");
  }
  const result: Obj = ObjectCreate(RealmIntrinsic("ObjectPrototype"));
  CreateDataPropertyOrThrow(result, "read", Get(rec, "read"));
  CreateDataPropertyOrThrow(result, "written", written);
  return result;
}

/**
 * Uint8Array.prototype.setFromBase64 ( string [ , options ] ).
 * @builtin Uint8Array.prototype.setFromBase64
 */
function Uint8ArrayPrototypeSetFromBase64(this: Value, string: Value, options?: Value): Obj {
  const into: Obj = ValidateUint8Array(this);
  if (!IsString(string)) {
    throw NewTypeError("expected string");
  }
  const opts: Value = GetOptionsObject(options);
  const url: boolean = Base64Alphabet(opts);
  const last: i32 = LastChunkHandling(opts);
  if (TAOutOfBounds(into)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  const rec: Obj = Base64Decode(AsStr(string), url, last, TALength(into));
  return WriteDecoded(into, rec, false);
}

/**
 * Uint8Array.prototype.setFromHex ( string ).
 * @builtin Uint8Array.prototype.setFromHex
 */
function Uint8ArrayPrototypeSetFromHex(this: Value, string: Value): Obj {
  const into: Obj = ValidateUint8Array(this);
  if (!IsString(string)) {
    throw NewTypeError("expected string");
  }
  if (TAOutOfBounds(into)) {
    throw NewTypeError("ArrayBuffer is detached");
  }
  const rec: Obj = HexDecode(AsStr(string), TALength(into));
  return WriteDecoded(into, rec, true);
}

/**
 * Int8Array ( ...args ), 23.2.5.1.
 * @constructor Int8Array
 * @extends %TypedArray%
 * @length 3
 */
function Int8ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 0, args);
}

/**
 * Int8Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Int8Array.BYTES_PER_ELEMENT
 */
function Int8ArrayBytesPerElement(): f64 {
  return 1;
}

/**
 * Int8Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Int8Array.prototype.BYTES_PER_ELEMENT
 */
function Int8ArrayPrototypeBytesPerElement(): f64 {
  return 1;
}

/**
 * Uint8Array ( ...args ), 23.2.5.1.
 * @constructor Uint8Array
 * @extends %TypedArray%
 * @length 3
 */
function Uint8ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 1, args);
}

/**
 * Uint8Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Uint8Array.BYTES_PER_ELEMENT
 */
function Uint8ArrayBytesPerElement(): f64 {
  return 1;
}

/**
 * Uint8Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Uint8Array.prototype.BYTES_PER_ELEMENT
 */
function Uint8ArrayPrototypeBytesPerElement(): f64 {
  return 1;
}

/**
 * Uint8ClampedArray ( ...args ), 23.2.5.1.
 * @constructor Uint8ClampedArray
 * @extends %TypedArray%
 * @length 3
 */
function Uint8ClampedArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 2, args);
}

/**
 * Uint8ClampedArray.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Uint8ClampedArray.BYTES_PER_ELEMENT
 */
function Uint8ClampedArrayBytesPerElement(): f64 {
  return 1;
}

/**
 * Uint8ClampedArray.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Uint8ClampedArray.prototype.BYTES_PER_ELEMENT
 */
function Uint8ClampedArrayPrototypeBytesPerElement(): f64 {
  return 1;
}

/**
 * Int16Array ( ...args ), 23.2.5.1.
 * @constructor Int16Array
 * @extends %TypedArray%
 * @length 3
 */
function Int16ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 3, args);
}

/**
 * Int16Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Int16Array.BYTES_PER_ELEMENT
 */
function Int16ArrayBytesPerElement(): f64 {
  return 2;
}

/**
 * Int16Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Int16Array.prototype.BYTES_PER_ELEMENT
 */
function Int16ArrayPrototypeBytesPerElement(): f64 {
  return 2;
}

/**
 * Uint16Array ( ...args ), 23.2.5.1.
 * @constructor Uint16Array
 * @extends %TypedArray%
 * @length 3
 */
function Uint16ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 4, args);
}

/**
 * Uint16Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Uint16Array.BYTES_PER_ELEMENT
 */
function Uint16ArrayBytesPerElement(): f64 {
  return 2;
}

/**
 * Uint16Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Uint16Array.prototype.BYTES_PER_ELEMENT
 */
function Uint16ArrayPrototypeBytesPerElement(): f64 {
  return 2;
}

/**
 * Int32Array ( ...args ), 23.2.5.1.
 * @constructor Int32Array
 * @extends %TypedArray%
 * @length 3
 */
function Int32ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 5, args);
}

/**
 * Int32Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Int32Array.BYTES_PER_ELEMENT
 */
function Int32ArrayBytesPerElement(): f64 {
  return 4;
}

/**
 * Int32Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Int32Array.prototype.BYTES_PER_ELEMENT
 */
function Int32ArrayPrototypeBytesPerElement(): f64 {
  return 4;
}

/**
 * Uint32Array ( ...args ), 23.2.5.1.
 * @constructor Uint32Array
 * @extends %TypedArray%
 * @length 3
 */
function Uint32ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 6, args);
}

/**
 * Uint32Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Uint32Array.BYTES_PER_ELEMENT
 */
function Uint32ArrayBytesPerElement(): f64 {
  return 4;
}

/**
 * Uint32Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Uint32Array.prototype.BYTES_PER_ELEMENT
 */
function Uint32ArrayPrototypeBytesPerElement(): f64 {
  return 4;
}

/**
 * Float16Array ( ...args ), 23.2.5.1.
 * @constructor Float16Array
 * @extends %TypedArray%
 * @length 3
 */
function Float16ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 7, args);
}

/**
 * Float16Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Float16Array.BYTES_PER_ELEMENT
 */
function Float16ArrayBytesPerElement(): f64 {
  return 2;
}

/**
 * Float16Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Float16Array.prototype.BYTES_PER_ELEMENT
 */
function Float16ArrayPrototypeBytesPerElement(): f64 {
  return 2;
}

/**
 * Float32Array ( ...args ), 23.2.5.1.
 * @constructor Float32Array
 * @extends %TypedArray%
 * @length 3
 */
function Float32ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 8, args);
}

/**
 * Float32Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Float32Array.BYTES_PER_ELEMENT
 */
function Float32ArrayBytesPerElement(): f64 {
  return 4;
}

/**
 * Float32Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Float32Array.prototype.BYTES_PER_ELEMENT
 */
function Float32ArrayPrototypeBytesPerElement(): f64 {
  return 4;
}

/**
 * Float64Array ( ...args ), 23.2.5.1.
 * @constructor Float64Array
 * @extends %TypedArray%
 * @length 3
 */
function Float64ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 9, args);
}

/**
 * Float64Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value Float64Array.BYTES_PER_ELEMENT
 */
function Float64ArrayBytesPerElement(): f64 {
  return 8;
}

/**
 * Float64Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value Float64Array.prototype.BYTES_PER_ELEMENT
 */
function Float64ArrayPrototypeBytesPerElement(): f64 {
  return 8;
}

/**
 * BigInt64Array ( ...args ), 23.2.5.1.
 * @constructor BigInt64Array
 * @extends %TypedArray%
 * @length 3
 */
function BigInt64ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 10, args);
}

/**
 * BigInt64Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value BigInt64Array.BYTES_PER_ELEMENT
 */
function BigInt64ArrayBytesPerElement(): f64 {
  return 8;
}

/**
 * BigInt64Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value BigInt64Array.prototype.BYTES_PER_ELEMENT
 */
function BigInt64ArrayPrototypeBytesPerElement(): f64 {
  return 8;
}

/**
 * BigUint64Array ( ...args ), 23.2.5.1.
 * @constructor BigUint64Array
 * @extends %TypedArray%
 * @length 3
 */
function BigUint64ArrayConstructor(...args: Value[]): Obj {
  return TypedArrayConstruct(new.target, 11, args);
}

/**
 * BigUint64Array.BYTES_PER_ELEMENT, 23.2.6.1.
 * @value BigUint64Array.BYTES_PER_ELEMENT
 */
function BigUint64ArrayBytesPerElement(): f64 {
  return 8;
}

/**
 * BigUint64Array.prototype.BYTES_PER_ELEMENT, 23.2.7.1.
 * @value BigUint64Array.prototype.BYTES_PER_ELEMENT
 */
function BigUint64ArrayPrototypeBytesPerElement(): f64 {
  return 8;
}
