// Array.fromAsync (ES2026 23.1.2.2, 0051). The spec runs an abstract closure as an
// async function; the builtin language has no `await`, so the closure is a state
// machine over a FromAsyncState record (D1511) whose awaits subscribe two builtin
// closures (kinds 16 and 17 of NewBuiltinClosure) that resume it.

/** What the pending await of Array.fromAsync resumes. @constants */
enum FromAsyncPhases {
  /** Awaiting `next()`'s result. */
  next = 1,
  /** Awaiting a mapped value. */
  mapped = 2,
  /** Awaiting the iterator's `return()` before rejecting. */
  closing = 3,
  /** Awaiting an array-like element. */
  element = 4,
  /** Awaiting an array-like element's mapped value. */
  elementMapped = 5,
}

/**
 * Array.fromAsync's closure: the result promise, mapfn (undefined: no mapping),
 * thisArg, the iterator (undefined for an array-like), its `next`, the array A, k, the
 * phase the pending await resumes, the array-like, its length, the error that rejects
 * once the iterator is closed.
 * @record
 */
enum FromAsyncState {
  promise,
  mapfn,
  thisArg,
  iterator,
  next,
  array,
  k,
  /** @enum FromAsyncPhases */
  phase,
  arrayLike,
  length,
  error,
}

/**
 * Array.fromAsync ( asyncItems [ , mapfn [ , thisArg ] ] ).
 * @builtin Array.fromAsync
 * @length 1
 */
function ArrayFromAsync(this: Value, asyncItems: Value, mapfn?: Value, thisArg?: Value): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  try {
    FromAsyncStart(this, promise, asyncItems, mapfn, thisArg);
  } catch (e) {
    RejectPromise(promise, e);
  }
  return promise;
}

/** The closure up to its first await. */
function FromAsyncStart(c: Value, promise: Obj, asyncItems: Value, mapfn: Value, thisArg: Value): void {
  if (mapfn !== undefined && !IsCallable(mapfn)) {
    throw NewTypeError("mapfn is not callable");
  }
  const s: Obj = NewRecord(FromAsyncState);
  RecordSet(s, FromAsyncState.promise, promise);
  RecordSet(s, FromAsyncState.mapfn, mapfn);
  RecordSet(s, FromAsyncState.thisArg, thisArg);
  RecordSet(s, FromAsyncState.k, 0);
  const usingAsyncIterator: Value = GetMethod(asyncItems, WellKnownSymbol("asyncIterator"));
  let usingSyncIterator: Value = undefined;
  if (usingAsyncIterator === undefined) {
    usingSyncIterator = GetMethod(asyncItems, WellKnownSymbol("iterator"));
  }
  if (usingAsyncIterator !== undefined || usingSyncIterator !== undefined) {
    let iterator: Obj = s;
    if (usingAsyncIterator !== undefined) {
      iterator = GetIteratorFromMethod(asyncItems, usingAsyncIterator);
    } else {
      const syncIterator: Obj = GetIteratorFromMethod(asyncItems, usingSyncIterator);
      iterator = CreateAsyncFromSyncIterator(syncIterator, Get(syncIterator, "next"));
    }
    RecordSet(s, FromAsyncState.iterator, iterator);
    RecordSet(s, FromAsyncState.next, Get(iterator, "next"));
    RecordSet(s, FromAsyncState.array, IsConstructor(c) ? Construct(c) : ArrayCreate(0));
    FromAsyncIteratorNext(s);
    return;
  }
  const arrayLike: Obj = ToObject(asyncItems);
  const len: f64 = LengthOfArrayLike(arrayLike);
  RecordSet(s, FromAsyncState.array, IsConstructor(c) ? Construct(c, len) : ArrayCreate(len));
  RecordSet(s, FromAsyncState.arrayLike, arrayLike);
  RecordSet(s, FromAsyncState.length, len);
  FromAsyncArrayLikeNext(s);
}

/** Await ( v ): resumes s at `phase` with the outcome. PromiseResolve may throw. */
function FromAsyncAwait(s: Obj, v: Value, phase: i32): void {
  RecordSet(s, FromAsyncState.phase, phase);
  const promise: Value = PromiseResolve(IntrinsicPromise(), v);
  PerformPromiseThen(AsObj(promise), NewBuiltinClosure(16, s, 1), NewBuiltinClosure(17, s, 1), undefined);
}

/** Rejects the result promise. */
function FromAsyncReject(s: Obj, e: Value): void {
  RejectPromise(AsObj(RecordGet(s, FromAsyncState.promise)), e);
}

/** Step iv.1-4 of the iterator loop: call `next` and await its result. */
function FromAsyncIteratorNext(s: Obj): void {
  try {
    if (AsNumber(RecordGet(s, FromAsyncState.k)) >= 9007199254740991) {
      FromAsyncClose(s, NewTypeError("Array too long"));
      return;
    }
    const result: Value = Call(RecordGet(s, FromAsyncState.next), RecordGet(s, FromAsyncState.iterator));
    FromAsyncAwait(s, result, FromAsyncPhases.next);
  } catch (e) {
    FromAsyncReject(s, e);
  }
}

/**
 * AsyncIteratorClose ( iteratorRecord, ThrowCompletion ( error ) ), 7.4.12: calls
 * `return` and awaits its result, ignoring errors, then rejects with error.
 */
