// Async generators (ECMA-262 27.6) and async iteration (27.1.4, 27.1.6), on the
// suspendable frames of interp/run/suspend.rs (D1375, D1421).
//
// An async generator's body runs in frames resumed by AsyncGeneratorStep: by a request
// (next/return/throw) when it is suspended at the start or at a yield, or by the job of
// an awaited promise. When the frame hands control back, its state says why: awaiting
// (a job resumes it later), suspendedYield (the oldest request completes with the
// value), still executing (the body returned). An exception leaving the body rejects
// the oldest request. Requests are AsyncGeneratorRequest records, and their promises
// are internal capabilities.

/** A request of an async generator's queue (interp/run/suspend.rs keeps the queue). @record */
enum AsyncGeneratorRequest {
  /** @enum ResumeMode */
  type,
  value,
  promise,
}

/** The promise of a request for an async generator method (qjs's messages). */
function AsyncGeneratorEnqueueRequest(g: Value, type: i32, value: Value): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  const state: i32 = AsyncGeneratorState(g);
  if (state < 0) {
    RejectWithData(promise, NewTypeError("not an AsyncGenerator object"));
    return promise;
  }
  const gen: Obj = AsObj(g);
  if (type === ResumeMode.next && state === GeneratorStates.completed) {
    ResolveWithData(promise, CreateIterResultObject(undefined, true));
    return promise;
  }
  if (type === ResumeMode.throw && (state === GeneratorStates.start || state === GeneratorStates.completed)) {
    GeneratorSetState(gen, GeneratorStates.completed);
    RejectWithData(promise, value);
    return promise;
  }
  const request: Obj = NewRecord(AsyncGeneratorRequest);
  RecordSet(request, AsyncGeneratorRequest.type, type);
  RecordSet(request, AsyncGeneratorRequest.value, value);
  RecordSet(request, AsyncGeneratorRequest.promise, promise);
  AsyncGeneratorEnqueue(gen, request);
  if (type === ResumeMode.return && (state === GeneratorStates.start || state === GeneratorStates.completed)) {
    GeneratorSetState(gen, GeneratorStates.awaitingReturn);
    AsyncGeneratorAwaitReturn(gen);
  } else if (state === GeneratorStates.start || state === GeneratorStates.suspendedYield) {
    AsyncGeneratorStep(gen, value, type);
  }
  return promise;
}

/**
 * %AsyncGeneratorPrototype%.next ( value ), 27.6.1.2.
 * @builtin %AsyncGeneratorPrototype%.next
 */
function AsyncGeneratorPrototypeNext(this: Value, value: Value): Obj {
  return AsyncGeneratorEnqueueRequest(this, ResumeMode.next, value);
}

/**
 * %AsyncGeneratorPrototype%.return ( value ), 27.6.1.3.
 * @builtin %AsyncGeneratorPrototype%.return
 */
function AsyncGeneratorPrototypeReturn(this: Value, value: Value): Obj {
  return AsyncGeneratorEnqueueRequest(this, ResumeMode.return, value);
}

/**
 * %AsyncGeneratorPrototype%.throw ( exception ), 27.6.1.4.
 * @builtin %AsyncGeneratorPrototype%.throw
 */
function AsyncGeneratorPrototypeThrow(this: Value, exception: Value): Obj {
  return AsyncGeneratorEnqueueRequest(this, ResumeMode.throw, exception);
}

/**
 * Runs the generator's frame from a suspension with mode 0 (a value), 1 (throw) or 2
 * (return, at a yield), until it hands control back, and settles requests accordingly:
 * AsyncGeneratorResume, AsyncGeneratorYield, AsyncGeneratorStart's completion steps.
 * @intrinsic
 */
function AsyncGeneratorStep(g: Obj, value: Value, mode: i32): void {
  let v: Value = value;
  let m: i32 = mode;
  while (true) {
    let result: Value = undefined;
    try {
      result = GeneratorResume(g, v, m);
    } catch (e) {
      GeneratorSetState(g, GeneratorStates.completed);
      AsyncGeneratorCompleteStep(g, ResumeMode.throw, e, true);
      AsyncGeneratorDrainQueue(g);
      return;
    }
    const state: i32 = AsyncGeneratorState(g);
    if (state === GeneratorStates.awaiting) {
      return;
    }
    if (state !== GeneratorStates.suspendedYield) {
      GeneratorSetState(g, GeneratorStates.completed);
      AsyncGeneratorCompleteStep(g, ResumeMode.next, result, true);
      AsyncGeneratorDrainQueue(g);
      return;
    }
    // A yield: executing while the request settles (user code may enqueue more).
    GeneratorSetState(g, GeneratorStates.executing);
    AsyncGeneratorCompleteStep(g, ResumeMode.next, result, false);
    GeneratorSetState(g, GeneratorStates.suspendedYield);
    const next: Value = AsyncGeneratorFirstRequest(g);
    if (next === undefined) {
      return;
    }
    v = RecordGet(AsObj(next), AsyncGeneratorRequest.value);
    m = F64ToI32(AsNumber(RecordGet(AsObj(next), AsyncGeneratorRequest.type)));
  }
}

