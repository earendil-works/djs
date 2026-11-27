// Explicit resource management, the runtime parts (ES2026 27.3, 27.4, 20.5.8, 0051):
// DisposableStack, AsyncDisposableStack, SuppressedError, and the dispose methods of
// %IteratorPrototype% and %AsyncIteratorPrototype%. The `using` syntax is a compiler
// issue of its own.
//
// A stack is a DisposableStack object (heap class 33; its slots below): whether it is
// an AsyncDisposableStack, its state, the resources, an internal Array of [value,
// method, kind] triples, oldest first (kinds: ResourceKinds, D1512).

/** A DisposableStack's or AsyncDisposableStack's slots (heap/object.rs). @slots DisposableStack */
enum DisposableStackSlots {
  async,
  /** @enum DisposableStates */
  state,
  resources,
}

/** A DisposableStack's `state`. @constants */
enum DisposableStates {
  pending = 0,
  disposed = 1,
}

/**
 * How a resource triple is disposed (D1512). The kinds are stored in the resource
 * Arrays (plain elements, not named in a snapshot).
 * @constants
 */
enum ResourceKinds {
  /** Calls method with `this` = value. */
  method = 0,
  /** Calls method(value) (adopt's closure). */
  adopted = 1,
  /**
   * An AsyncDisposableStack resource with only a Symbol.dispose method (GetDisposeMethod's
   * closure: the call's outcome is awaited).
   */
  syncInAsync = 2,
  /** An AsyncDisposableStack resource of null or undefined (no method; only forces an await). */
  none = 3,
  /** An `await using` resource with a Symbol.asyncDispose method, whose result is awaited. */
  awaited = 4,
}

/**
 * The errors of a disposal: the one to throw (or reject with) and whether there is one.
 * AsyncDisposeState and UsingRecord start with them.
 * @record
 */
enum DisposeErrors {
  error,
  hasError,
}

/** The SuppressedError DisposeResources throws when a disposal throws after another. */
function NewSuppressedError(error: Value, suppressed: Value): Obj {
  const o: Obj = ErrorCreate(RealmIntrinsic("SuppressedErrorPrototype"), undefined);
  DefineDataPropertyOrThrow(o, "error", error, 5);
  DefineDataPropertyOrThrow(o, "suppressed", suppressed, 5);
  RecordStack(o);
  return o;
}

/** The stack `this` of a method, still pending (qjs's ReferenceError otherwise). */
function PendingDisposableStack(v: Value, isAsync: boolean): Obj {
  const stack: Obj = ThisDisposableStack(v, isAsync);
  if (F64ToI32(AsNumber(RecordGet(stack, DisposableStackSlots.state))) !== DisposableStates.pending) {
    throw NewReferenceError(isAsync ? "AsyncDisposableStack is disposed" : "DisposableStack is disposed");
  }
  return stack;
}

/** Appends a resource triple to a stack's (or a `using` scope's) resources. */
function AddResource(resources: Obj, value: Value, method: Value, kind: i32): void {
  const n: f64 = LengthOfArrayLike(resources);
  CreateDataPropertyOrThrow(resources, n, value);
  CreateDataPropertyOrThrow(resources, n + 1, method);
  CreateDataPropertyOrThrow(resources, n + 2, kind);
}

/**
 * AddDisposableResource ( disposeCapability, V, hint ), 27.3.3.1 (without a method):
 * CreateDisposableResource and GetDisposeMethod for `use`.
 */
function AddDisposableValue(resources: Obj, v: Value, isAsync: boolean): void {
  if (v === undefined || v === null) {
    if (isAsync) {
      AddResource(resources, undefined, undefined, ResourceKinds.none);
    }
    return;
  }
  if (!IsObject(v)) {
    throw NewTypeError("using: value is not an object");
  }
  let kind: i32 = ResourceKinds.method;
  let method: Value = undefined;
  if (isAsync) {
    method = GetV(v, WellKnownSymbol("asyncDispose"));
  }
  if (method === undefined || method === null) {
    method = GetV(v, WellKnownSymbol("dispose"));
    if (isAsync) {
      kind = ResourceKinds.syncInAsync;
    }
    if (method === undefined || method === null) {
      throw NewTypeError(isAsync ? "using: value has no Symbol.asyncDispose or Symbol.dispose method" : "using: value has no Symbol.dispose method");
    }
  }
  if (!IsCallable(method)) {
    throw NewTypeError("using: dispose method is not a function");
  }
  AddResource(resources, v, method, kind);
}

