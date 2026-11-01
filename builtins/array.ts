// Array.prototype methods (ECMA-262 23.1.3).

/** ArraySpeciesCreate ( originalArray, length ), 10.4.2.3 (one realm). */
function ArraySpeciesCreate(original: Obj, length: f64): Obj {
  if (!IsArray(original)) {
    return ArrayCreate(length);
  }
  let c: Value = Get(original, "constructor");
  if (IsObject(c)) {
    c = Get(AsObj(c), WellKnownSymbol("species"));
    if (c === null) {
      c = undefined;
    }
  }
  if (c === undefined) {
    return ArrayCreate(length);
  }
  if (!IsConstructor(c)) {
    throw NewTypeError("not a constructor");
  }
  return Construct(c, length);
}

/**
 * Array.prototype.forEach ( callbackfn [ , thisArg ] ), 23.1.3.15.
 * @builtin Array.prototype.forEach
 * @native
 */
function ArrayPrototypeForEach(this: Value, callbackfn: Value, thisArg?: Value): void {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  if (!IsCallable(callbackfn)) {
    throw NewTypeError("not a function");
  }
  for (let k: f64 = 0; k < len; k++) {
    if (HasProperty(o, k)) {
      Call(callbackfn, thisArg, Get(o, k), k, o);
    }
  }
}

/**
 * Array.prototype.map ( callbackfn [ , thisArg ] ), 23.1.3.21.
 * @builtin Array.prototype.map
 * @native
 */
function ArrayPrototypeMap(this: Value, callbackfn: Value, thisArg?: Value): Obj {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  if (!IsCallable(callbackfn)) {
    throw NewTypeError("not a function");
  }
  const a: Obj = ArraySpeciesCreate(o, len);
  for (let k: f64 = 0; k < len; k++) {
    if (HasProperty(o, k)) {
      const kValue: Value = Get(o, k);
      const mappedValue: Value = Call(callbackfn, thisArg, kValue, k, o);
      CreateDataPropertyOrThrow(a, k, mappedValue);
    }
  }
  return a;
}

/**
 * Array.prototype.indexOf ( searchElement [ , fromIndex ] ), 23.1.3.17.
 * @builtin Array.prototype.indexOf
 */
function ArrayPrototypeIndexOf(this: Value, searchElement: Value, fromIndex?: Value): f64 {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  if (len === 0) {
    return -1;
  }
  let n: f64 = ToIntegerOrInfinity(fromIndex);
  if (n >= len) {
    return -1;
  }
  if (n < 0) {
    n = len + n;
    if (n < 0) {
      n = 0;
    }
  }
  for (let k: f64 = n; k < len; k++) {
    if (HasProperty(o, k)) {
      if (Get(o, k) === searchElement) {
        return k;
      }
    }
  }
  return -1;
}

/**
 * Array.prototype.join ( separator ), 23.1.3.18.
 * @builtin Array.prototype.join
 * @native
 */
function ArrayPrototypeJoin(this: Value, separator: Value): Str {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  let sep: Str = ",";
  if (separator !== undefined) {
    sep = ToString(separator);
  }
  let r: Str = "";
  for (let k: f64 = 0; k < len; k++) {
    if (k > 0) {
      r += sep;
    }
    const element: Value = Get(o, k);
    if (element !== undefined && element !== null) {
      r += ToString(element);
    }
  }
  return r;
}

/** SameValueZero ( x, y ), 7.2.11. */
function SameValueZero(x: Value, y: Value): boolean {
  if (IsNumber(x) && IsNumber(y)) {
    const a: f64 = AsNumber(x);
    const b: f64 = AsNumber(y);
    // NaN is the only value that is not equal to itself.
    return a === b || (a !== a && b !== b);
  }
  return x === y;
}

/**
 * Array.prototype.includes ( searchElement [ , fromIndex ] ), 23.1.3.16.
 * @builtin Array.prototype.includes
 */
function ArrayPrototypeIncludes(this: Value, searchElement: Value, fromIndex?: Value): boolean {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  if (len === 0) {
    return false;
  }
  let n: f64 = ToIntegerOrInfinity(fromIndex);
  if (n < 0) {
    n = len + n;
    if (n < 0) {
      n = 0;
    }
  }
  for (let k: f64 = n; k < len; k++) {
    if (SameValueZero(Get(o, k), searchElement)) {
      return true;
    }
  }
  return false;
}