/**
 * AsyncGeneratorCompleteStep ( generator, completion, done ), 27.6.3.4: settles the
 * oldest request (type 1 rejects).
 */
function AsyncGeneratorCompleteStep(g: Obj, type: i32, value: Value, done: boolean): void {
  const request: Obj = AsObj(AsyncGeneratorDequeue(g));
  const promise: Value = RecordGet(request, AsyncGeneratorRequest.promise);
  if (type === ResumeMode.throw) {
    RejectWithData(promise, value);
  } else {
    ResolveWithData(promise, CreateIterResultObject(value, done));
  }
}

/** AsyncGeneratorDrainQueue ( generator ), 27.6.3.10: after completion. */
function AsyncGeneratorDrainQueue(g: Obj): void {
  while (true) {
    const next: Value = AsyncGeneratorFirstRequest(g);
    if (next === undefined) {
      return;
    }
    const type: f64 = AsNumber(RecordGet(AsObj(next), AsyncGeneratorRequest.type));
    if (type === ResumeMode.return) {
      GeneratorSetState(g, GeneratorStates.awaitingReturn);
      AsyncGeneratorAwaitReturn(g);
      return;
    }
    if (type === ResumeMode.throw) {
      AsyncGeneratorCompleteStep(g, ResumeMode.throw, RecordGet(AsObj(next), AsyncGeneratorRequest.value), true);
    } else {
      AsyncGeneratorCompleteStep(g, ResumeMode.next, undefined, true);
    }
  }
}

/** AsyncGeneratorAwaitReturn ( generator ), 27.6.3.9: awaits the return() value. */
function AsyncGeneratorAwaitReturn(g: Obj): void {
  const next: Obj = AsObj(AsyncGeneratorFirstRequest(g));
  let promise: Value = undefined;
  try {
    promise = PromiseResolve(IntrinsicPromise(), RecordGet(next, AsyncGeneratorRequest.value));
  } catch (e) {
    GeneratorSetState(g, GeneratorStates.completed);
    AsyncGeneratorCompleteStep(g, ResumeMode.throw, e, true);
    AsyncGeneratorDrainQueue(g);
    return;
  }
  PerformPromiseThen(AsObj(promise), NewBuiltinClosure(11, g, 1), NewBuiltinClosure(12, g, 1), undefined);
}

/**
 * The fulfilled step of AsyncGeneratorAwaitReturn: `this` is the generator.
 * @intrinsic
 */
function AsyncGeneratorAwaitReturnFulfilled(this: Value, value: Value): void {
  const g: Obj = AsObj(this);
  GeneratorSetState(g, GeneratorStates.completed);
  AsyncGeneratorCompleteStep(g, ResumeMode.next, value, true);
  AsyncGeneratorDrainQueue(g);
}

/**
 * The rejected step of AsyncGeneratorAwaitReturn.
 * @intrinsic
 */
function AsyncGeneratorAwaitReturnRejected(this: Value, reason: Value): void {
  const g: Obj = AsObj(this);
  GeneratorSetState(g, GeneratorStates.completed);
  AsyncGeneratorCompleteStep(g, ResumeMode.throw, reason, true);
  AsyncGeneratorDrainQueue(g);
}

// ---- Async iteration ---------------------------------------------------------------------

/**
 * %AsyncIteratorPrototype% [ %Symbol.asyncIterator% ] ( ), 27.1.4.1.
 * @builtin %AsyncIteratorPrototype%[Symbol.asyncIterator]
 */
function AsyncIteratorPrototypeAsyncIterator(this: Value): Value {
  return this;
}

/**
 * GetIterator ( obj, async ), 7.4.3, for `for await`: the iterator (a sync one wrapped
 * by CreateAsyncFromSyncIterator). The caller reads `next` once, right after.
 * @intrinsic
 */
function GetAsyncIterator(obj: Value): Obj {
  const method: Value = GetMethod(obj, WellKnownSymbol("asyncIterator"));
  if (method === undefined) {
    const syncMethod: Value = GetMethod(obj, WellKnownSymbol("iterator"));
    // qjs calls the missing method.
    if (syncMethod === undefined) {
      throw NewTypeError("not a function");
    }
    const syncIterator: Obj = GetIteratorFromMethod(obj, syncMethod);
    return CreateAsyncFromSyncIterator(syncIterator, Get(syncIterator, "next"));
  }
  return GetIteratorFromMethod(obj, method);
}

/**
 * The `return` method of an iterator (GetMethod), or undefined: `for await` closes with
 * a call of it, an await and an object check (AsyncIteratorClose, 7.4.13, normal
 * completion).
 * @intrinsic
 */
function AsyncIteratorReturnMethod(iterator: Obj): Value {
  return GetMethod(iterator, "return");
}

/**
 * AsyncIteratorClose for a throw completion: calls `return` and gives a promise to
 * await that always fulfills (errors and the result are ignored; the original
 * exception wins).
 * @intrinsic
 */