/** The resources of a pending stack. */
function StackResources(stack: Obj): Obj {
  return AsObj(RecordGet(stack, DisposableStackSlots.resources));
}

/** Calls the disposal of resource triple i (kinds method, adopted, syncInAsync). */
function CallDisposeMethod(resources: Obj, i: f64): Value {
  const value: Value = Get(resources, i);
  const method: Value = Get(resources, i + 1);
  if (F64ToI32(AsNumber(Get(resources, i + 2))) === ResourceKinds.adopted) {
    return Call(method, undefined, value);
  }
  return Call(method, value);
}

/**
 * DisposableStack ( ), 27.3.1.1.
 * @constructor DisposableStack
 */
function DisposableStackConstructor(): Obj {
  if (new.target === undefined) {
    throw NewTypeError("must be called with new");
  }
  return DisposableStackCreate(GetPrototypeFromConstructor(new.target, "DisposableStackPrototype"), false);
}

/**
 * get DisposableStack.prototype.disposed, 27.3.3.3.
 * @getter DisposableStack.prototype.disposed
 */
function DisposableStackPrototypeDisposed(this: Value): boolean {
  return F64ToI32(AsNumber(RecordGet(ThisDisposableStack(this, false), DisposableStackSlots.state))) === DisposableStates.disposed;
}

/**
 * DisposableStack.prototype.dispose ( ), 27.3.3.2; also [ %Symbol.dispose% ].
 * DisposeResources ( disposeCapability, NormalCompletion ), 27.3.3.4.
 * @builtin DisposableStack.prototype.dispose
 * @builtin DisposableStack.prototype[Symbol.dispose]
 */
function DisposableStackPrototypeDispose(this: Value): void {
  const stack: Obj = ThisDisposableStack(this, false);
  if (F64ToI32(AsNumber(RecordGet(stack, DisposableStackSlots.state))) === DisposableStates.disposed) {
    return;
  }
  RecordSet(stack, DisposableStackSlots.state, DisposableStates.disposed);
  const resources: Obj = AsObj(RecordGet(stack, DisposableStackSlots.resources));
  RecordSet(stack, DisposableStackSlots.resources, ArrayCreate(0));
  let hasError: boolean = false;
  let error: Value = undefined;
  for (let i: f64 = LengthOfArrayLike(resources) - 3; i >= 0; i = i - 3) {
    try {
      CallDisposeMethod(resources, i);
    } catch (e) {
      error = hasError ? NewSuppressedError(e, error) : e;
      hasError = true;
    }
  }
  if (hasError) {
    throw error;
  }
}

/**
 * DisposableStack.prototype.use ( value ), 27.3.3.6.
 * @builtin DisposableStack.prototype.use
 */
function DisposableStackPrototypeUse(this: Value, value: Value): Value {
  AddDisposableValue(StackResources(PendingDisposableStack(this, false)), value, false);
  return value;
}

/**
 * DisposableStack.prototype.adopt ( value, onDispose ), 27.3.3.1.
 * @builtin DisposableStack.prototype.adopt
 */
function DisposableStackPrototypeAdopt(this: Value, value: Value, onDispose: Value): Value {
  const stack: Obj = PendingDisposableStack(this, false);
  if (!IsCallable(onDispose)) {
    throw NewTypeError("not a function");
  }
  AddResource(StackResources(stack), value, onDispose, ResourceKinds.adopted);
  return value;
}

/**
 * DisposableStack.prototype.defer ( onDispose ), 27.3.3.2.
 * @builtin DisposableStack.prototype.defer
 */
function DisposableStackPrototypeDefer(this: Value, onDispose: Value): void {
  const stack: Obj = PendingDisposableStack(this, false);
  if (!IsCallable(onDispose)) {
    throw NewTypeError("not a function");
  }
  AddResource(StackResources(stack), undefined, onDispose, ResourceKinds.method);
}

/** move ( ) of both stacks: a new stack takes the resources; this one is disposed. */
function DisposableStackMove(thisValue: Value, isAsync: boolean): Obj {
  const stack: Obj = PendingDisposableStack(thisValue, isAsync);
  const proto: Obj = RealmIntrinsic(isAsync ? "AsyncDisposableStackPrototype" : "DisposableStackPrototype");
  const newStack: Obj = DisposableStackCreate(proto, isAsync);
  RecordSet(newStack, DisposableStackSlots.resources, RecordGet(stack, DisposableStackSlots.resources));
  RecordSet(stack, DisposableStackSlots.resources, ArrayCreate(0));
  RecordSet(stack, DisposableStackSlots.state, DisposableStates.disposed);
  return newStack;
}

