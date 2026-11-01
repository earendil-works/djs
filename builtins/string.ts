// String (ECMA-262 22.1): the constructor only, so far.

/**
 * String ( value ), 22.1.1.1. String wrapper objects (`new String`) are not supported
 * yet.
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
  if (new.target !== undefined) {
    throw NewTypeError("not supported yet: String objects");
  }
  return s;
}
