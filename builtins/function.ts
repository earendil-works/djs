// Function.prototype (ECMA-262 20.2.3). The Function constructor is not supported (it
// would need eval-like compilation).

/**
 * Function.prototype.call ( thisArg, ...args ), 20.2.3.3.
 * @builtin Function.prototype.call
 * @length 1
 */
function FunctionPrototypeCall(this: Value, thisArg?: Value, ...args: Value[]): Value {
  if (!IsCallable(this)) {
    throw NewTypeError("not a function");
  }
  return CallWithArray(this, thisArg, args);
}

/** CreateListFromArrayLike ( obj ), 7.3.20, as a dense array. */
function CreateListFromArrayLike(obj: Value): Obj {
  if (!IsObject(obj)) {
    throw NewTypeError("not a object");
  }
  const o: Obj = AsObj(obj);
  const len: f64 = LengthOfArrayLike(o);
  const list: Obj = ArrayCreate(0);
  for (let k: f64 = 0; k < len; k++) {
    CreateDataPropertyOrThrow(list, k, Get(o, k));
  }
  return list;
}

/**
 * Function.prototype.apply ( thisArg, argArray ), 20.2.3.1.
 * @builtin Function.prototype.apply
 */
function FunctionPrototypeApply(this: Value, thisArg: Value, argArray: Value): Value {
  if (!IsCallable(this)) {
    throw NewTypeError("not a function");
  }
  if (argArray === undefined || argArray === null) {
    return Call(this, thisArg);
  }
  return CallWithArray(this, thisArg, CreateListFromArrayLike(argArray));
}

/**
 * Function.prototype.bind ( thisArg, ...args ), 20.2.3.2.
 * @builtin Function.prototype.bind
 * @length 1
 */
function FunctionPrototypeBind(this: Value, thisArg?: Value, ...args: Value[]): Obj {
  if (!IsCallable(this)) {
    throw NewTypeError("not a function");
  }
  const target: Fn = AsFn(this);
  const f: Obj = BoundFunctionCreate(target, thisArg, args);
  let l: f64 = 0;
  if (HasOwnProperty(target, "length")) {
    const targetLen: Value = Get(target, "length");
    if (IsNumber(targetLen)) {
      const n: f64 = AsNumber(targetLen);
      if (n === 1 / 0) {
        l = n;
      } else if (n !== -1 / 0) {
        l = ToIntegerOrInfinityNumber(n) - LengthOfArrayLike(args);
        if (l < 0) {
          l = 0;
        }
      }
    }
  }
  DefineDataPropertyOrThrow(f, "length", l, 4);
  const name: Value = Get(target, "name");
  const nameStr: Str = IsString(name) ? AsStr(name) : "";
  DefineDataPropertyOrThrow(f, "name", "bound " + nameStr, 4);
  return f;
}

/**
 * Function.prototype.toString ( ), 20.2.3.5.
 * @builtin Function.prototype.toString
 */
function FunctionPrototypeToString(this: Value): Str {
  return FunctionToString(this);
}