/**
 * DisposableStack.prototype.move ( ), 27.3.3.5.
 * @builtin DisposableStack.prototype.move
 */
function DisposableStackPrototypeMove(this: Value): Obj {
  return DisposableStackMove(this, false);
}

/**
 * DisposableStack.prototype [ %Symbol.toStringTag% ], 27.3.3.8.
 * @value DisposableStack.prototype[Symbol.toStringTag]
 * @attributes configurable
 */
function DisposableStackPrototypeToStringTag(): Str {
  return "DisposableStack";
}

/**
 * AsyncDisposableStack ( ), 27.4.1.1.
 * @constructor AsyncDisposableStack
 */
function AsyncDisposableStackConstructor(): Obj {
  if (new.target === undefined) {
    throw NewTypeError("must be called with new");
  }
  return DisposableStackCreate(GetPrototypeFromConstructor(new.target, "AsyncDisposableStackPrototype"), true);
}

/**
 * get AsyncDisposableStack.prototype.disposed, 27.4.3.4.
 * @getter AsyncDisposableStack.prototype.disposed
 */
function AsyncDisposableStackPrototypeDisposed(this: Value): boolean {
  return F64ToI32(AsNumber(RecordGet(ThisDisposableStack(this, true), DisposableStackSlots.state))) === DisposableStates.disposed;
}

/**
 * AsyncDisposableStack.prototype.use ( value ), 27.4.3.6.
 * @builtin AsyncDisposableStack.prototype.use
 */
function AsyncDisposableStackPrototypeUse(this: Value, value: Value): Value {
  AddDisposableValue(StackResources(PendingDisposableStack(this, true)), value, true);
  return value;
}

/**
 * AsyncDisposableStack.prototype.adopt ( value, onDisposeAsync ), 27.4.3.1.
 * @builtin AsyncDisposableStack.prototype.adopt
 */
function AsyncDisposableStackPrototypeAdopt(this: Value, value: Value, onDisposeAsync: Value): Value {
  const stack: Obj = PendingDisposableStack(this, true);
  if (!IsCallable(onDisposeAsync)) {
    throw NewTypeError("not a function");
  }
  AddResource(StackResources(stack), value, onDisposeAsync, ResourceKinds.adopted);
  return value;
}

/**
 * AsyncDisposableStack.prototype.defer ( onDisposeAsync ), 27.4.3.2.
 * @builtin AsyncDisposableStack.prototype.defer
 */
function AsyncDisposableStackPrototypeDefer(this: Value, onDisposeAsync: Value): void {
  const stack: Obj = PendingDisposableStack(this, true);
  if (!IsCallable(onDisposeAsync)) {
    throw NewTypeError("not a function");
  }
  AddResource(StackResources(stack), undefined, onDisposeAsync, ResourceKinds.method);
}

/**
 * AsyncDisposableStack.prototype.move ( ), 27.4.3.5.
 * @builtin AsyncDisposableStack.prototype.move
 */
function AsyncDisposableStackPrototypeMove(this: Value): Obj {
  return DisposableStackMove(this, true);
}

/**
 * AsyncDisposableStack.prototype [ %Symbol.toStringTag% ], 27.4.3.8.
 * @value AsyncDisposableStack.prototype[Symbol.toStringTag]
 * @attributes configurable
 */
function AsyncDisposableStackPrototypeToStringTag(): Str {
  return "AsyncDisposableStack";
}

// ---- AsyncDisposableStack.prototype.disposeAsync: DisposeResources with awaits ----------
//
// Like Array.fromAsync (D1511), a record state machine resumed by builtin closures 18
// and 19.

/**
 * disposeAsync's state: the errors, the result promise, the resources, the index of the
 * next triple to dispose (counting down), needsAwait, hasAwaited, a syncInAsync call's
 * exception, to throw once the await is over.
 * @record
 * @extends DisposeErrors
 */
enum AsyncDisposeState {
  error,
  hasError,
  promise,
  resources,
  index,
  needsAwait,
  hasAwaited,
  pending,
}