function FromAsyncClose(s: Obj, error: Value): void {
  RecordSet(s, FromAsyncState.error, error);
  try {
    const iterator: Obj = AsObj(RecordGet(s, FromAsyncState.iterator));
    const method: Value = GetMethod(iterator, "return");
    if (method === undefined) {
      FromAsyncReject(s, error);
      return;
    }
    FromAsyncAwait(s, Call(method, iterator), FromAsyncPhases.closing);
  } catch (e) {
    FromAsyncReject(s, error);
  }
}

/** Steps iv.5-10 once `next()`'s result is in. */
function FromAsyncIteratorResult(s: Obj, nextResult: Value): void {
  let value: Value = undefined;
  try {
    if (!IsObject(nextResult)) {
      throw NewTypeError("iterator must return an object");
    }
    if (ToBoolean(Get(AsObj(nextResult), "done"))) {
      Set(AsObj(RecordGet(s, FromAsyncState.array)), "length", RecordGet(s, FromAsyncState.k));
      ResolvePromise(AsObj(RecordGet(s, FromAsyncState.promise)), RecordGet(s, FromAsyncState.array));
      return;
    }
    value = Get(AsObj(nextResult), "value");
  } catch (e) {
    FromAsyncReject(s, e);
    return;
  }
  const mapfn: Value = RecordGet(s, FromAsyncState.mapfn);
  if (mapfn === undefined) {
    FromAsyncIteratorStore(s, value);
    return;
  }
  try {
    const mapped: Value = Call(mapfn, RecordGet(s, FromAsyncState.thisArg), value, RecordGet(s, FromAsyncState.k));
    FromAsyncAwait(s, mapped, FromAsyncPhases.mapped);
  } catch (e) {
    FromAsyncClose(s, e);
  }
}

/** Steps iv.11-13: store the (mapped) value, then the next iteration. */
function FromAsyncIteratorStore(s: Obj, value: Value): void {
  const k: f64 = AsNumber(RecordGet(s, FromAsyncState.k));
  try {
    CreateDataPropertyOrThrow(AsObj(RecordGet(s, FromAsyncState.array)), k, value);
  } catch (e) {
    FromAsyncClose(s, e);
    return;
  }
  RecordSet(s, FromAsyncState.k, k + 1);
  FromAsyncIteratorNext(s);
}

/** The array-like loop (step 3.i.vii): await the next element, or finish. */
function FromAsyncArrayLikeNext(s: Obj): void {
  try {
    const k: f64 = AsNumber(RecordGet(s, FromAsyncState.k));
    const len: f64 = AsNumber(RecordGet(s, FromAsyncState.length));
    if (k < len) {
      FromAsyncAwait(s, Get(AsObj(RecordGet(s, FromAsyncState.arrayLike)), k), FromAsyncPhases.element);
      return;
    }
    Set(AsObj(RecordGet(s, FromAsyncState.array)), "length", len);
    ResolvePromise(AsObj(RecordGet(s, FromAsyncState.promise)), RecordGet(s, FromAsyncState.array));
  } catch (e) {
    FromAsyncReject(s, e);
  }
}

/** An array-like element (phase 4) or its mapped value (phase 5) is in. */
function FromAsyncArrayLikeValue(s: Obj, value: Value, phase: i32): void {
  try {
    const k: f64 = AsNumber(RecordGet(s, FromAsyncState.k));
    const mapfn: Value = RecordGet(s, FromAsyncState.mapfn);
    if (phase === FromAsyncPhases.element && mapfn !== undefined) {
      FromAsyncAwait(s, Call(mapfn, RecordGet(s, FromAsyncState.thisArg), value, k), FromAsyncPhases.elementMapped);
      return;
    }
    CreateDataPropertyOrThrow(AsObj(RecordGet(s, FromAsyncState.array)), k, value);
    RecordSet(s, FromAsyncState.k, k + 1);
  } catch (e) {
    FromAsyncReject(s, e);
    return;
  }
  FromAsyncArrayLikeNext(s);
}

/**
 * Resumes Array.fromAsync's closure after a fulfilled await: `this` is its FromAsyncState.
 * @intrinsic
 */
function FromAsyncFulfilled(this: Value, value: Value): void {
  const s: Obj = AsObj(this);
  const phase: i32 = F64ToI32(AsNumber(RecordGet(s, FromAsyncState.phase)));
  if (phase === FromAsyncPhases.next) {
    FromAsyncIteratorResult(s, value);
  } else if (phase === FromAsyncPhases.mapped) {
    FromAsyncIteratorStore(s, value);
  } else if (phase === FromAsyncPhases.closing) {
    FromAsyncReject(s, RecordGet(s, FromAsyncState.error));
  } else {
    FromAsyncArrayLikeValue(s, value, phase);
  }
}

/**
 * Resumes Array.fromAsync's closure after a rejected await: `this` is its FromAsyncState. A
 * rejected mapped value closes the iterator (IfAbruptCloseAsyncIterator).
 * @intrinsic
 */
function FromAsyncRejected(this: Value, reason: Value): void {
  const s: Obj = AsObj(this);
  const phase: i32 = F64ToI32(AsNumber(RecordGet(s, FromAsyncState.phase)));
  if (phase === FromAsyncPhases.mapped) {
    FromAsyncClose(s, reason);
  } else if (phase === FromAsyncPhases.closing) {
    FromAsyncReject(s, RecordGet(s, FromAsyncState.error));
  } else {
    FromAsyncReject(s, reason);
  }
}
