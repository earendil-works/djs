// TextEncoder and TextDecoder (WHATWG Encoding standard, UTF-8 only; 0244, D3172). The
// arguments are converted as WebIDL does, in order, before the steps; the bytes are
// read and written by the leaves in builtins/encoding.rs. Messages are node's. Methods
// and accessors are enumerable, as WebIDL's are.
// @slim-omit: left out of the slim build (D2411).

/** The prototype for a new codec: new.target's, else the realm constructor `name`'s. */
function TextCodecPrototype(newTarget: Value, name: Str): Obj {
  const proto: Value = Get(AsObj(newTarget), "prototype");
  if (IsObject(proto)) {
    return AsObj(proto);
  }
  return AsObj(Get(RealmConstructor(name), "prototype"));
}

/**
 * Whether the WebIDL dictionary argument v has members to read: false for undefined
 * and null, true for an object, else TypeError.
 */
function TextDictionary(v: Value): boolean {
  if (v === undefined || v === null) {
    return false;
  }
  if (!IsObject(v)) {
    throw NewTypeError("The \"options\" argument must be of type object");
  }
  return true;
}

/**
 * TextEncoder ( ).
 * @constructor TextEncoder
 */
function TextEncoderConstructor(): Obj {
  if (new.target === undefined) {
    throw NewTypeError("must be called with new");
  }
  return AsObj(TextCodecCreate(TextCodecPrototype(new.target, "TextEncoder"), 0, "utf-8"));
}

/**
 * get TextEncoder.prototype.encoding.
 * @getter TextEncoder.prototype.encoding
 */
function TextEncoderPrototypeEncoding(this: Value): Str {
  TextCodecFlags(this, false);
  return "utf-8";
}

/**
 * TextEncoder.prototype.encode ( optional USVString input = "" ).
 * @builtin TextEncoder.prototype.encode
 */
function TextEncoderPrototypeEncode(this: Value, input?: Value): Obj {
  TextCodecFlags(this, false);
  return Utf8Encode(input === undefined ? "" : ToString(input));
}

/**
 * TextEncoder.prototype.encodeInto ( USVString source, Uint8Array destination ).
 * @builtin TextEncoder.prototype.encodeInto
 * @length 2
 */
function TextEncoderPrototypeEncodeInto(this: Value, ...args: Value[]): Obj {
  TextCodecFlags(this, false);
  if (LengthOfArrayLike(args) < 2) {
    throw NewTypeError("2 arguments required");
  }
  const r: Value = Utf8EncodeInto(ToString(Get(args, 0)), Get(args, 1));
  if (r === undefined) {
    throw NewTypeError("The \"dest\" argument must be an instance of Uint8Array");
  }
  return AsObj(r);
}

/**
 * TextEncoder.prototype [ %Symbol.toStringTag% ]; installed last, it makes the members
 * enumerable.
 * @value TextEncoder.prototype[Symbol.toStringTag]
 * @attributes configurable
 */
function TextEncoderToStringTag(): Str {
  const proto: Obj = AsObj(Get(RealmConstructor("TextEncoder"), "prototype"));
  DOMExceptionEnumerable(proto, "encoding");
  DOMExceptionEnumerable(proto, "encode");
  DOMExceptionEnumerable(proto, "encodeInto");
  return "TextEncoder";
}

/**
 * TextDecoder ( optional DOMString label = "utf-8", optional TextDecoderOptions options ).
 * @constructor TextDecoder
 * @length 0
 */
function TextDecoderConstructor(label?: Value, options?: Value): Obj {
  if (new.target === undefined) {
    throw NewTypeError("must be called with new");
  }
  const l: Str = label === undefined ? "utf-8" : ToString(label);
  let flags: i32 = 1;
  if (TextDictionary(options)) {
    if (ToBoolean(Get(AsObj(options), "fatal"))) {
      flags = flags | 2;
    }
    if (ToBoolean(Get(AsObj(options), "ignoreBOM"))) {
      flags = flags | 4;
    }
  }
  const d: Value = TextCodecCreate(TextCodecPrototype(new.target, "TextDecoder"), flags, l);
  if (d === undefined) {
    throw NewRangeError("The \"" + l + "\" encoding is not supported");
  }
  return AsObj(d);
}

/**
 * get TextDecoder.prototype.encoding.
 * @getter TextDecoder.prototype.encoding
 */
function TextDecoderPrototypeEncoding(this: Value): Str {
  TextCodecFlags(this, true);
  return "utf-8";
}

/**
 * get TextDecoder.prototype.fatal.
 * @getter TextDecoder.prototype.fatal
 */
function TextDecoderPrototypeFatal(this: Value): boolean {
  return (TextCodecFlags(this, true) & 2) !== 0;
}

/**
 * get TextDecoder.prototype.ignoreBOM.
 * @getter TextDecoder.prototype.ignoreBOM
 */
function TextDecoderPrototypeIgnoreBOM(this: Value): boolean {
  return (TextCodecFlags(this, true) & 4) !== 0;
}

/**
 * TextDecoder.prototype.decode ( optional AllowSharedBufferSource input, optional
 * TextDecodeOptions options ).
 * @builtin TextDecoder.prototype.decode
 * @length 0
 */
function TextDecoderPrototypeDecode(this: Value, input?: Value, options?: Value): Str {
  TextCodecFlags(this, true);
  if (input !== undefined && !IsBufferSource(input)) {
    throw NewTypeError("The \"input\" argument must be an instance of ArrayBuffer or ArrayBufferView");
  }
  let stream: boolean = false;
  if (TextDictionary(options)) {
    stream = ToBoolean(Get(AsObj(options), "stream"));
  }
  const r: Value = Utf8Decode(AsObj(this), input, stream);
  if (r === undefined) {
    throw NewTypeError("The encoded data was not valid for encoding utf-8");
  }
  return AsStr(r);
}

/**
 * TextDecoder.prototype [ %Symbol.toStringTag% ]; installed last, it makes the members
 * enumerable.
 * @value TextDecoder.prototype[Symbol.toStringTag]
 * @attributes configurable
 */
function TextDecoderToStringTag(): Str {
  const proto: Obj = AsObj(Get(RealmConstructor("TextDecoder"), "prototype"));
  DOMExceptionEnumerable(proto, "encoding");
  DOMExceptionEnumerable(proto, "fatal");
  DOMExceptionEnumerable(proto, "ignoreBOM");
  DOMExceptionEnumerable(proto, "decode");
  return "TextDecoder";
}