/**
 * AsyncDisposableStack.prototype.disposeAsync ( ), 27.4.3.3; also
 * [ %Symbol.asyncDispose% ].
 * @builtin AsyncDisposableStack.prototype.disposeAsync
 * @builtin AsyncDisposableStack.prototype[Symbol.asyncDispose]
 */
function AsyncDisposableStackPrototypeDisposeAsync(this: Value): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  let stack: Obj = promise;
  try {
    stack = ThisDisposableStack(this, true);
  } catch (e) {
    RejectPromise(promise, e);
    return promise;
  }
  if (F64ToI32(AsNumber(RecordGet(stack, DisposableStackSlots.state))) === DisposableStates.disposed) {
    ResolvePromise(promise, undefined);
    return promise;
  }
  RecordSet(stack, DisposableStackSlots.state, DisposableStates.disposed);
  const s: Obj = NewRecord(AsyncDisposeState);
  RecordSet(s, AsyncDisposeState.promise, promise);
  const resources: Obj = AsObj(RecordGet(stack, DisposableStackSlots.resources));
  RecordSet(stack, DisposableStackSlots.resources, ArrayCreate(0));
  RecordSet(s, AsyncDisposeState.resources, resources);
  RecordSet(s, AsyncDisposeState.index, LengthOfArrayLike(resources) - 3);
  RecordSet(s, AsyncDisposeState.hasError, false);
  RecordSet(s, AsyncDisposeState.needsAwait, false);
  RecordSet(s, AsyncDisposeState.hasAwaited, false);
  RecordSet(s, AsyncDisposeState.pending, undefined);
  AsyncDisposeContinue(s);
  return promise;
}

/**
 * Records a disposal error in `s` (DisposeErrors; step 2.c.iii: a SuppressedError over an
 * earlier one).
 */
function AsyncDisposeError(s: Obj, e: Value): void {
  if (RecordGet(s, DisposeErrors.hasError) === true) {
    RecordSet(s, DisposeErrors.error, NewSuppressedError(e, RecordGet(s, DisposeErrors.error)));
  } else {
    RecordSet(s, DisposeErrors.error, e);
    RecordSet(s, DisposeErrors.hasError, true);
  }
}

/** Await ( v ) of disposeAsync: resumes with AsyncDisposeResume. */
function AsyncDisposeAwait(s: Obj, v: Value): void {
  const promise: Value = PromiseResolve(IntrinsicPromise(), v);
  PerformPromiseThen(AsObj(promise), NewBuiltinClosure(18, s, 1), NewBuiltinClosure(19, s, 1), undefined);
}

/**
 * The loop of DisposeResources (27.3.3.4 step 2) from the current index: disposes
 * resources until one needs an await, then the final await (step 3) and the result.
 */
function AsyncDisposeContinue(s: Obj): void {
  const resources: Obj = AsObj(RecordGet(s, AsyncDisposeState.resources));
  while (true) {
    const i: f64 = AsNumber(RecordGet(s, AsyncDisposeState.index));
    if (i < 0) {
      break;
    }
    RecordSet(s, AsyncDisposeState.index, i - 3);
    const kind: i32 = F64ToI32(AsNumber(Get(resources, i + 2)));
    if (kind === ResourceKinds.none) {
      RecordSet(s, AsyncDisposeState.needsAwait, true);
      continue;
    }
    let result: Value = undefined;
    try {
      result = CallDisposeMethod(resources, i);
    } catch (e) {
      if (kind !== ResourceKinds.syncInAsync) {
        AsyncDisposeError(s, e);
        continue;
      }
      // GetDisposeMethod's closure returns a rejected promise: awaited, then thrown.
      RecordSet(s, AsyncDisposeState.pending, e);
      result = undefined;
    }
    RecordSet(s, AsyncDisposeState.hasAwaited, true);
    if (kind === ResourceKinds.syncInAsync) {
      result = undefined;
    }
    try {
      AsyncDisposeAwait(s, result);
    } catch (e) {
      AsyncDisposeError(s, e);
      continue;
    }
    return;
  }
  if (RecordGet(s, AsyncDisposeState.needsAwait) === true && RecordGet(s, AsyncDisposeState.hasAwaited) !== true) {
    RecordSet(s, AsyncDisposeState.hasAwaited, true);
    AsyncDisposeAwait(s, undefined);
    return;
  }
  const promise: Obj = AsObj(RecordGet(s, AsyncDisposeState.promise));
  if (RecordGet(s, AsyncDisposeState.hasError) === true) {
    RejectPromise(promise, RecordGet(s, AsyncDisposeState.error));
  } else {
    ResolvePromise(promise, undefined);
  }
}

