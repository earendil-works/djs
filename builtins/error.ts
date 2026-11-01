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
 * InternalError (QuickJS's, for stack overflow and out of memory).
 * @constructor InternalError
 * @extends Error
 * @length 1
 */
function InternalErrorConstructor(message?: Value, options?: Value): Obj {
  return ErrorConstruct(new.target, "InternalErrorPrototype", message, options);
}
