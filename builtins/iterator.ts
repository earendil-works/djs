// Iterator helpers (ECMA-262 27.1.3 and 27.1.2.1, ES2025) and Iterator.concat
// (iterator-sequencing). D1510: a helper object is an IteratorHelper object (heap
// class 28) whose generator-like state lives in slots, and each `next` runs one step of
// a hand-written state machine instead of a generator closure:
//
//   0 kind: 0 Iterator.from's wrapper, 1 map, 2 filter, 3 take, 4 drop, 5 flatMap,
//     6 concat
//   1 state: 0 suspended-start, 1 suspended-yield, 2 executing, 3 completed
//   2 the underlying iterator (concat: the current one), 3 its `next` method
//   4 the function (map, filter, flatMap), the remaining count (take, drop), the
//     iterables (concat: an array of [method, iterable] pairs)
//   5 the counter (concat: the index of the next iterable)
//   6 flatMap's inner iterator, 7 its `next` method
//
// A step either yields (sets state 1 and returns the value) or returns with the state
// still executing, which completes the helper.

/** The first slot of the per-kind state. */
function HelperState(o: Obj): i32 {
  return F64ToI32(AsNumber(RecordGet(o, 1)));
}

/** GetIteratorDirect ( obj ), 27.1.3.1.1 (ES2025 7.4.4): the `next` method. */
function GetIteratorDirectNext(o: Obj): Value {
  return Get(o, "next");
}

/**
 * IteratorStep on an iterator and its `next`, 7.4.9: the result object, or undefined
 * once it is done. The caller reads `value`.
 */
function IteratorStepResult(iterator: Obj, next: Value): Value {
  const result: Obj = IteratorNextResult(iterator, next);
  if (ToBoolean(Get(result, "done"))) {
    return undefined;
  }
  return result;
}

/**
 * GetIteratorFlattenable ( obj, primitiveHandling ), 7.4.5: the iterator; `strings`
 * selects iterate-string-primitives over reject-primitives.
 */
function GetIteratorFlattenable(obj: Value, strings: boolean): Obj {
  if (!IsObject(obj)) {
    if (!strings || !IsString(obj)) {
      throw NewTypeError("not an object");
    }
  }
  const method: Value = GetMethod(obj, WellKnownSymbol("iterator"));
  let iterator: Value = obj;
  if (method !== undefined) {
    iterator = Call(method, obj);
  }
  if (!IsObject(iterator)) {
    throw NewTypeError("not an object");
  }
  return AsObj(iterator);
}

/** The `this` of an Iterator.prototype method: an object (qjs's message). */
function ThisIteratorObject(v: Value): Obj {
  if (!IsObject(v)) {
    throw NewTypeError("not an object");
  }
  return AsObj(v);
}

/**
 * The callable argument of a helper or a consuming method: if it is not callable, the
 * receiver is closed (IteratorClose with a throw completion) and the TypeError thrown.
 */
function HelperCallable(o: Obj, f: Value): void {
  if (!IsCallable(f)) {
    const error: Obj = NewTypeError("not a function");
    IteratorCloseThrow(o);
    throw error;
  }
}

/**
 * The limit of take and drop, 27.1.4.4 and 27.1.4.12 steps 3-9: the receiver is
 * closed when the conversion throws or the limit is NaN, too large or negative.
 */
function HelperLimit(o: Obj, limit: Value): f64 {
  let numLimit: f64 = 0;
  try {
    numLimit = ToNumber(limit);
  } catch (e) {
    IteratorCloseThrow(o);
    throw e;
  }
  // NaN, or finite and above 2**53 - 1 (the 2025 spec).
  if (numLimit !== numLimit || (numLimit > 9007199254740991 && numLimit !== 1 / 0)) {
    const error: Obj = NewRangeError("must be positive");
    IteratorCloseThrow(o);
    throw error;
  }
  const integerLimit: f64 = ToIntegerOrInfinityNumber(numLimit);
  if (integerLimit < 0) {
    const error: Obj = NewRangeError("must be positive");
    IteratorCloseThrow(o);
    throw error;
  }
  return integerLimit;
}

/** A new helper of `kind` over iterator o (GetIteratorDirect) with slot 4 = data. */
function NewIteratorHelper(kind: i32, o: Obj, data: Value): Obj {
  const next: Value = GetIteratorDirectNext(o);
  const helper: Obj = CreateIteratorHelper(kind, o, next);
  RecordSet(helper, 4, data);
  RecordSet(helper, 5, 0);
  return helper;
}

