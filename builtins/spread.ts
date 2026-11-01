// Spread over a general iterable: ArrayAccumulation of a SpreadElement (ECMA-262
// 13.2.4.1). The interpreter's `array_spread` does strings and arrays with unmodified
// iteration itself and calls %ArraySpread for everything else.

/**
 * Appends the values of iterating src to the array literal arr; returns arr.
 * @intrinsic
 */
function ArraySpread(arr: Obj, src: Value): Obj {
  const iterator: Obj = GetIterator(src);
  const next: Value = Get(iterator, "next");
  let n: f64 = LengthOfArrayLike(arr);
  while (true) {
    const result: Value = Call(next, iterator);
    if (!IsObject(result)) {
      throw NewTypeError("iterator must return an object");
    }
    if (ToBoolean(Get(AsObj(result), "done"))) {
      return arr;
    }
    CreateDataPropertyOrThrow(arr, n, Get(AsObj(result), "value"));
    n = n + 1;
  }
}
