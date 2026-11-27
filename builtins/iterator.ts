// Iterator helpers (ECMA-262 27.1.3 and 27.1.2.1, ES2025) and Iterator.concat
// (iterator-sequencing). D1510: a helper object is an IteratorHelper object (heap
// class 28) whose generator-like state lives in its fields, and each `next` runs one
// step of a hand-written state machine instead of a generator closure. Every helper
// starts with its kind and its state; its record schema (below, D3710) names the rest.
// A step either yields (sets the state to suspendedYield and returns the value) or
// returns with the state still executing, which completes the helper.

/** What a helper does (`HelperHeader.kind`). @constants */
enum HelperKinds {
  /** Iterator.from's wrapper. */
  wrap = 0,
  map = 1,
  filter = 2,
  take = 3,
  drop = 4,
  flatMap = 5,
  concat = 6,
  chunks = 7,
  windows = 8,
  /** zip and zipKeyed. */
  zip = 9,
}

/** A helper's state (`HelperHeader.state`). @constants */
enum HelperStates {
  start = 0,
  suspendedYield = 1,
  executing = 2,
  completed = 3,
}

/** Iterator.zip's modes. @constants */
enum ZipModes {
  shortest = 0,
  longest = 1,
  strict = 2,
}

/**
 * The fields every helper starts with (CreateIteratorHelper sets them).
 * @record
 * @class IteratorHelper
 */
enum HelperHeader {
  /** @enum HelperKinds */
  kind,
  /** @enum HelperStates */
  state,
}

/**
 * A helper over one underlying iterator and its `next` method; with these fields only,
 * Iterator.from's wrapper.
 * @record
 * @class IteratorHelper
 * @extends HelperHeader
 */
enum IteratorHelper {
  kind,
  state,
  iterator,
  next,
}

/**
 * map and filter: the function and the counter it gets.
 * @record
 * @class IteratorHelper
 * @extends IteratorHelper
 */
enum CallbackHelper {
  kind,
  state,
  iterator,
  next,
  fn,
  counter,
}

/**
 * flatMap: also the inner iterator and its `next` method (undefined between them).
 * @record
 * @class IteratorHelper
 * @extends CallbackHelper
 */
enum FlatMapHelper {
  kind,
  state,
  iterator,
  next,
  fn,
  counter,
  inner,
  innerNext,
}

/**
 * take and drop: the remaining count.
 * @record
 * @class IteratorHelper
 * @extends IteratorHelper
 */
enum LimitHelper {
  kind,
  state,
  iterator,
  next,
  remaining,
}

/**
 * chunks: the size, and true once the underlying iterator is done (a last partial chunk
 * was yielded).
 * @record
 * @class IteratorHelper
 * @extends IteratorHelper
 */
enum ChunksHelper {
  kind,
  state,
  iterator,
  next,
  size,
  done,
}

/**
 * windows: the size, allow-partial, done (as for chunks), the window (an Array).
 * @record
 * @class IteratorHelper
 * @extends IteratorHelper
 */
enum WindowsHelper {
  kind,
  state,
  iterator,
  next,
  size,
  allowPartial,
  done,
  window,
}

/**
 * Iterator.concat: the current iterator and its `next` (undefined between items), the
 * iterables (an Array of [method, iterable] pairs), the index of the next one.
 * @record
 * @class IteratorHelper
 * @extends IteratorHelper
 */
enum ConcatHelper {
  kind,
  state,
  iterator,
  next,
  iterables,
  index,
}

/**
 * zip and zipKeyed: the iterators (an Array; an entry is null once done in longest
 * mode), their `next` methods, the padding, the mode, which iterators are open
 * (booleans), zipKeyed's keys (undefined for zip).
 * @record
 * @class IteratorHelper
 * @extends HelperHeader
 */
enum ZipHelper {
  kind,
  state,
  iterators,
  nexts,
  padding,
  /** @enum ZipModes */
  mode,
  open,
  keys,
}

/**
 * Iterator.zip's and zipKeyed's options, as ZipOptions reads them: the mode and the
 * padding option.
 * @record
 */
enum ZipSettings {
  /** @enum ZipModes */
  mode,
  padding,
}

/** The helper's state. */
function HelperState(o: Obj): i32 {
  return F64ToI32(AsNumber(RecordGet(o, HelperHeader.state)));
}