function AsyncIteratorCloseThrow(iterator: Obj): Value {
  let promise: Value = undefined;
  try {
    const method: Value = GetMethod(iterator, "return");
    if (method === undefined) {
      return undefined;
    }
    promise = PromiseResolve(IntrinsicPromise(), Call(method, iterator));
  } catch (e) {
    return undefined;
  }
  const result: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  const ignore: Fn = NewBuiltinClosure(9, undefined, 0);
  PerformPromiseThen(AsObj(promise), ignore, ignore, result);
  return result;
}

/**
 * AsyncFromSyncIteratorContinuation ( result, promiseCapability, syncIteratorRecord,
 * closeOnRejection ), 27.1.6.4.
 */
function AsyncFromSyncIteratorContinuation(result: Obj, iterator: Obj, closeOnRejection: boolean): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  let done: boolean = false;
  let valueWrapper: Value = undefined;
  try {
    done = ToBoolean(Get(result, "done"));
    const value: Value = Get(result, "value");
    try {
      valueWrapper = PromiseResolve(IntrinsicPromise(), value);
    } catch (e) {
      if (!done && closeOnRejection) {
        IteratorCloseThrow(iterator);
      }
      throw e;
    }
  } catch (e) {
    RejectWithData(promise, e);
    return promise;
  }
  const onFulfilled: Fn = NewBuiltinClosure(13, done, 1);
  let onRejected: Value = undefined;
  if (!done && closeOnRejection) {
    onRejected = NewBuiltinClosure(14, iterator, 1);
  }
  PerformPromiseThen(AsObj(valueWrapper), onFulfilled, onRejected, promise);
  return promise;
}

/**
 * The unwrap function of AsyncFromSyncIteratorContinuation: `this` is `done`.
 * @intrinsic
 */
function AsyncFromSyncIteratorUnwrap(this: Value, value: Value): Obj {
  return CreateIterResultObject(value, AsBool(this));
}

/**
 * The close-on-rejection function: `this` is the sync iterator; closes it and rethrows.
 * @intrinsic
 */
function AsyncFromSyncIteratorCloseThrow(this: Value, error: Value): Value {
  IteratorCloseThrow(AsObj(this));
  throw error;
}

/**
 * %AsyncFromSyncIteratorPrototype%.next ( [ value ] ), 27.1.6.2.1.
 * @builtin %AsyncFromSyncIteratorPrototype%.next
 * @length 1
 */
function AsyncFromSyncIteratorNext(this: Value, ...args: Value[]): Obj {
  const o: Obj = AsObj(this);
  const iterator: Obj = AsObj(RecordGet(o, AsyncFromSyncIterator.iterator));
  let result: Value = undefined;
  try {
    result = CallWithArray(RecordGet(o, AsyncFromSyncIterator.next), iterator, args);
    if (!IsObject(result)) {
      throw NewTypeError("iterator must return an object");
    }
  } catch (e) {
    const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
    RejectWithData(promise, e);
    return promise;
  }
  return AsyncFromSyncIteratorContinuation(AsObj(result), iterator, true);
}

/**
 * %AsyncFromSyncIteratorPrototype%.return ( [ value ] ), 27.1.6.2.2.
 * @builtin %AsyncFromSyncIteratorPrototype%.return
 * @length 1
 */
function AsyncFromSyncIteratorReturn(this: Value, ...args: Value[]): Obj {
  const iterator: Obj = AsObj(RecordGet(AsObj(this), AsyncFromSyncIterator.iterator));
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  let result: Value = undefined;
  try {
    const method: Value = GetMethod(iterator, "return");
    if (method === undefined) {
      ResolveWithData(promise, CreateIterResultObject(Get(args, 0), true));
      return promise;
    }
    result = CallWithArray(method, iterator, args);
    if (!IsObject(result)) {
      throw NewTypeError("iterator must return an object");
    }
  } catch (e) {
    RejectWithData(promise, e);
    return promise;
  }
  return AsyncFromSyncIteratorContinuation(AsObj(result), iterator, false);
}

/**
 * %AsyncFromSyncIteratorPrototype%.throw ( [ value ] ), 27.1.6.2.3: without a `throw`
 * method the sync iterator is closed and the promise rejects with a TypeError.
 * @builtin %AsyncFromSyncIteratorPrototype%.throw
 * @length 1
 */
function AsyncFromSyncIteratorThrow(this: Value, ...args: Value[]): Obj {
  const iterator: Obj = AsObj(RecordGet(AsObj(this), AsyncFromSyncIterator.iterator));
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  let result: Value = undefined;
  try {
    const method: Value = GetMethod(iterator, "throw");
    if (method === undefined) {
      IteratorClose(iterator);
      throw NewTypeError("throw is not a method");
    }
    result = CallWithArray(method, iterator, args);
    if (!IsObject(result)) {
      throw NewTypeError("iterator must return an object");
    }
  } catch (e) {
    RejectWithData(promise, e);
    return promise;
  }
  return AsyncFromSyncIteratorContinuation(AsObj(result), iterator, true);
}