/**
 * Resumes disposeAsync after a fulfilled await: `this` is its AsyncDisposeState.
 * @intrinsic
 */
function AsyncDisposeFulfilled(this: Value, value: Value): void {
  const s: Obj = AsObj(this);
  const pending: Value = RecordGet(s, AsyncDisposeState.pending);
  if (pending !== undefined) {
    RecordSet(s, AsyncDisposeState.pending, undefined);
    AsyncDisposeError(s, pending);
  }
  AsyncDisposeContinue(s);
}

/**
 * Resumes disposeAsync after a rejected await: `this` is its AsyncDisposeState.
 * @intrinsic
 */
function AsyncDisposeRejected(this: Value, reason: Value): void {
  const s: Obj = AsObj(this);
  AsyncDisposeError(s, reason);
  AsyncDisposeContinue(s);
}

// ---- `using` and `await using` declarations (0058, docs/compiler.md 7.10) ---------------

/**
 * A scope with `using` declarations keeps one on the operand stack: the errors, the
 * resources (triples as in a DisposableStack, kinds method, syncInAsync, none and
 * awaited), needsAwait, hasAwaited, the index of the next triple to dispose (undefined
 * before the first step), the value to await, a syncInAsync call's exception to merge
 * once the await is over, whether there is one.
 * @record
 * @extends DisposeErrors
 */
enum UsingRecord {
  error,
  hasError,
  resources,
  needsAwait,
  hasAwaited,
  next,
  value,
  pendingError,
  hasPending,
}

/**
 * The resource record of a scope's `using` declarations.
 * @intrinsic
 */
function UsingCreate(): Obj {
  const s: Obj = NewRecord(UsingRecord);
  RecordSet(s, UsingRecord.resources, ArrayCreate(0));
  return s;
}

/**
 * `using x = v`: AddDisposableResource ( s, v, sync-dispose ), 27.3.3.1. Returns v.
 * @intrinsic
 */
function UsingAdd(s: Obj, v: Value): Value {
  AddDisposableValue(AsObj(RecordGet(s, UsingRecord.resources)), v, false);
  return v;
}

/**
 * `await using x = v`: AddDisposableResource ( s, v, async-dispose ). Returns v.
 * @intrinsic
 */
function AwaitUsingAdd(s: Obj, v: Value): Value {
  const resources: Obj = AsObj(RecordGet(s, UsingRecord.resources));
  AddDisposableValue(resources, v, true);
  // An async add always appends one triple (none for null and undefined); a method
  // there is a Symbol.asyncDispose method, awaited unlike a `using` resource's.
  const k: f64 = LengthOfArrayLike(resources) - 1;
  if (F64ToI32(AsNumber(Get(resources, k))) === ResourceKinds.method) {
    CreateDataPropertyOrThrow(resources, k, ResourceKinds.awaited);
  }
  return v;
}

/**
 * Records an exception: the one the scope exits with (first), or one of a disposal
 * (as a SuppressedError over the earlier ones).
 * @intrinsic
 */
function UsingDisposeError(s: Obj, e: Value): void {
  AsyncDisposeError(s, e);
}

/**
 * DisposeResources ( s, completion ), 27.3.3.4, of a scope without `await using`: the
 * completion is the error recorded by UsingDisposeError, if any; throws the result.
 * @intrinsic
 */
function UsingDispose(s: Obj): void {
  const resources: Obj = AsObj(RecordGet(s, UsingRecord.resources));
  for (let i: f64 = LengthOfArrayLike(resources) - 3; i >= 0; i = i - 3) {
    try {
      CallDisposeMethod(resources, i);
    } catch (e) {
      AsyncDisposeError(s, e);
    }
  }
  if (RecordGet(s, UsingRecord.hasError) === true) {
    throw RecordGet(s, UsingRecord.error);
  }
}

/**
 * DisposeResources of a scope with `await using`, up to its next Await: true when the
 * code must await UsingAwaitValue(s) (an exception from it goes to UsingDisposeError)
 * and step again, false when done; throws the result.
 * @intrinsic
 */