/** The helper's kind. */
function HelperKind(o: Obj): i32 {
  return F64ToI32(AsNumber(RecordGet(o, HelperHeader.kind)));
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

/** A new helper of `schema` and `kind` over iterator o (GetIteratorDirect). */
function NewIteratorHelper(schema: Schema, kind: i32, o: Obj): Obj {
  const next: Value = GetIteratorDirectNext(o);
  const helper: Obj = CreateIteratorHelper(schema, kind);
  RecordSet(helper, IteratorHelper.iterator, o);
  RecordSet(helper, IteratorHelper.next, next);
  return helper;
}

/** A new map, filter or flatMap helper over o with function f. */
function NewCallbackHelper(schema: Schema, kind: i32, o: Obj, f: Value): Obj {
  const helper: Obj = NewIteratorHelper(schema, kind, o);
  RecordSet(helper, CallbackHelper.fn, f);
  RecordSet(helper, CallbackHelper.counter, 0);
  return helper;
}

/** Yield ( value ) of a step: suspends the helper at a yield. */
function HelperYield(o: Obj, value: Value): Value {
  RecordSet(o, HelperHeader.state, HelperStates.suspendedYield);
  return value;
}

/**
 * Calls the helper's function on (value, counter) and advances the counter; an
 * exception closes the underlying iterator (IfAbruptCloseIterator).
 */
function HelperCallFunction(o: Obj, iterator: Obj, value: Value): Value {
  const counter: f64 = AsNumber(RecordGet(o, CallbackHelper.counter));
  let result: Value = undefined;
  try {
    result = Call(RecordGet(o, CallbackHelper.fn), undefined, value, counter);
  } catch (e) {
    IteratorCloseThrow(iterator);
    throw e;
  }
  RecordSet(o, CallbackHelper.counter, counter + 1);
  return result;
}

/** One step of a helper (state executing): yields a value or completes. */
function IteratorHelperStep(o: Obj): Value {
  const kind: i32 = HelperKind(o);
  if (kind === HelperKinds.concat) {
    return ConcatStep(o);
  }
  if (kind === HelperKinds.zip) {
    return ZipStep(o);
  }
  const iterator: Obj = AsObj(RecordGet(o, IteratorHelper.iterator));
  const next: Value = RecordGet(o, IteratorHelper.next);
  if (kind === HelperKinds.map) {
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      return undefined;
    }
    const value: Value = Get(AsObj(result), "value");
    return HelperYield(o, HelperCallFunction(o, iterator, value));
  }
  if (kind === HelperKinds.filter) {
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
  if (kind === HelperKinds.take) {
    const remaining: f64 = AsNumber(RecordGet(o, LimitHelper.remaining));
    if (remaining === 0) {
      IteratorClose(iterator);
      return undefined;
    }
    if (remaining !== 1 / 0) {
      RecordSet(o, LimitHelper.remaining, remaining - 1);
    }
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      return undefined;
    }
    return HelperYield(o, Get(AsObj(result), "value"));
  }
  if (kind === HelperKinds.drop) {
    let remaining: f64 = AsNumber(RecordGet(o, LimitHelper.remaining));
    while (remaining > 0) {
      if (remaining !== 1 / 0) {
        remaining = remaining - 1;
        RecordSet(o, LimitHelper.remaining, remaining);
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
  if (kind === HelperKinds.flatMap) {
    return FlatMapStep(o, iterator, next);
  }
  if (kind === HelperKinds.chunks) {
    return ChunksStep(o, iterator, next);
  }
  return WindowsStep(o, iterator, next);
}

/** A copy of the first n elements of the internal array a, as a new Array. */
function ArrayPrefixCopy(a: Obj, from: f64, n: f64): Obj {
  const copy: Obj = ArrayCreate(0);
  for (let i: f64 = 0; i < n; i++) {
    CreateDataPropertyOrThrow(copy, i, Get(a, from + i));
  }
  return copy;
}

/** A step of chunks (iterator-chunking): the next full chunk, or the partial last one. */
function ChunksStep(o: Obj, iterator: Obj, next: Value): Value {
  if (RecordGet(o, ChunksHelper.done) === true) {
    return undefined;
  }
  const size: f64 = AsNumber(RecordGet(o, ChunksHelper.size));
  const buffer: Obj = ArrayCreate(0);
  let n: f64 = 0;
  while (true) {
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      if (n > 0) {
        RecordSet(o, ChunksHelper.done, true);
        return HelperYield(o, buffer);
      }
      return undefined;
    }
    CreateDataPropertyOrThrow(buffer, n, Get(AsObj(result), "value"));
    n++;
    if (n === size) {
      return HelperYield(o, buffer);
    }
  }
}

/**
 * A step of windows (iterator-chunking): the next full window; with allow-partial, a
 * short last one if the iterator ended before the first window filled.
 */
function WindowsStep(o: Obj, iterator: Obj, next: Value): Value {
  if (RecordGet(o, WindowsHelper.done) === true) {
    return undefined;
  }
  const size: f64 = AsNumber(RecordGet(o, WindowsHelper.size));
  let window: Value = RecordGet(o, WindowsHelper.window);
  if (window === undefined) {
    window = ArrayCreate(0);
    RecordSet(o, WindowsHelper.window, window);
  }
  const w: Obj = AsObj(window);
  while (true) {
    let n: f64 = LengthOfArrayLike(w);
    const result: Value = IteratorStepResult(iterator, next);
    if (result === undefined) {
      if (RecordGet(o, WindowsHelper.allowPartial) === true && n > 0 && n < size) {
        RecordSet(o, WindowsHelper.done, true);
        return HelperYield(o, ArrayPrefixCopy(w, 0, n));
      }
      return undefined;
    }
    const value: Value = Get(AsObj(result), "value");
    if (n === size) {
      // Drop the oldest element: shift the window down by one.
      for (let i: f64 = 1; i < n; i++) {
        CreateDataPropertyOrThrow(w, i - 1, Get(w, i));
      }
      n = n - 1;
    }
    CreateDataPropertyOrThrow(w, n, value);
    Set(w, "length", n + 1);
    if (n + 1 === size) {
      return HelperYield(o, ArrayPrefixCopy(w, 0, size));
    }
  }
}

/**
 * IteratorCloseAll ( iters, completion ) (joint-iteration): closes the open iterators
 * of the list in reverse order, then completes with `error` if `isThrow` or if a
 * `return` threw (the first such error; later ones are ignored).
 */
function IteratorCloseAll(iters: Obj, open: Obj, error: Value, isThrow: boolean): void {
  let threw: boolean = isThrow;
  let err: Value = error;
  for (let i: f64 = LengthOfArrayLike(iters) - 1; i >= 0; i--) {
    if (Get(open, i) === true) {
      const iter: Obj = AsObj(Get(iters, i));
      if (threw) {
        IteratorCloseThrow(iter);
      } else {
        try {
          IteratorClose(iter);
        } catch (e) {
          threw = true;
          err = e;
        }
      }
    }
  }
  if (threw) {
    throw err;
  }
}

/** A step of IteratorZip (joint-iteration): the next results array or object. */
function ZipStep(o: Obj): Value {
  const iters: Obj = AsObj(RecordGet(o, ZipHelper.iterators));
  const nexts: Obj = AsObj(RecordGet(o, ZipHelper.nexts));
  const padding: Obj = AsObj(RecordGet(o, ZipHelper.padding));
  const mode: i32 = F64ToI32(AsNumber(RecordGet(o, ZipHelper.mode)));
  const open: Obj = AsObj(RecordGet(o, ZipHelper.open));
  const n: f64 = LengthOfArrayLike(iters);
  if (n === 0) {
    return undefined;
  }
  const results: Obj = ArrayCreate(0);
  for (let i: f64 = 0; i < n; i++) {
    const iter: Value = Get(iters, i);
    let value: Value = undefined;
    if (iter === null) {
      value = Get(padding, i);
    } else {
      let result: Value = undefined;
      try {
        result = IteratorStepResult(AsObj(iter), Get(nexts, i));
        if (result !== undefined) {
          value = Get(AsObj(result), "value");
        }
      } catch (e) {
        CreateDataPropertyOrThrow(open, i, false);
        IteratorCloseAll(iters, open, e, true);
      }
      if (result === undefined) {
        CreateDataPropertyOrThrow(open, i, false);
        if (mode === ZipModes.shortest) {
          IteratorCloseAll(iters, open, undefined, false);
          return undefined;
        }
        if (mode === ZipModes.strict) {
          if (i !== 0) {
            IteratorCloseAll(iters, open, NewTypeError("iterators have different lengths"), true);
          }
          for (let k: f64 = 1; k < n; k++) {
            let rest: Value = undefined;
            try {
              rest = IteratorStepResult(AsObj(Get(iters, k)), Get(nexts, k));
            } catch (e) {
              CreateDataPropertyOrThrow(open, k, false);
              IteratorCloseAll(iters, open, e, true);
            }
            if (rest !== undefined) {
              IteratorCloseAll(iters, open, NewTypeError("iterators have different lengths"), true);
            }
            CreateDataPropertyOrThrow(open, k, false);
          }
          return undefined;
        }
        if (!ZipAnyOpen(open, n)) {
          return undefined;
        }
        CreateDataPropertyOrThrow(iters, i, null);
        value = Get(padding, i);
      }
    }
    CreateDataPropertyOrThrow(results, i, value);
  }
  const keys: Value = RecordGet(o, ZipHelper.keys);
  if (keys === undefined) {
    return HelperYield(o, results);
  }
  const obj: Obj = ObjectCreate(null);
  for (let i: f64 = 0; i < n; i++) {
    CreateDataPropertyOrThrow(obj, Get(AsObj(keys), i), Get(results, i));
  }
  return HelperYield(o, obj);
}

/** Whether one of the n iterators of a zip is still open. */
function ZipAnyOpen(open: Obj, n: f64): boolean {
  for (let i: f64 = 0; i < n; i++) {
    if (Get(open, i) === true) {
      return true;
    }
  }
  return false;
}

/** A step of flatMap, 27.1.4.7: the inner iterator's next value, or the next mapping. */
function FlatMapStep(o: Obj, iterator: Obj, next: Value): Value {
  while (true) {
    const inner: Value = RecordGet(o, FlatMapHelper.inner);
    if (inner !== undefined) {
      let result: Value = undefined;
      let value: Value = undefined;
      try {
        result = IteratorStepResult(AsObj(inner), RecordGet(o, FlatMapHelper.innerNext));
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
      RecordSet(o, FlatMapHelper.inner, undefined);
      RecordSet(o, FlatMapHelper.innerNext, undefined);
    }
    const outer: Value = IteratorStepResult(iterator, next);
    if (outer === undefined) {
      return undefined;
    }
    const mapped: Value = HelperCallFunction(o, iterator, Get(AsObj(outer), "value"));
    try {
      const innerIterator: Obj = GetIteratorFlattenable(mapped, false);
      const innerNext: Value = GetIteratorDirectNext(innerIterator);
      RecordSet(o, FlatMapHelper.inner, innerIterator);
      RecordSet(o, FlatMapHelper.innerNext, innerNext);
    } catch (e) {
      IteratorCloseThrow(iterator);
      throw e;
    }
  }
}

/** A step of Iterator.concat's closure, 27.1.3.2.3 (iterator-sequencing). */
function ConcatStep(o: Obj): Value {
  while (true) {
    const current: Value = RecordGet(o, ConcatHelper.iterator);
    if (current !== undefined) {
      const result: Value = IteratorStepResult(AsObj(current), RecordGet(o, ConcatHelper.next));
      if (result !== undefined) {
        return HelperYield(o, Get(AsObj(result), "value"));
      }
      RecordSet(o, ConcatHelper.iterator, undefined);
      RecordSet(o, ConcatHelper.next, undefined);
    }
    const iterables: Obj = AsObj(RecordGet(o, ConcatHelper.iterables));
    const index: f64 = AsNumber(RecordGet(o, ConcatHelper.index));
    if (index >= LengthOfArrayLike(iterables)) {
      return undefined;
    }
    RecordSet(o, ConcatHelper.index, index + 1);
    const pair: Obj = AsObj(Get(iterables, index));
    const iter: Value = Call(Get(pair, 0), Get(pair, 1));
    if (!IsObject(iter)) {
      throw NewTypeError("not an object");
    }
    const next: Value = GetIteratorDirectNext(AsObj(iter));
    RecordSet(o, ConcatHelper.iterator, iter);
    RecordSet(o, ConcatHelper.next, next);
  }
}

/**
 * Resumes a suspended helper at its yield with a return completion: closes the
 * iterators the step was reading (flatMap: the inner one first, 27.1.4.7 step
 * 5.b.viii.4.b).
 */
function IteratorHelperReturnAtYield(o: Obj): void {
  const kind: i32 = HelperKind(o);
  if (kind === HelperKinds.zip) {
    ZipCloseAll(o);
    return;
  }
  // The yield of a last partial chunk or window ignores the completion.
  if (kind === HelperKinds.chunks && RecordGet(o, ChunksHelper.done) === true) {
    return;
  }
  if (kind === HelperKinds.windows && RecordGet(o, WindowsHelper.done) === true) {
    return;
  }
  if (kind === HelperKinds.flatMap) {
    const inner: Value = RecordGet(o, FlatMapHelper.inner);
    if (inner !== undefined) {
      try {
        IteratorClose(AsObj(inner));
      } catch (e) {
        IteratorCloseThrow(AsObj(RecordGet(o, IteratorHelper.iterator)));
        throw e;
      }
    }
  }
  IteratorClose(AsObj(RecordGet(o, IteratorHelper.iterator)));
}

/** Closes a zip helper's open iterators (a return completion). */
function ZipCloseAll(o: Obj): void {
  IteratorCloseAll(AsObj(RecordGet(o, ZipHelper.iterators)), AsObj(RecordGet(o, ZipHelper.open)), undefined, false);
}

/**
 * %IteratorHelperPrototype%.next ( ), 27.1.2.1.1: GeneratorResume.
 * @builtin %IteratorHelperPrototype%.next
 */
function IteratorHelperPrototypeNext(this: Value): Obj {
  const o: Obj = ThisIteratorHelper(this, false);
  const state: i32 = HelperState(o);
  if (state === HelperStates.executing) {
    throw NewTypeError("cannot invoke a running iterator");
  }
  if (state === HelperStates.completed) {
    return CreateIterResultObject(undefined, true);
  }
  RecordSet(o, HelperHeader.state, HelperStates.executing);
  let value: Value = undefined;
  try {
    value = IteratorHelperStep(o);
  } catch (e) {
    RecordSet(o, HelperHeader.state, HelperStates.completed);
    throw e;
  }
  if (HelperState(o) === HelperStates.suspendedYield) {
    return CreateIterResultObject(value, false);
  }
  RecordSet(o, HelperHeader.state, HelperStates.completed);
  return CreateIterResultObject(undefined, true);
}

/**
 * %IteratorHelperPrototype%.return ( ), 27.1.2.1.2.
 * @builtin %IteratorHelperPrototype%.return
 */
function IteratorHelperPrototypeReturn(this: Value): Obj {
  const o: Obj = ThisIteratorHelper(this, false);
  const state: i32 = HelperState(o);
  if (state === HelperStates.executing) {
    throw NewTypeError("cannot invoke a running iterator");
  }
  if (state === HelperStates.completed) {
    return CreateIterResultObject(undefined, true);
  }
  if (state === HelperStates.start) {
    RecordSet(o, HelperHeader.state, HelperStates.completed);
    if (HelperKind(o) === HelperKinds.zip) {
      ZipCloseAll(o);
      return CreateIterResultObject(undefined, true);
    }
    // concat has no underlying iterator before it starts.
    const iterator: Value = RecordGet(o, IteratorHelper.iterator);
    if (iterator !== undefined) {
      IteratorClose(AsObj(iterator));
    }
    return CreateIterResultObject(undefined, true);
  }
  RecordSet(o, HelperHeader.state, HelperStates.executing);
  try {
    IteratorHelperReturnAtYield(o);
  } finally {
    RecordSet(o, HelperHeader.state, HelperStates.completed);
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
  const wrapper: Obj = CreateIteratorHelper(IteratorHelper, HelperKinds.wrap);
  RecordSet(wrapper, IteratorHelper.iterator, iterator);
  RecordSet(wrapper, IteratorHelper.next, next);
  return wrapper;
}

/**
 * %WrapForValidIteratorPrototype%.next ( ), 27.1.3.2.2.1.1.
 * @builtin %WrapForValidIteratorPrototype%.next
 */
function WrapForValidIteratorPrototypeNext(this: Value): Value {
  const o: Obj = ThisIteratorHelper(this, true);
  return Call(RecordGet(o, IteratorHelper.next), RecordGet(o, IteratorHelper.iterator));
}

/**
 * %WrapForValidIteratorPrototype%.return ( ), 27.1.3.2.2.1.2.
 * @builtin %WrapForValidIteratorPrototype%.return
 */
function WrapForValidIteratorPrototypeReturn(this: Value): Value {
  const o: Obj = ThisIteratorHelper(this, true);
  const iterator: Obj = AsObj(RecordGet(o, IteratorHelper.iterator));
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
  const helper: Obj = CreateIteratorHelper(ConcatHelper, HelperKinds.concat);
  RecordSet(helper, ConcatHelper.iterables, iterables);
  RecordSet(helper, ConcatHelper.index, 0);
  return helper;
}

/**
 * Iterator.prototype.map ( mapper ), 27.1.4.8.
 * @builtin Iterator.prototype.map
 */
function IteratorPrototypeMap(this: Value, mapper: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, mapper);
  return NewCallbackHelper(CallbackHelper, HelperKinds.map, o, mapper);
}

/**
 * Iterator.prototype.filter ( predicate ), 27.1.4.5.
 * @builtin Iterator.prototype.filter
 */
function IteratorPrototypeFilter(this: Value, predicate: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, predicate);
  return NewCallbackHelper(CallbackHelper, HelperKinds.filter, o, predicate);
}

/**
 * Iterator.prototype.take ( limit ), 27.1.4.12.
 * @builtin Iterator.prototype.take
 */
function IteratorPrototypeTake(this: Value, limit: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  const remaining: f64 = HelperLimit(o, limit);
  const helper: Obj = NewIteratorHelper(LimitHelper, HelperKinds.take, o);
  RecordSet(helper, LimitHelper.remaining, remaining);
  return helper;
}

/**
 * Iterator.prototype.drop ( limit ), 27.1.4.4.
 * @builtin Iterator.prototype.drop
 */
function IteratorPrototypeDrop(this: Value, limit: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  const remaining: f64 = HelperLimit(o, limit);
  const helper: Obj = NewIteratorHelper(LimitHelper, HelperKinds.drop, o);
  RecordSet(helper, LimitHelper.remaining, remaining);
  return helper;
}

/**
 * Iterator.prototype.flatMap ( mapper ), 27.1.4.7.
 * @builtin Iterator.prototype.flatMap
 */
function IteratorPrototypeFlatMap(this: Value, mapper: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  HelperCallable(o, mapper);
  return NewCallbackHelper(FlatMapHelper, HelperKinds.flatMap, o, mapper);
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

/**
 * The size argument of chunks and windows (iterator-chunking): an integral Number in
 * [1, 2**32 - 1], else the receiver is closed and a TypeError or RangeError thrown.
 */
function ChunkingSize(o: Obj, size: Value, name: Str): f64 {
  let error: Value = undefined;
  // Integral means finite too.
  if (!IsNumber(size) || ToIntegerOrInfinityNumber(AsNumber(size)) !== AsNumber(size) || AsNumber(size) - AsNumber(size) !== 0) {
    error = NewTypeError(name + " is not an integral number");
  } else if (AsNumber(size) < 1 || AsNumber(size) > 4294967295) {
    error = NewRangeError(name + " out of range");
  }
  if (error !== undefined) {
    IteratorCloseThrow(o);
    throw error;
  }
  return AsNumber(size);
}

/**
 * Iterator.prototype.chunks ( chunkSize ) (iterator-chunking).
 * @builtin Iterator.prototype.chunks
 */
function IteratorPrototypeChunks(this: Value, chunkSize: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  const size: f64 = ChunkingSize(o, chunkSize, "chunkSize");
  const helper: Obj = NewIteratorHelper(ChunksHelper, HelperKinds.chunks, o);
  RecordSet(helper, ChunksHelper.size, size);
  return helper;
}

/**
 * Iterator.prototype.windows ( windowSize [ , undersized ] ) (iterator-chunking).
 * @builtin Iterator.prototype.windows
 * @length 1
 */
function IteratorPrototypeWindows(this: Value, windowSize: Value, undersized?: Value): Obj {
  const o: Obj = ThisIteratorObject(this);
  const size: f64 = ChunkingSize(o, windowSize, "windowSize");
  if (undersized !== undefined && undersized !== "only-full" && undersized !== "allow-partial") {
    const error: Obj = NewTypeError("bad undersized option");
    IteratorCloseThrow(o);
    throw error;
  }
  const helper: Obj = NewIteratorHelper(WindowsHelper, HelperKinds.windows, o);
  RecordSet(helper, WindowsHelper.size, size);
  RecordSet(helper, WindowsHelper.allowPartial, undersized === "allow-partial");
  return helper;
}

/**
 * Iterator.prototype.includes ( searchElement [ , skippedElements ] ) (iterator-includes).
 * @builtin Iterator.prototype.includes
 * @length 1
 */
function IteratorPrototypeIncludes(this: Value, searchElement: Value, skippedElements?: Value): boolean {
  const o: Obj = ThisIteratorObject(this);
  let toSkip: f64 = 0;
  if (skippedElements !== undefined) {
    let error: Value = undefined;
    if (!IsNumber(skippedElements)) {
      error = NewTypeError("not a number");
    } else {
      toSkip = AsNumber(skippedElements);
      if (toSkip !== toSkip || ToIntegerOrInfinityNumber(toSkip) !== toSkip) {
        error = NewTypeError("not an integral number");
      } else if (toSkip < 0 || (toSkip > 9007199254740991 && toSkip !== 1 / 0)) {
        error = NewRangeError("must be positive");
      }
    }
    if (error !== undefined) {
      IteratorCloseThrow(o);
      throw error;
    }
  }
  const next: Value = GetIteratorDirectNext(o);
  let skipped: f64 = 0;
  while (true) {
    const result: Value = IteratorStepResult(o, next);
    if (result === undefined) {
      return false;
    }
    const value: Value = Get(AsObj(result), "value");
    if (skipped < toSkip) {
      skipped++;
    } else if (SameValueZero(value, searchElement)) {
      IteratorClose(o);
      return true;
    }
  }
}

/**
 * Iterator.prototype.join ( separator ) (Iterator.prototype.join).
 * @builtin Iterator.prototype.join
 */
function IteratorPrototypeJoin(this: Value, separator: Value): Str {
  const o: Obj = ThisIteratorObject(this);
  let sep: Str = ",";
  if (separator !== undefined) {
    try {
      sep = ToString(separator);
    } catch (e) {
      IteratorCloseThrow(o);
      throw e;
    }
  }
  const next: Value = GetIteratorDirectNext(o);
  let r: Str = "";
  let first: boolean = true;
  while (true) {
    const result: Value = IteratorStepResult(o, next);
    if (result === undefined) {
      return r;
    }
    const value: Value = Get(AsObj(result), "value");
    if (!first) {
      r += sep;
    }
    first = false;
    if (value !== undefined && value !== null) {
      try {
        r += ToString(value);
      } catch (e) {
        IteratorCloseThrow(o);
        throw e;
      }
    }
  }
}

/** The options of Iterator.zip and zipKeyed (joint-iteration): a ZipSettings record. */
function ZipOptions(options: Value): Obj {
  let opts: Obj = ObjectCreate(null);
  if (options !== undefined) {
    if (!IsObject(options)) {
      throw NewTypeError("options must be an object");
    }
    opts = AsObj(options);
  }
  const mode: Value = Get(opts, "mode");
  let m: i32 = ZipModes.shortest;
  if (mode === "longest") {
    m = ZipModes.longest;
  } else if (mode === "strict") {
    m = ZipModes.strict;
  } else if (mode !== undefined && mode !== "shortest") {
    throw NewTypeError("invalid mode");
  }
  let padding: Value = undefined;
  if (m === ZipModes.longest) {
    padding = Get(opts, "padding");
    if (padding !== undefined && !IsObject(padding)) {
      throw NewTypeError("padding must be an object");
    }
  }
  const r: Obj = NewRecord(ZipSettings);
  RecordSet(r, ZipSettings.mode, m);
  RecordSet(r, ZipSettings.padding, padding);
  return r;
}

/**
 * Appends the iterator of `value` (GetIteratorFlattenable, reject-primitives, and its
 * `next`) to the zip lists; on an exception, closes `outer` (if any) after the
 * iterators so far and rethrows.
 */
function ZipAddIterator(iters: Obj, nexts: Obj, open: Obj, value: Value, outer: Value): void {
  const n: f64 = LengthOfArrayLike(iters);
  try {
    const iter: Obj = GetIteratorFlattenable(value, false);
    const next: Value = GetIteratorDirectNext(iter);
    CreateDataPropertyOrThrow(iters, n, iter);
    CreateDataPropertyOrThrow(nexts, n, next);
    CreateDataPropertyOrThrow(open, n, true);
  } catch (e) {
    // IfAbruptCloseIterators over « outer » followed by the iterators: in reverse.
    try {
      IteratorCloseAll(iters, open, e, true);
    } finally {
      if (outer !== undefined) {
        IteratorCloseThrow(AsObj(outer));
      }
    }
  }
}

/** The helper of IteratorZip ( iters, mode, padding, finishResults ). */
function ZipCreate(iters: Obj, nexts: Obj, open: Obj, padding: Obj, mode: i32, keys: Value): Obj {
  const helper: Obj = CreateIteratorHelper(ZipHelper, HelperKinds.zip);
  RecordSet(helper, ZipHelper.iterators, iters);
  RecordSet(helper, ZipHelper.nexts, nexts);
  RecordSet(helper, ZipHelper.padding, padding);
  RecordSet(helper, ZipHelper.mode, mode);
  RecordSet(helper, ZipHelper.open, open);
  RecordSet(helper, ZipHelper.keys, keys);
  return helper;
}

/**
 * Iterator.zip ( iterables [ , options ] ) (joint-iteration).
 * @builtin Iterator.zip
 * @length 1
 */
function IteratorZip(iterables: Value, options?: Value): Obj {
  if (!IsObject(iterables)) {
    throw NewTypeError("not an object");
  }
  const opts: Obj = ZipOptions(options);
  const mode: i32 = F64ToI32(AsNumber(RecordGet(opts, ZipSettings.mode)));
  const paddingOption: Value = RecordGet(opts, ZipSettings.padding);
  const iters: Obj = ArrayCreate(0);
  const nexts: Obj = ArrayCreate(0);
  const open: Obj = ArrayCreate(0);
  const inputIter: Obj = GetIterator(iterables);
  const inputNext: Value = Get(inputIter, "next");
  while (true) {
    let result: Value = undefined;
    let value: Value = undefined;
    try {
      result = IteratorStepResult(inputIter, inputNext);
      if (result !== undefined) {
        value = Get(AsObj(result), "value");
      }
    } catch (e) {
      IteratorCloseAll(iters, open, e, true);
    }
    if (result === undefined) {
      break;
    }
    ZipAddIterator(iters, nexts, open, value, inputIter);
  }
  const n: f64 = LengthOfArrayLike(iters);
  const padding: Obj = ArrayCreate(0);
  if (mode === ZipModes.longest) {
    if (paddingOption === undefined) {
      for (let i: f64 = 0; i < n; i++) {
        CreateDataPropertyOrThrow(padding, i, undefined);
      }
    } else {
      let paddingIter: Obj = iters;
      let paddingNext: Value = undefined;
      try {
        paddingIter = GetIterator(paddingOption);
        paddingNext = Get(paddingIter, "next");
      } catch (e) {
        IteratorCloseAll(iters, open, e, true);
      }
      let usingIterator: boolean = true;
      for (let i: f64 = 0; i < n; i++) {
        let value: Value = undefined;
        if (usingIterator) {
          try {
            const result: Value = IteratorStepResult(paddingIter, paddingNext);
            if (result === undefined) {
              usingIterator = false;
            } else {
              value = Get(AsObj(result), "value");
            }
          } catch (e) {
            IteratorCloseAll(iters, open, e, true);
          }
        }
        CreateDataPropertyOrThrow(padding, i, value);
      }
      if (usingIterator) {
        try {
          IteratorClose(paddingIter);
        } catch (e) {
          IteratorCloseAll(iters, open, e, true);
        }
      }
    }
  }
  return ZipCreate(iters, nexts, open, padding, mode, undefined);
}

/**
 * Iterator.zipKeyed ( iterables [ , options ] ) (joint-iteration).
 * @builtin Iterator.zipKeyed
 * @length 1
 */
function IteratorZipKeyed(iterables: Value, options?: Value): Obj {
  if (!IsObject(iterables)) {
    throw NewTypeError("not an object");
  }
  const source: Obj = AsObj(iterables);
  const opts: Obj = ZipOptions(options);
  const mode: i32 = F64ToI32(AsNumber(RecordGet(opts, ZipSettings.mode)));
  const paddingOption: Value = RecordGet(opts, ZipSettings.padding);
  const iters: Obj = ArrayCreate(0);
  const nexts: Obj = ArrayCreate(0);
  const open: Obj = ArrayCreate(0);
  const keys: Obj = ArrayCreate(0);
  const allKeys: Obj = OwnPropertyKeys(source, 2);
  const count: f64 = LengthOfArrayLike(allKeys);
  for (let k: f64 = 0; k < count; k++) {
    const key: Value = Get(allKeys, k);
    let value: Value = undefined;
    try {
      const attrs: i32 = OwnPropertyAttributes(source, key);
      if (attrs >= 0 && (attrs & 2) !== 0) {
        value = Get(source, key);
      }
    } catch (e) {
      IteratorCloseAll(iters, open, e, true);
    }
    if (value !== undefined) {
      ZipAddIterator(iters, nexts, open, value, undefined);
      CreateDataPropertyOrThrow(keys, LengthOfArrayLike(keys), key);
    }
  }
  const n: f64 = LengthOfArrayLike(iters);
  const padding: Obj = ArrayCreate(0);
  if (mode === ZipModes.longest) {
    for (let i: f64 = 0; i < n; i++) {
      let value: Value = undefined;
      if (paddingOption !== undefined) {
        try {
          value = Get(AsObj(paddingOption), Get(keys, i));
        } catch (e) {
          IteratorCloseAll(iters, open, e, true);
        }
      }
      CreateDataPropertyOrThrow(padding, i, value);
    }
  }
  return ZipCreate(iters, nexts, open, padding, mode, keys);
}
