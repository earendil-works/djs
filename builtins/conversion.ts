// Type conversion abstract operations (ECMA-262 7.1) and friends.

/**
 * ToPrimitive ( input [ , preferredType ] ), 7.1.1. `hint` is "default", "string" or
 * "number". The interpreter calls it for objects in `+`, comparisons and so on.
 * @intrinsic
 */
function ToPrimitive(input: Value, hint: Str): Value {
  if (!IsObject(input)) {
    return input;
  }
  const o: Obj = AsObj(input);
  const exoticToPrim: Value = GetMethod(o, WellKnownSymbol("toPrimitive"));
  if (exoticToPrim !== undefined) {
    const result: Value = Call(exoticToPrim, o, hint);
    if (IsObject(result)) {
      throw NewTypeError("toPrimitive");
    }
    return result;
  }
  return OrdinaryToPrimitive(o, hint === "string" ? "string" : "number");
}

/** OrdinaryToPrimitive ( O, hint ), 7.1.1.1. */
function OrdinaryToPrimitive(o: Obj, hint: Str): Value {
  let name: Str = hint === "string" ? "toString" : "valueOf";
  for (let i: i32 = 0; i < 2; i++) {
    const method: Value = Get(o, name);
    if (IsCallable(method)) {
      const result: Value = Call(method, o);
      if (!IsObject(result)) {
        return result;
      }
    }
    name = name === "toString" ? "valueOf" : "toString";
  }
  throw NewTypeError("toPrimitive");
}

/** GetMethod ( V, P ), 7.3.11. */
function GetMethod(v: Value, key: Value): Value {
  const func: Value = GetV(v, key);
  if (func === undefined || func === null) {
    return undefined;
  }
  if (!IsCallable(func)) {
    throw NewTypeError("not a function");
  }
  return func;
}

/**
 * ToNumber ( argument ), 7.1.4.
 * @intrinsic
 */
function ToNumber(v: Value): f64 {
  if (IsObject(v)) {
    return ToNumberPrimitive(ToPrimitive(v, "number"));
  }
  return ToNumberPrimitive(v);
}

/**
 * ToString ( argument ), 7.1.17.
 * @intrinsic
 */
function ToString(v: Value): Str {
  if (IsObject(v)) {
    return ToStringPrimitive(ToPrimitive(v, "string"));
  }
  return ToStringPrimitive(v);
}

/** ToIntegerOrInfinity ( argument ), 7.1.5. */
function ToIntegerOrInfinity(v: Value): f64 {
  return ToIntegerOrInfinityNumber(ToNumber(v));
}

/** ToLength ( argument ), 7.1.20. */
function ToLength(v: Value): f64 {
  const len: f64 = ToIntegerOrInfinity(v);
  if (len <= 0) {
    return 0;
  }
  if (len > 9007199254740991) {
    return 9007199254740991;
  }
  return len;
}

/** LengthOfArrayLike ( obj ), 7.3.18. */
function LengthOfArrayLike(o: Obj): f64 {
  return ToLength(Get(o, "length"));
}