function UsingDisposeStep(s: Obj): boolean {
  if (RecordGet(s, UsingRecord.hasPending) === true) {
    RecordSet(s, UsingRecord.hasPending, false);
    AsyncDisposeError(s, RecordGet(s, UsingRecord.pendingError));
  }
  const resources: Obj = AsObj(RecordGet(s, UsingRecord.resources));
  let i: f64 = 0;
  if (RecordGet(s, UsingRecord.next) === undefined) {
    i = LengthOfArrayLike(resources) - 3;
  } else {
    i = AsNumber(RecordGet(s, UsingRecord.next));
  }
  while (i >= 0) {
    const kind: i32 = F64ToI32(AsNumber(Get(resources, i + 2)));
    if (kind === ResourceKinds.method && RecordGet(s, UsingRecord.needsAwait) === true && RecordGet(s, UsingRecord.hasAwaited) !== true) {
      // A sync disposal after a null `await using`: the pending Await comes first.
      RecordSet(s, UsingRecord.needsAwait, false);
      RecordSet(s, UsingRecord.next, i);
      RecordSet(s, UsingRecord.value, undefined);
      return true;
    }
    RecordSet(s, UsingRecord.next, i - 3);
    if (kind === ResourceKinds.none) {
      RecordSet(s, UsingRecord.needsAwait, true);
    } else if (kind === ResourceKinds.method) {
      try {
        CallDisposeMethod(resources, i);
      } catch (e) {
        AsyncDisposeError(s, e);
      }
    } else {
      let result: Value = undefined;
      let threw: boolean = false;
      try {
        result = CallDisposeMethod(resources, i);
      } catch (e) {
        threw = true;
        if (kind === ResourceKinds.syncInAsync) {
          // GetDisposeMethod's closure returns a rejected promise: awaited, then thrown.
          RecordSet(s, UsingRecord.pendingError, e);
          RecordSet(s, UsingRecord.hasPending, true);
        } else {
          AsyncDisposeError(s, e);
        }
      }
      if (kind === ResourceKinds.syncInAsync || !threw) {
        RecordSet(s, UsingRecord.hasAwaited, true);
        RecordSet(s, UsingRecord.value, kind === ResourceKinds.syncInAsync ? undefined : result);
        return true;
      }
    }
    i = i - 3;
  }
  if (RecordGet(s, UsingRecord.needsAwait) === true && RecordGet(s, UsingRecord.hasAwaited) !== true) {
    RecordSet(s, UsingRecord.needsAwait, false);
    RecordSet(s, UsingRecord.value, undefined);
    return true;
  }
  if (RecordGet(s, UsingRecord.hasError) === true) {
    throw RecordGet(s, UsingRecord.error);
  }
  return false;
}

/**
 * The value UsingDisposeStep asks to await.
 * @intrinsic
 */
function UsingAwaitValue(s: Obj): Value {
  return RecordGet(s, UsingRecord.value);
}

/**
 * %IteratorPrototype% [ %Symbol.dispose% ] ( ), 27.1.4.15.
 * @builtin Iterator.prototype[Symbol.dispose]
 */
function IteratorPrototypeDispose(this: Value): void {
  const method: Value = GetMethod(this, "return");
  if (method !== undefined) {
    Call(method, this);
  }
}

/**
 * %AsyncIteratorPrototype% [ %Symbol.asyncDispose% ] ( ), 27.1.5.1.
 * @builtin %AsyncIteratorPrototype%[Symbol.asyncDispose]
 */
function AsyncIteratorPrototypeAsyncDispose(this: Value): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  let wrapper: Value = undefined;
  try {
    const method: Value = GetMethod(this, "return");
    if (method === undefined) {
      ResolvePromise(promise, undefined);
      return promise;
    }
    wrapper = PromiseResolve(IntrinsicPromise(), Call(method, this));
  } catch (e) {
    RejectPromise(promise, e);
    return promise;
  }
  // The unwrap closure returns undefined: kind 9 returns its data.
  PerformPromiseThen(AsObj(wrapper), NewBuiltinClosure(9, undefined, 1), undefined, promise);
  return promise;
}

/**
 * Symbol.dispose, 20.4.2.
 * @value Symbol.dispose
 */
function SymbolDispose(): Sym {
  return WellKnownSymbol("dispose");
}

/**
 * Symbol.asyncDispose, 20.4.2.
 * @value Symbol.asyncDispose
 */
function SymbolAsyncDispose(): Sym {
  return WellKnownSymbol("asyncDispose");
}