/** Yield ( value ) of a step: suspends the helper at a yield. */
function HelperYield(o: Obj, value: Value): Value {
  RecordSet(o, 1, 1);
  return value;
}

/**
 * Calls the helper's function on (value, counter) and advances the counter; an
 * exception closes the underlying iterator (IfAbruptCloseIterator).
 */
function HelperCallFunction(o: Obj, iterator: Obj, value: Value): Value {
  const counter: f64 = AsNumber(RecordGet(o, 5));
  let result: Value = undefined;
  try {
    result = Call(RecordGet(o, 4), undefined, value, counter);
  } catch (e) {
    IteratorCloseThrow(iterator);
    throw e;
  }
  RecordSet(o, 5, counter + 1);
  return result;
}

/** One step of a helper (state executing): yields a value or completes. */
function IteratorHelperStep(o: Obj): Value {
  const kind: i32 = F64ToI32(AsNumber(RecordGet(o, 0)));
  if (kind === 6) {
    return ConcatStep(o);
  }
  const iterator: Obj = AsObj(RecordGet(o, 2));
  const next: Value = RecordGet(o, 3);
  if (kind === 1) {
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      return undefined;
    }
    const value: Value = Get(AsObj(result), "value");
    return HelperYield(o, HelperCallFunction(o, iterator, value));
  }
  if (kind === 2) {
    while (true) {
      const result: Value = IteratorStepResult(iterator, next);
      if (result === undefined) {
        return undefined;
      }
      const value: Value = Get(AsObj(result), "value");
      if (ToBoolean(HelperCallFunction(o, iterator, value))) {
        return HelperYield(o, value);
      }
    }
  }
  if (kind === 3) {
    const remaining: f64 = AsNumber(RecordGet(o, 4));
    if (remaining === 0) {
      IteratorClose(iterator);
      return undefined;
    }
    if (remaining !== 1 / 0) {
      RecordSet(o, 4, remaining - 1);
    }
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      return undefined;
    }
    return HelperYield(o, Get(AsObj(result), "value"));
  }
  if (kind === 4) {
    let remaining: f64 = AsNumber(RecordGet(o, 4));
    while (remaining > 0) {
      if (remaining !== 1 / 0) {
        remaining = remaining - 1;
        RecordSet(o, 4, remaining);
      }
      if (IteratorStepResult(iterator, next) === undefined) {
        return undefined;
      }
    }
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      return undefined;
    }
    return HelperYield(o, Get(AsObj(result), "value"));
  }
  return FlatMapStep(o, iterator, next);
}

/** A step of flatMap, 27.1.4.7: the inner iterator's next value, or the next mapping. */
function FlatMapStep(o: Obj, iterator: Obj, next: Value): Value {
  while (true) {
    const inner: Value = RecordGet(o, 6);
    if (inner !== undefined) {
      let result: Value = undefined;
      let value: Value = undefined;
      try {
        result = IteratorStepResult(AsObj(inner), RecordGet(o, 7));
        if (result !== undefined) {
          value = Get(AsObj(result), "value");
        }
      } catch (e) {
        IteratorCloseThrow(iterator);
        throw e;
      }
      if (result !== undefined) {
        return HelperYield(o, value);
      }
      RecordSet(o, 6, undefined);
      RecordSet(o, 7, undefined);
    }
    const outer: Value = IteratorStepResult(iterator, next);
    if (outer === undefined) {
      return undefined;
    }
    const mapped: Value = HelperCallFunction(o, iterator, Get(AsObj(outer), "value"));
    try {
      const innerIterator: Obj = GetIteratorFlattenable(mapped, false);
      const innerNext: Value = GetIteratorDirectNext(innerIterator);
      RecordSet(o, 6, innerIterator);
      RecordSet(o, 7, innerNext);
    } catch (e) {
      IteratorCloseThrow(iterator);
      throw e;
    }
  }
}

/** A step of Iterator.concat's closure, 27.1.3.2.3 (iterator-sequencing). */
function ConcatStep(o: Obj): Value {
  while (true) {
    const current: Value = RecordGet(o, 2);
    if (current !== undefined) {
      const result: Value = IteratorStepResult(AsObj(current), RecordGet(o, 3));
      if (result !== undefined) {
        return HelperYield(o, Get(AsObj(result), "value"));
      }
      RecordSet(o, 2, undefined);
      RecordSet(o, 3, undefined);
    }
    const iterables: Obj = AsObj(RecordGet(o, 4));
    const index: f64 = AsNumber(RecordGet(o, 5));
    if (index >= LengthOfArrayLike(iterables)) {
      return undefined;
    }
    RecordSet(o, 5, index + 1);
    const pair: Obj = AsObj(Get(iterables, index));
    const iter: Value = Call(Get(pair, 0), Get(pair, 1));
    if (!IsObject(iter)) {
      throw NewTypeError("not an object");
    }
    const next: Value = GetIteratorDirectNext(AsObj(iter));
    RecordSet(o, 2, iter);
    RecordSet(o, 3, next);
  }
}

