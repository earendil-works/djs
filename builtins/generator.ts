// Generator objects (ECMA-262 27.5) and the await/return steps of async functions
// (27.7), over the suspendable frames of interp/run/suspend.rs (D1375). The bytecode
// ops generator_start/yield/await/async_return/async_reject suspend and finish frames;
// these builtins resume them and settle the result promises.

/** GeneratorValidate ( generator, brand ), 27.5.3.2: the state (qjs's messages). */
function GeneratorValidate(generator: Value): i32 {
  const state: i32 = GeneratorState(generator);
  if (state < 0) {
    throw NewTypeError("not a generator");
  }
  if (state === 2) {
    throw NewTypeError("cannot invoke a running generator");
  }
  return state;
}

/**
 * Resumes generator g (suspended) with mode 0 next, 1 throw, 2 return, and makes the
 * iterator result: a yield leaves it suspended (done false); a return or a throw
 * completes it.
 */
function GeneratorStep(g: Obj, value: Value, mode: i32): Obj {
  let result: Value = undefined;
  try {
    result = GeneratorResume(g, value, mode);
  } catch (e) {
    GeneratorSetState(g, 3);
    throw e;
  }
  if (GeneratorState(g) === 1) {
    return CreateIterResultObject(result, false);
  }
  GeneratorSetState(g, 3);
  return CreateIterResultObject(result, true);
}

/**
 * %GeneratorPrototype%.next ( value ), 27.5.1.2.
 * @builtin %GeneratorPrototype%.next
 */
function GeneratorPrototypeNext(this: Value, value: Value): Obj {
  const state: i32 = GeneratorValidate(this);
  if (state === 3) {
    return CreateIterResultObject(undefined, true);
  }
  return GeneratorStep(AsObj(this), value, 0);
}

/**
 * %GeneratorPrototype%.return ( value ), 27.5.1.3.
 * @builtin %GeneratorPrototype%.return
 */
function GeneratorPrototypeReturn(this: Value, value: Value): Obj {
  const state: i32 = GeneratorValidate(this);
  if (state === 0 || state === 3) {
    GeneratorSetState(AsObj(this), 3);
    return CreateIterResultObject(value, true);
  }
  return GeneratorStep(AsObj(this), value, 2);
}

/**
 * %GeneratorPrototype%.throw ( exception ), 27.5.1.4.
 * @builtin %GeneratorPrototype%.throw
 */
function GeneratorPrototypeThrow(this: Value, exception: Value): Obj {
  const state: i32 = GeneratorValidate(this);
  if (state === 0 || state === 3) {
    GeneratorSetState(AsObj(this), 3);
    throw exception;
  }
  return GeneratorStep(AsObj(this), exception, 1);
}

/**
 * %GeneratorPrototype% [ %Symbol.toStringTag% ], 27.5.1.5.
 * @value %GeneratorPrototype%[Symbol.toStringTag]
 * @attributes configurable
 */
function GeneratorPrototypeToStringTag(): Str {
  return "Generator";
}

/**
 * Await ( value ), 27.7.5.3, after the `await` op suspended the frame: subscribes the
 * async state to the promise and returns the function's result promise to whoever
 * runs it (its caller the first time, the resuming job later). If PromiseResolve
 * throws, the function continues at once with that exception.
 * @intrinsic
 */
function AsyncAwait(a: Obj, v: Value): Value {
  let promise: Value = undefined;
  try {
    promise = PromiseResolve(IntrinsicPromise(), v);
  } catch (e) {
    return GeneratorResume(a, e, 1);
  }
  PerformPromiseThen(AsObj(promise), a, a, undefined);
  return AsyncFunctionPromise(a);
}

/**
 * An async function returns v (`async_return`): resolves its promise, returns it.
 * @intrinsic
 */
function AsyncFunctionReturn(a: Obj, v: Value): Value {
  GeneratorSetState(a, 3);
  const promise: Obj = AsyncFunctionPromise(a);
  ResolvePromise(promise, v);
  return promise;
}

/**
 * An exception e leaves an async function (`async_reject`): rejects its promise.
 * @intrinsic
 */
function AsyncFunctionThrow(a: Obj, e: Value): Value {
  GeneratorSetState(a, 3);
  const promise: Obj = AsyncFunctionPromise(a);
  RejectPromise(promise, e);
  return promise;
}
