// The Math object (ECMA-262 21.3).

/**
 * The Math object, 21.3: an ordinary object whose Symbol.toStringTag is "Math".
 * @value globalThis.Math
 * @attributes writable configurable
 */
function MathObject(): Obj {
  const m: Obj = ObjectCreate(RealmIntrinsic("ObjectPrototype"));
  DefineDataPropertyOrThrow(m, WellKnownSymbol("toStringTag"), "Math", 4);
  return m;
}

/**
 * Math.pow ( base, exponent ), 21.3.2.26.
 * @builtin Math.pow
 */
function MathPow(base: Value, exponent: Value): f64 {
  const b: f64 = ToNumber(base);
  return NumberPow(b, ToNumber(exponent));
}
