// Error objects (ECMA-262 20.5).

/** InstallErrorCause ( O, options ), 20.5.8.1. */
function InstallErrorCause(o: Obj, options: Value): void {
  if (IsObject(options)) {
    const opts: Obj = AsObj(options);
    if (HasProperty(opts, "cause")) {
      DefineDataPropertyOrThrow(o, "cause", Get(opts, "cause"), 5);
    }
  }
}

/**
 * get Error.prototype.stack (the error stack accessor proposal, D1971): the trace an
 * Error recorded at its creation, undefined for an object without [[ErrorData]].
 * @getter Error.prototype.stack
 */
function ErrorPrototypeGetStack(this: Value): Value {
  if (!IsObject(this)) {
    throw NewTypeError("not an object");
  }
  return ErrorStack(this);
}

/**
 * set Error.prototype.stack: SetterThatIgnoresPrototypeProperties ( this,
 * %Error.prototype%, "stack", v ) for a string v.
 * @setter Error.prototype.stack
 */
function ErrorPrototypeSetStack(this: Value, v: Value): Value {
  if (!IsObject(this)) {
    throw NewTypeError("not an object");
  }
  if (!IsString(v)) {
    throw NewTypeError("not a string");
  }
  const o: Obj = AsObj(this);
  if (o === RealmIntrinsic("ErrorPrototype")) {
    throw NewTypeError("cannot set stack of Error.prototype");
  }
  if (!HasOwnProperty(o, "stack")) {
    CreateDataPropertyOrThrow(o, "stack", v);
  } else {
    Set(o, "stack", v);
  }
  return undefined;
}

/**
 * Error.isError ( arg ) (proposal-is-error): whether arg has an [[ErrorData]] slot
 * (a proxy has none).
 * @builtin Error.isError
 */
function ErrorIsError(arg: Value): boolean {
  if (!IsObject(arg) || IsProxy(arg)) {
    return false;
  }
  return ObjectClassTag(AsObj(arg)) === "Error";
}

/**
 * Error.captureStackTrace ( object [ , constructorOpt ] ) (V8, qjs; D2010): an own
 * non-enumerable `stack` with the current trace, from below the topmost call of
 * constructorOpt if given.
 * @builtin Error.captureStackTrace
 */
function ErrorCaptureStackTrace(object: Value, constructorOpt: Value): void {
  if (!IsObject(object)) {
    throw NewTypeError("not an object");
  }
  const o: Obj = AsObj(object);
  const below: Value = IsCallable(constructorOpt) ? constructorOpt : undefined;
  const stack: Str = StackTraceString(below);
  if (!HasOwnProperty(o, "stack") && !IsExtensible(o)) {
    throw NewTypeError("object is not extensible");
  }
  DefineDataPropertyOrThrow(o, "stack", stack, 5);
}

/**
 * Error.stackTraceLimit (V8, qjs; D2030): the number of frames a stack trace records.
 * A plain data property that the VM reads without running code; a value that is not a
 * number records none.
 * @value Error.stackTraceLimit
 * @attributes writable configurable
 */
function ErrorStackTraceLimit(): i32 {
  return 10;
}

/**
 * The steps shared by Error ( message [ , options ] ), 20.5.1.1, and the
 * NativeError constructors, 20.5.6.1.1. `fallback` names the realm prototype.
 */
function ErrorConstruct(newTarget: Value, fallback: Str, message: Value, options: Value): Obj {
  const o: Obj = ErrorCreate(GetPrototypeFromConstructor(newTarget, fallback));
  if (message !== undefined) {
    DefineDataPropertyOrThrow(o, "message", ToString(message), 5);
  }
  InstallErrorCause(o, options);
  CaptureStack(o);
  return o;
}

/**
 * Error ( message [ , options ] ), 20.5.1.1.
 * @constructor Error
 * @length 1
 */
function ErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "ErrorPrototype", message, options);
}

/**
 * EvalError, 20.5.5.1.
 * @constructor EvalError
 * @extends Error
 * @length 1
 */
function EvalErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "EvalErrorPrototype", message, options);
}

/**
 * RangeError, 20.5.5.2.
 * @constructor RangeError
 * @extends Error
 * @length 1
 */
function RangeErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "RangeErrorPrototype", message, options);
}

/**
 * ReferenceError, 20.5.5.3.
 * @constructor ReferenceError
 * @extends Error
 * @length 1
 */
function ReferenceErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "ReferenceErrorPrototype", message, options);
}

/**
 * SyntaxError, 20.5.5.4.
 * @constructor SyntaxError
 * @extends Error
 * @length 1
 */
function SyntaxErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "SyntaxErrorPrototype", message, options);
}

/**
 * TypeError, 20.5.5.5.
 * @constructor TypeError
 * @extends Error
 * @length 1
 */
function TypeErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "TypeErrorPrototype", message, options);
}

/**
 * URIError, 20.5.5.6.
 * @constructor URIError
 * @extends Error
 * @length 1
 */
function URIErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "URIErrorPrototype", message, options);
}

/**
 * Error.prototype.toString ( ), 20.5.3.4.
 * @builtin Error.prototype.toString
 */
function ErrorPrototypeToString(this: Value): Str {
  if (!IsObject(this)) {
    throw NewTypeError("not an object");
  }
  const o: Obj = AsObj(this);
  const name: Value = Get(o, "name");
  const nameStr: Str = name === undefined ? "Error" : ToString(name);
  const msg: Value = Get(o, "message");
  const msgStr: Str = msg === undefined ? "" : ToString(msg);
  if (nameStr === "") {
    return msgStr;
  }
  if (msgStr === "") {
    return nameStr;
  }
  return nameStr + ": " + msgStr;
}

/**
 * InternalError (QuickJS's, for out of memory; stack overflow is a RangeError, D013).
 * @constructor InternalError
 * @extends Error
 * @length 1
 */
function InternalErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "InternalErrorPrototype", message, options);
}

/**
 * AggregateError ( errors, message [ , options ] ), 20.5.7.1.1.
 * @constructor AggregateError
 * @extends Error
 * @length 2
 */
function AggregateErrorConstructor(errors: Value, message: Value, options?: Value): Obj {
  const o: Obj = ErrorConstruct(new.target, "AggregateErrorPrototype", message, options);
  DefineDataPropertyOrThrow(o, "errors", IterableToList(errors), 5);
  return o;
}

/**
 * SuppressedError ( error, suppressed, message ), 20.5.8.1.1.
 * @constructor SuppressedError
 * @extends Error
 */
function SuppressedErrorConstructor(error: Value, suppressed: Value, message: Value): Obj {
  const o: Obj = ErrorCreate(GetPrototypeFromConstructor(new.target, "SuppressedErrorPrototype"));
  if (message !== undefined) {
    DefineDataPropertyOrThrow(o, "message", ToString(message), 5);
  }
  DefineDataPropertyOrThrow(o, "error", error, 5);
  DefineDataPropertyOrThrow(o, "suppressed", suppressed, 5);
  CaptureStack(o);
  return o;
}