/**
 * Resumes a suspended helper at its yield with a return completion: closes the
 * iterators the step was reading (flatMap: the inner one first, 27.1.4.7 step
 * 5.b.viii.4.b).
 */
function IteratorHelperReturnAtYield(o: Obj): void {
  const kind: i32 = F64ToI32(AsNumber(RecordGet(o, 0)));
  if (kind === 5) {
    const inner: Value = RecordGet(o, 6);
    if (inner !== undefined) {
      try {
        IteratorClose(AsObj(inner));
      } catch (e) {
        IteratorCloseThrow(AsObj(RecordGet(o, 2)));
        throw e;
      }
    }
  }
  IteratorClose(AsObj(RecordGet(o, 2)));
}

/**
 * %IteratorHelperPrototype%.next ( ), 27.1.2.1.1: GeneratorResume.
 * @builtin %IteratorHelperPrototype%.next
 */
function IteratorHelperPrototypeNext(this: Value): Obj {
  const o: Obj = ThisIteratorHelper(this, false);
  const state: i32 = HelperState(o);
  if (state === 2) {
    throw NewTypeError("cannot invoke a running iterator");
  }
  if (state === 3) {
    return CreateIterResultObject(undefined, true);
  }
  RecordSet(o, 1, 2);
  let value: Value = undefined;
  try {
    value = IteratorHelperStep(o);
  } catch (e) {
    RecordSet(o, 1, 3);
    throw e;
  }
  if (HelperState(o) === 1) {
    return CreateIterResultObject(value, false);
  }
  RecordSet(o, 1, 3);
  return CreateIterResultObject(undefined, true);
}

/**
 * %IteratorHelperPrototype%.return ( ), 27.1.2.1.2.
 * @builtin %IteratorHelperPrototype%.return
 */
function IteratorHelperPrototypeReturn(this: Value): Obj {
  const o: Obj = ThisIteratorHelper(this, false);
  const state: i32 = HelperState(o);
  if (state === 2) {
    throw NewTypeError("cannot invoke a running iterator");
  }
  if (state === 3) {
    return CreateIterResultObject(undefined, true);
  }
  if (state === 0) {
    RecordSet(o, 1, 3);
    // concat has no underlying iterator before it starts.
    const iterator: Value = RecordGet(o, 2);
    if (iterator !== undefined) {
      IteratorClose(AsObj(iterator));
    }
    return CreateIterResultObject(undefined, true);
  }
  RecordSet(o, 1, 2);
  try {
    IteratorHelperReturnAtYield(o);
  } finally {
    RecordSet(o, 1, 3);
  }
  return CreateIterResultObject(undefined, true);
}

/**
 * %IteratorHelperPrototype% [ %Symbol.toStringTag% ], 27.1.2.1.3.
 * @value %IteratorHelperPrototype%[Symbol.toStringTag]
 * @attributes configurable
 */
function IteratorHelperPrototypeToStringTag(): Str {
  return "Iterator Helper";
}

/**
 * Iterator ( ), 27.1.3.1: abstract.
 * @constructor Iterator
 */
function IteratorConstructor(): Obj {
  if (new.target === undefined) {
    throw NewTypeError("constructor requires 'new'");
  }
  if (new.target === RealmConstructor("Iterator")) {
    throw NewTypeError("abstract class not constructable");
  }
  return ObjectCreate(GetPrototypeFromConstructor(new.target, "IteratorPrototype"));
}

/**
 * Iterator.from ( O ), 27.1.3.2.2.
 * @builtin Iterator.from
 */
function IteratorFrom(o: Value): Obj {
  const iterator: Obj = GetIteratorFlattenable(o, true);
  const next: Value = GetIteratorDirectNext(iterator);
  if (OrdinaryHasInstance(RealmConstructor("Iterator"), iterator)) {
    return iterator;
  }
  return CreateIteratorHelper(0, iterator, next);
}

