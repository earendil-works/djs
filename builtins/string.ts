// String (ECMA-262 22.1): the constructor and the String object basics so far.

/**
 * String ( value ), 22.1.1.1.
 * @constructor String
 * @length 1
 */
function StringConstructor(...args: Value[]): Value {
  let s: Str = "";
  if (LengthOfArrayLike(args) > 0) {
    const value: Value = Get(args, 0);
    if (new.target === undefined && IsSymbol(value)) {
      return SymbolDescriptiveString(AsSym(value));
    }
    s = ToString(value);
  }
  if (new.target === undefined) {
    return s;
  }
  return WrapPrimitive(s, GetPrototypeFromConstructor(new.target, "StringPrototype"));
}

/** ThisStringValue ( value ), 22.1.3.35.1. */
function ThisStringValue(value: Value): Str {
  const s: Value = PrimitiveValue(value);
  if (!IsString(s)) {
    throw NewTypeError("not a string");
  }
  return AsStr(s);
}

/**
 * String.prototype.toString ( ), 22.1.3.32.
 * @builtin String.prototype.toString
 */
function StringPrototypeToString(this: Value): Str {
  return ThisStringValue(this);
}

/**
 * String.prototype.valueOf ( ), 22.1.3.35.
 * @builtin String.prototype.valueOf
 */
function StringPrototypeValueOf(this: Value): Str {
  return ThisStringValue(this);
}