/**
 * %WrapForValidIteratorPrototype%.next ( ), 27.1.3.2.2.1.1.
 * @builtin %WrapForValidIteratorPrototype%.next
 */
function WrapForValidIteratorPrototypeNext(this: Value): Value {
  const o: Obj = ThisIteratorHelper(this, true);
  return Call(RecordGet(o, 3), RecordGet(o, 2));
}

/**
 * %WrapForValidIteratorPrototype%.return ( ), 27.1.3.2.2.1.2.
 * @builtin %WrapForValidIteratorPrototype%.return
 */
function WrapForValidIteratorPrototypeReturn(this: Value): Value {
  const o: Obj = ThisIteratorHelper(this, true);
  const iterator: Obj = AsObj(RecordGet(o, 2));
  const method: Value = GetMethod(iterator, "return");
  if (method === undefined) {
    return CreateIterResultObject(undefined, true);
  }
  return Call(method, iterator);
}

/**
 * Iterator.concat ( ...items ) (iterator-sequencing).
 * @builtin Iterator.concat
 */
function IteratorConcat(...items: Value[]): Obj {
  const n: f64 = LengthOfArrayLike(items);
  const iterables: Obj = ArrayCreate(0);
  for (let i: f64 = 0; i < n; i++) {
    const item: Value = Get(items, i);
    if (!IsObject(item)) {
      throw NewTypeError("not an object");
    }
    const method: Value = GetMethod(item, WellKnownSymbol("iterator"));
    if (method === undefined) {
      throw NewTypeError("not a function");
    }
    CreateDataPropertyOrThrow(iterables, i, CreateArrayFromPair(method, item));
  }
  const helper: Obj = CreateIteratorHelper(6, undefined, undefined);
  RecordSet(helper, 4, iterables);
  RecordSet(helper, 5, 0);
  return helper;
}

/**
 * Iterator.prototype.map ( mapper ), 27.1.4.8.
 * @builtin Iterator.prototype.map
 */
function IteratorPrototypeMap(this: Value, mapper: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, mapper);
  return NewIteratorHelper(1, o, mapper);
}

/**
 * Iterator.prototype.filter ( predicate ), 27.1.4.5.
 * @builtin Iterator.prototype.filter
 */
function IteratorPrototypeFilter(this: Value, predicate: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, predicate);
  return NewIteratorHelper(2, o, predicate);
}

/**
 * Iterator.prototype.take ( limit ), 27.1.4.12.
 * @builtin Iterator.prototype.take
 */
function IteratorPrototypeTake(this: Value, limit: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  return NewIteratorHelper(3, o, HelperLimit(o, limit));
}

/**
 * Iterator.prototype.drop ( limit ), 27.1.4.4.
 * @builtin Iterator.prototype.drop
 */
function IteratorPrototypeDrop(this: Value, limit: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  return NewIteratorHelper(4, o, HelperLimit(o, limit));
}

/**
 * Iterator.prototype.flatMap ( mapper ), 27.1.4.7.
 * @builtin Iterator.prototype.flatMap
 */
function IteratorPrototypeFlatMap(this: Value, mapper: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, mapper);
  return NewIteratorHelper(5, o, mapper);
}

/**
 * Iterator.prototype.reduce ( reducer [ , initialValue ] ), 27.1.4.9.
 * @builtin Iterator.prototype.reduce
 * @length 1
 */
function IteratorPrototypeReduce(this: Value, reducer: Value, ...rest: Value[]): Value {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, reducer);
  const next: Value = GetIteratorDirectNext(o);
  let accumulator: Value = undefined;
  let counter: f64 = 0;
  if (LengthOfArrayLike(rest) === 0) {
    const first: Value = IteratorStepResult(o, next);
    if (first === undefined) {
      throw NewTypeError("empty iterator");
    }
    accumulator = Get(AsObj(first), "value");
    counter = 1;
  } else {
    accumulator = Get(rest, 0);
  }
  while (true) {
    const result: Value = IteratorStepResult(o, next);
    if (result === undefined) {
      return accumulator;
    }
    const value: Value = Get(AsObj(result), "value");
    try {
      accumulator = Call(reducer, undefined, accumulator, value, counter);
    } catch (e) {
      IteratorCloseThrow(o);
      throw e;
    }
    counter++;
  }
}

/**
 * Iterator.prototype.toArray ( ), 27.1.4.13.
 * @builtin Iterator.prototype.toArray
 */
function IteratorPrototypeToArray(this: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  const next: Value = GetIteratorDirectNext(o);
  const items: Obj = ArrayCreate(0);
  let n: f64 = 0;
  while (true) {
    const result: Value = IteratorStepResult(o, next);
    if (result === undefined) {
      return items;
    }
    CreateDataPropertyOrThrow(items, n, Get(AsObj(result), "value"));
    n++;
  }
}

/**
 * The loop of forEach, some, every and find (27.1.4.6, 27.1.4.11, 27.1.4.2, 27.1.4.3):
 * calls fn on each value; mode 0 forEach, 1 some, 2 every, 3 find decides when a
 * result stops the loop (which closes the iterator).
 */
function IteratorConsume(thisValue: Value, fn: Value, mode: i32): Value {
  const o: Obj = ThisIteratorObject(thisValue);
  HelperCallable(o, fn);
  const next: Value = GetIteratorDirectNext(o);
  let counter: f64 = 0;
  while (true) {
    const result: Value = IteratorStepResult(o, next);
    if (result === undefined) {
      if (mode === 1) {
        return false;
      }
      if (mode === 2) {
        return true;
      }
      return undefined;
    }
    const value: Value = Get(AsObj(result), "value");
    let r: Value = undefined;
    try {
      r = Call(fn, undefined, value, counter);
    } catch (e) {
      IteratorCloseThrow(o);
      throw e;
    }
    if (mode !== 0) {
      const b: boolean = ToBoolean(r);
      if (mode === 2 ? !b : b) {
        IteratorClose(o);
        if (mode === 3) {
          return value;
        }
        return mode === 1;
      }
    }
    counter++;
  }
}

/**
 * Iterator.prototype.forEach ( procedure ), 27.1.4.6.
 * @builtin Iterator.prototype.forEach
 */
function IteratorPrototypeForEach(this: Value, procedure: Value): Value {
  return IteratorConsume(this, procedure, 0);
}

/**
 * Iterator.prototype.some ( predicate ), 27.1.4.11.
 * @builtin Iterator.prototype.some
 */
function IteratorPrototypeSome(this: Value, predicate: Value): Value {
  return IteratorConsume(this, predicate, 1);
}

/**
 * Iterator.prototype.every ( predicate ), 27.1.4.2.
 * @builtin Iterator.prototype.every
 */
function IteratorPrototypeEvery(this: Value, predicate: Value): Value {
  return IteratorConsume(this, predicate, 2);
}

/**
 * Iterator.prototype.find ( predicate ), 27.1.4.3.
 * @builtin Iterator.prototype.find
 */
function IteratorPrototypeFind(this: Value, predicate: Value): Value {
  return IteratorConsume(this, predicate, 3);
}

/**
 * SetterThatIgnoresPrototypeProperties ( thisValue, home, p, v ), 27.1.4.1.1.
 */
function SetterThatIgnoresPrototypeProperties(thisValue: Value, home: Obj, p: Value, v: Value): void {
  if (!IsObject(thisValue)) {
    throw NewTypeError("not an object");
  }
  const o: Obj = AsObj(thisValue);
  if (o === home) {
    throw NewTypeError("Cannot assign to read only property");
  }
  if (OwnPropertyAttributes(o, p) < 0) {
    CreateDataPropertyOrThrow(o, p, v);
  } else {
    Set(o, p, v);
  }
}

/**
 * get Iterator.prototype.constructor, 27.1.4.1.
 * @getter Iterator.prototype.constructor
 */
function IteratorPrototypeConstructorGet(this: Value): Value {
  return RealmConstructor("Iterator");
}

/**
 * set Iterator.prototype.constructor, 27.1.4.1.
 * @setter Iterator.prototype.constructor
 */
function IteratorPrototypeConstructorSet(this: Value, v: Value): void {
  SetterThatIgnoresPrototypeProperties(this, RealmIntrinsic("IteratorPrototype"), "constructor", v);
}

/**
 * get Iterator.prototype [ %Symbol.toStringTag% ], 27.1.4.14.
 * @getter Iterator.prototype[Symbol.toStringTag]
 */
function IteratorPrototypeToStringTagGet(this: Value): Value {
  return "Iterator";
}

/**
 * set Iterator.prototype [ %Symbol.toStringTag% ], 27.1.4.14.
 * @setter Iterator.prototype[Symbol.toStringTag]
 */
function IteratorPrototypeToStringTagSet(this: Value, v: Value): void {
  SetterThatIgnoresPrototypeProperties(this, RealmIntrinsic("IteratorPrototype"), WellKnownSymbol("toStringTag"), v);
}
