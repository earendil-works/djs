// Runs the generated builtins (builtins/gen/prelude.js, with %Name(...) rewritten to
// I.Name(...)) against qjs's native implementations. Prints one line per mismatch and
// "ok N" at the end. Used by tests/behavior.rs; PRELUDE is prepended by the test.
"use strict";

const installed = {};
const ourProxies = new WeakMap();
const TA_CTORS = [Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array, Int32Array, Uint32Array, Float16Array, Float32Array, Float64Array, BigInt64Array, BigUint64Array];
const TA_PROTO = Object.getPrototypeOf(Int8Array.prototype);
const DV_NAMES = ["Int8", "Uint8", "Uint8", "Int16", "Uint16", "Int32", "Uint32", "Float16", "Float32", "Float64", "BigInt64", "BigUint64"];
const dataViews = new WeakMap();
function taKind(v) {
  const tag = Object.getOwnPropertyDescriptor(TA_PROTO, Symbol.toStringTag).get.call(v);
  return tag === undefined ? -1 : TA_CTORS.findIndex((C) => C.name === tag);
}
// The native setFromBase64/setFromHex throw at an error after writing what they could;
// the intrinsics return { read, bytes, error } instead. (The stand-in cannot recover the
// bytes written before an error, so the comparisons avoid partial-error cases.)
function decodeRecord(run) {
  const rec = Object.create(null);
  try {
    const [read, bytes] = run();
    rec.read = read;
    rec.bytes = bytes.buffer;
    rec.error = false;
  } catch (e) {
    if (!(e instanceof SyntaxError)) throw e;
    rec.read = 0;
    rec.bytes = new ArrayBuffer(0);
    rec.error = true;
  }
  return rec;
}
const iterState = new WeakMap();
const MapIterNext = Object.getPrototypeOf(new Map().keys()).next;
const SetIterNext = Object.getPrototypeOf(new Set().keys()).next;
function isMapLike(m) {
  try {
    Map.prototype.has.call(m, 1);
    return true;
  } catch {
    return false;
  }
}
function collProto(m) {
  for (const C of [Map, Set, WeakMap, WeakSet]) {
    try {
      C.prototype.has.call(m, {});
      return C.prototype;
    } catch {}
  }
  throw new Error("intrinsic misuse: not a collection");
}
const arrayIterators = new WeakMap();
let ourAIP;
function ourArrayIteratorPrototype() {
  if (!ourAIP) {
    const iterProto = {};
    Object.defineProperty(iterProto, Symbol.iterator, {
      value: installed["%IteratorPrototype%[Symbol.iterator]"].f,
      writable: true,
      configurable: true,
    });
    ourAIP = Object.create(iterProto);
    Object.defineProperty(ourAIP, "next", {
      value: installed["%ArrayIteratorPrototype%.next"].f,
      writable: true,
      configurable: true,
    });
  }
  return ourAIP;
}
const isObject = (v) => v !== null && (typeof v === "object" || typeof v === "function");
// Iterator helpers (0051): the slots of our helper objects (and Records), and their prototypes built
// from the installed methods.
const helperSlots = new WeakMap();
const helperProtos = [];
function helperProto(wrap) {
  if (!helperProtos.length) {
    for (const name of ["%WrapForValidIteratorPrototype%", "%IteratorHelperPrototype%"]) {
      const p = Object.create(Iterator.prototype);
      for (const m of ["next", "return"]) {
        Object.defineProperty(p, m, { value: installed[name + "." + m].f, writable: true, configurable: true });
      }
      helperProtos.push(p);
    }
    Object.defineProperty(helperProtos[1], Symbol.toStringTag, { value: "Iterator Helper", configurable: true });
  }
  return helperProtos[wrap ? 0 : 1];
}
function check(cond, what) {
  if (!cond) throw new Error("intrinsic misuse: " + what);
}

// Plain-JS stand-ins for the intrinsics of builtins/intrinsics.d.ts.
const I = {
  IsObject: isObject,
  IsCallable: (v) => typeof v === "function",
  IsConstructor: (v) => {
    if (typeof v !== "function") return false;
    try {
      Reflect.construct(String, [], v);
      return true;
    } catch {
      return false;
    }
  },
  IsArray: (v) => {
    const st = ourProxies.get(v);
    if (st !== undefined && st.target === null) throw new TypeError("revoked proxy");
    return Array.isArray(st !== undefined ? st.target : v);
  },
  IsString: (v) => typeof v === "string",
  IsNumber: (v) => typeof v === "number",
  IsSymbol: (v) => typeof v === "symbol",
  IsBoolean: (v) => typeof v === "boolean",
  AsObj: (v) => (check(isObject(v), "AsObj"), v),
  AsFn: (v) => (check(typeof v === "function", "AsFn"), v),
  AsStr: (v) => (check(typeof v === "string", "AsStr"), v),
  AsNumber: (v) => (check(typeof v === "number", "AsNumber"), v),
  AsSym: (v) => (check(typeof v === "symbol", "AsSym"), v),
  AsBool: (v) => (check(typeof v === "boolean", "AsBool"), v),
  ToBoolean: (v) => !!v,
  ToNumberPrimitive: (v) => (check(!isObject(v), "ToNumberPrimitive"), Number(v)),
  ToStringPrimitive: (v) => {
    check(!isObject(v), "ToStringPrimitive");
    if (typeof v === "symbol") throw new TypeError("cannot convert symbol to string");
    return String(v);
  },
  ToObject: (v) => {
    if (v === undefined || v === null) throw new TypeError("cannot convert to object");
    return Object(v);
  },
  ToIntegerOrInfinityNumber: (n) => {
    if (Number.isNaN(n) || n === 0) return 0;
    if (!Number.isFinite(n)) return n;
    return Math.trunc(n) + 0;
  },
  F64ToU32: (n) => (check(n >>> 0 === n, "F64ToU32"), n),
  F64ToI32: (n) => (check((n | 0) === n, "F64ToI32"), n),
  StrLength: (s) => s.length,
  Get: (o, k) => (check(isObject(o) && !isObject(k), "Get"), Reflect.get(o, k)),
  GetV: (v, k) => (check(!isObject(k), "GetV"), v[k]),
  Set: (o, k, v) => {
    o[k] = v;
  },
  HasProperty: (o, k) => k in o,
  CreateDataPropertyOrThrow: (o, k, v) => {
    Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true });
  },
  DeletePropertyOrThrow: (o, k) => {
    delete o[k];
  },
  Call: (f, t, ...args) => Reflect.apply(f, t, args),
  Construct: (f, ...args) => Reflect.construct(f, args),
  ObjectCreate: (proto) => Object.create(proto),
  ArrayCreate: (len) => {
    if (len > 4294967295) throw new RangeError("invalid array length");
    return new Array(len);
  },
  NewTypeError: (m) => new TypeError(m),
  NewRangeError: (m) => new RangeError(m),
  NewReferenceError: (m) => new ReferenceError(m),
  DisposableStackCreate: (proto, isAsync) => {
    const o = Object.create(proto);
    helperSlots.set(o, [isAsync, 0, []]);
    return o;
  },
  ThisDisposableStack: (v, isAsync) => {
    const slots = isObject(v) ? helperSlots.get(v) : undefined;
    if (!slots || slots.length !== 3 || slots[0] !== isAsync) {
      throw new TypeError(isAsync ? "AsyncDisposableStack object expected" : "DisposableStack object expected");
    }
    return v;
  },
  NewNotConstructorError: (f) => new TypeError(f.name ? f.name + " is not a constructor" : "not a constructor"),
  WellKnownSymbol: (name) => Symbol[name],
  InstallMethod: (target, key, f, length) => {
    installed[target + (typeof key === "symbol" ? "[" + key.description + "]" : "." + key)] = { f, length };
  },
  // Array Iterators: plain objects whose state lives in a WeakMap, with our next and
  // %IteratorPrototype%[Symbol.iterator] on their prototype chain.
  CreateArrayIterator: (o, kind) => {
    const it = Object.create(ourArrayIteratorPrototype());
    arrayIterators.set(it, { target: o, index: 0, kind });
    return it;
  },
  ArrayIteratorTarget: (it) => {
    const st = arrayIterators.get(it);
    if (!st) throw new TypeError("Array Iterator object expected");
    return st.target;
  },
  ArrayIteratorIndex: (it) => arrayIterators.get(it).index,
  ArrayIteratorSetIndex: (it, i) => {
    arrayIterators.get(it).index = i;
  },
  ArrayIteratorFinish: (it) => {
    arrayIterators.get(it).target = undefined;
  },
  ArrayIteratorKind: (it) => arrayIterators.get(it).kind,
  CreateIterResultObject: (value, done) => ({ value, done }),
  CreateIteratorHelper: (kind, iterator, next) => {
    const o = Object.create(helperProto(kind === 0));
    helperSlots.set(o, [kind, 0, iterator, next, undefined, undefined, undefined, undefined]);
    return o;
  },
  ThisIteratorHelper: (v, wrap) => {
    const slots = isObject(v) ? helperSlots.get(v) : undefined;
    if (!slots || (slots[0] === 0) !== wrap) {
      throw new TypeError(wrap ? "Iterator Wrap object expected" : "Iterator Helper object expected");
    }
    return v;
  },
  NewRecord: (n) => {
    const r = Object.create(null);
    helperSlots.set(r, new Array(n).fill(undefined));
    return r;
  },
  RecordGet: (r, i) => {
    check(helperSlots.has(r) && i < helperSlots.get(r).length, "RecordGet");
    return helperSlots.get(r)[i];
  },
  RecordSet: (r, i, v) => {
    check(helperSlots.has(r) && i < helperSlots.get(r).length, "RecordSet");
    helperSlots.get(r)[i] = v;
  },
  CreateArrayFromPair: (a, b) => [a, b],
  InstallIntrinsic: (name, f) => {
    installed["%" + name] = { f };
  },
  // The constructor gets the native one's prototype, so instances compare equal.
  InstallConstructor: (target, key, f, length, parent) => {
    const native = key === "%TypedArray%" ? Object.getPrototypeOf(Int8Array) : globalThis[key];
    Object.defineProperty(f, "prototype", { value: native.prototype });
    installed[target + "." + key] = { f, length };
  },
  InstallAccessor: (target, key, f, isSetter) => {
    installed[(isSetter ? "set " : "get ") + target + "." + String(key)] = { f };
  },
  InstallValue: (target, key, v, attrs) => {
    installed[target + "." + String(key)] = { v, attrs };
  },
  RealmIntrinsic: (name) => globalThis[name.replace(/Prototype$/, "")].prototype,
  ErrorCreate: (proto) => Object.setPrototypeOf(new Error(), proto),
  CaptureStack: (o) => {},
  DefineDataPropertyOrThrow: (o, k, v, attrs) => {
    check(isObject(o) && !isObject(k), "DefineDataPropertyOrThrow");
    Object.defineProperty(o, k, {
      value: v,
      writable: (attrs & 1) !== 0,
      enumerable: (attrs & 2) !== 0,
      configurable: (attrs & 4) !== 0,
    });
  },
  DefineOwnPropertyFromRecord: (o, k, d) => {
    check(isObject(o) && !isObject(k) && Object.getPrototypeOf(d) === null, "DefineOwnPropertyFromRecord");
    return Reflect.defineProperty(o, k, d);
  },
  GetOwnPropertyDescriptorObject: (o, k) => (check(!isObject(k), "GetOwnPropertyDescriptorObject"), Object.getOwnPropertyDescriptor(o, k)),
  HasOwnProperty: (o, k) => (check(!isObject(k), "HasOwnProperty"), Object.prototype.hasOwnProperty.call(o, k)),
  OwnPropertyAttributes: (o, k) => {
    check(!isObject(k), "OwnPropertyAttributes");
    const d = Reflect.getOwnPropertyDescriptor(o, k);
    if (d === undefined) return -1;
    return (d.writable ? 1 : 0) | (d.enumerable ? 2 : 0) | (d.configurable ? 4 : 0) | ("get" in d ? 8 : 0);
  },
  OwnPropertyKeys: (o, kind) =>
    Reflect.ownKeys(o).filter((k) =>
      kind === 0 ? typeof k === "string" : kind === 1 ? typeof k === "symbol" : kind === 2 ? true
        : typeof k === "string" && Object.getOwnPropertyDescriptor(o, k).enumerable),
  CallWithArray: (f, t, args) => (check(Array.isArray(args), "CallWithArray"), Reflect.apply(f, t, args)),
  BoundFunctionCreate: (target, t, args) => Function.prototype.bind.call(target, t, ...args),
  FunctionToString: (f) => Function.prototype.toString.call(f),
  NumberPow: (b, e) => (check(typeof b === "number" && typeof e === "number", "NumberPow"), Math.pow(b, e)),
  SymbolDescriptiveString: (s) => s.toString(),
  GetPrototypeOf: (o) => Object.getPrototypeOf(o),
  SetPrototypeOf: (o, p) => Reflect.setPrototypeOf(o, p),
  PreventExtensions: (o) => Reflect.preventExtensions(o),
  IsExtensible: (o) => Reflect.isExtensible(o),
  SetIntegrityLevel: (o, frozen) => {
    if (!Reflect.preventExtensions(o)) return false;
    for (const k of Reflect.ownKeys(o)) {
      const d = Reflect.getOwnPropertyDescriptor(o, k);
      const nd = frozen && "value" in d ? { configurable: false, writable: false } : { configurable: false };
      if (!Reflect.defineProperty(o, k, nd)) return false;
    }
    return true;
  },
  TestIntegrityLevel: (o, frozen) => (frozen ? Object.isFrozen(o) : Object.isSealed(o)),
  SameValue: (a, b) => Object.is(a, b),
  ObjectClassTag: (o) => {
    if (Array.isArray(o)) return "Array";
    if (typeof o === "function") return "Function";
    if (o instanceof Error) return "Error";
    try {
      Date.prototype.getTime.call(o);
      return "Date";
    } catch {}
    for (const [C, n] of [[Boolean, "Boolean"], [Number, "Number"], [String, "String"]]) {
      try {
        C.prototype.valueOf.call(o);
        return n;
      } catch {}
    }
    return "Object";
  },
  WrapPrimitive: (v, proto) => Object.setPrototypeOf(Object(v), proto),
  PrimitiveValue: (v) => {
    if (!isObject(v)) return v;
    for (const C of [Boolean, Number, String, Symbol, BigInt]) {
      try {
        return C.prototype.valueOf.call(v);
      } catch {}
    }
    return undefined;
  },
  BoundTargetFunction: (f) => undefined,
  IsBigInt: (v) => typeof v === "bigint",
  ToBigIntPrimitive: (v) => {
    check(!isObject(v), "ToBigIntPrimitive");
    if (typeof v === "number" || typeof v === "symbol" || v == null) throw new TypeError("cannot convert to bigint");
    try {
      return BigInt(v);
    } catch (e) {
      throw e instanceof SyntaxError ? new SyntaxError("invalid bigint literal") : e;
    }
  },
  NumberToBigInt: (n) => {
    check(typeof n === "number", "NumberToBigInt");
    if (!Number.isFinite(n)) throw new RangeError("cannot convert NaN or Infinity to BigInt");
    if (!Number.isInteger(n)) throw new RangeError("cannot convert to BigInt: not an integer");
    return BigInt(n);
  },
  BigIntToString: (b, r) => (check(typeof b === "bigint" && (r | 0) === r, "BigIntToString"), b.toString(r)),
  BigIntAsIntN: (n, b) => (check(typeof b === "bigint", "BigIntAsIntN"), BigInt.asIntN(n, b)),
  BigIntAsUintN: (n, b) => (check(typeof b === "bigint", "BigIntAsUintN"), BigInt.asUintN(n, b)),
  BigIntToNumber: (b) => (check(typeof b === "bigint", "BigIntToNumber"), Number(b)),
  NewSymbol: (d) => Symbol(d),
  SymbolDescription: (s) => s.description,
  SymbolFor: (k) => Symbol.for(k),
  SymbolKeyFor: (s) => Symbol.keyFor(s),
  NumberToString: (x, r) => {
    if (r < 2 || r > 36) throw new RangeError("radix must be between 2 and 36");
    return x.toString(r);
  },
  NumberToFixed: (x, d) => x.toFixed(d),
  NumberToExponential: (x, d) => x.toExponential(d),
  NumberToPrecision: (x, p) => x.toPrecision(p),
  StringParseFloat: (s) => parseFloat(s),
  StringParseInt: (s, r) => parseInt(s, r),
  NumberToInt32: (n) => n | 0,
  NumberToUint32: (n) => n >>> 0,
  MathRandom: () => 0.5,
  ArraySpliceDense: (o, start, deleteCount, itemCount) => false,
  // No direct paths in the stand-ins: always the iterator protocol.
  HasOriginalIteration: (v) => false,
  CreateStringIterator: (s) => s[Symbol.iterator](),
  StringIteratorTarget: (it) => {
    throw new Error("intrinsic misuse: StringIteratorTarget is not stood in");
  },
  StrCodePointString: (s, i) => String.fromCodePoint(s.codePointAt(i)),
  // Proxies (0045): ours are native proxy shells whose traps run our internal methods
  // (installed %Proxy... builtins), as the interpreter dispatches to them.
  IsProxy: (v) => ourProxies.has(v),
  NewProxy: (target, handler) => {
    const ours = (name) => (...args) => installed["%" + name].f(shell, ...args);
    const shell = new Proxy(target, {
      get: (t, k, r) => ours("ProxyGet")(k, r),
      set: (t, k, v, r) => ours("ProxySet")(k, v, r),
      has: (t, k) => ours("ProxyHas")(k),
      deleteProperty: (t, k) => ours("ProxyDeleteProperty")(k),
      ownKeys: () => ours("ProxyOwnPropertyKeys")(2),
      getOwnPropertyDescriptor: (t, k) => ours("ProxyGetOwnPropertyDescriptorObject")(k),
      defineProperty: (t, k, d) => ours("ProxyDefineOwnPropertyFromRecord")(k, Object.assign(Object.create(null), d)),
      getPrototypeOf: () => ours("ProxyGetPrototypeOf")(),
      setPrototypeOf: (t, v) => ours("ProxySetPrototypeOf")(v),
      isExtensible: () => ours("ProxyIsExtensible")(),
      preventExtensions: () => ours("ProxyPreventExtensions")(),
      apply: (t, self, args) => ours("ProxyCall")(self, args),
      construct: (t, args, nt) => ours("ProxyConstruct")(args, nt),
    });
    ourProxies.set(shell, { target, handler });
    return shell;
  },
  ProxyTarget: (p) => {
    const st = ourProxies.get(p);
    check(st !== undefined, "ProxyTarget");
    if (st.target === null) throw new TypeError("revoked proxy");
    return st.target;
  },
  ProxyHandler: (p) => {
    const st = ourProxies.get(p);
    check(st !== undefined, "ProxyHandler");
    if (st.handler === null) throw new TypeError("revoked proxy");
    return st.handler;
  },
  ProxyRevoke: (p) => {
    const st = ourProxies.get(p);
    st.target = st.handler = null;
  },
  NewBuiltinClosure: (kind, data, length) => {
    check(kind === 15, "NewBuiltinClosure kind");
    const f = () => installed["%ProxyRevokeFunction"].f.call(data);
    Object.defineProperty(f, "name", { value: "" });
    return f;
  },
  ConstructWithThis: () => {
    throw new Error("intrinsic misuse: ConstructWithThis is not stood in");
  },
  ConstructWithArray: (f, args, nt) => (check(Array.isArray(args), "ConstructWithArray"), Reflect.construct(f, args, nt)),
  GetWithReceiver: (o, k, r) => (check(isObject(o) && !isObject(k), "GetWithReceiver"), Reflect.get(o, k, r)),
  SetWithReceiver: (o, k, v, r) => (check(isObject(o) && !isObject(k), "SetWithReceiver"), Reflect.set(o, k, v, r)),
  DeleteProperty: (o, k) => (check(isObject(o) && !isObject(k), "DeleteProperty"), Reflect.deleteProperty(o, k)),
  CollectionCreate: (cls, proto) => Object.setPrototypeOf(new [Map, Set, WeakMap, WeakSet][cls](), proto),
  CollectionClass: (v) => {
    for (const [i, C] of [[0, Map], [1, Set], [2, WeakMap], [3, WeakSet]]) {
      try {
        C.prototype.has.call(v, {});
        return i;
      } catch {}
    }
    return -1;
  },
  CollectionGet: (m, k) => (m instanceof Map || isMapLike(m) ? Map.prototype.get.call(m, k) : WeakMap.prototype.get.call(m, k)),
  CollectionHas: (m, k) => collProto(m).has.call(m, k),
  CollectionSet: (m, k, v) => {
    const p = collProto(m);
    if (p.set) p.set.call(m, k, v);
    else p.add.call(m, k);
  },
  CollectionDelete: (m, k) => collProto(m).delete.call(m, k),
  CollectionClear: (m) => collProto(m).clear.call(m),
  CollectionSize: (m) => Reflect.get(collProto(m), "size", m),
  CollectionIterator: (m, kind) => {
    const p = collProto(m);
    const it = [p.keys, p.values, p.entries][kind].call(m);
    iterState.set(it, {});
    return it;
  },
  CollectionIteratorNext: (it, isSet) => (isSet ? SetIterNext : MapIterNext).call(it),
  CollectionIteratorStep: (it) => {
    const r = it.next();
    if (r.done) return false;
    const st = iterState.get(it);
    if (Array.isArray(r.value)) [st.k, st.v] = r.value;
    else st.k = st.v = r.value;
    return true;
  },
  CollectionIteratorKey: (it) => iterState.get(it).k,
  CollectionIteratorValue: (it) => iterState.get(it).v,
  CanBeHeldWeakly: (v) => isObject(v) || (typeof v === "symbol" && Symbol.keyFor(v) === undefined),
  DateNow: () => 1646177400000,
  DateCreate: (proto, t) => Object.setPrototypeOf(new Date(t), proto),
  DateValue: (v) => Date.prototype.getTime.call(v),
  DateSetValue: (v, t) => Date.prototype.setTime.call(v, t),
  DateFromFields: (list, local) => {
    check(Array.isArray(list) && list.every((x) => typeof x === "number"), "DateFromFields");
    if (!local) return Date.UTC(...list);
    const f = [0, 0, 1, 0, 0, 0, 0];
    list.forEach((x, i) => (f[i] = x));
    return new Date(f[0], f[1], f[2], f[3], f[4], f[5], f[6]).getTime();
  },
  DateGetField: (t, f, local) => {
    const names = ["FullYear", "Month", "Date", "Hours", "Minutes", "Seconds", "Milliseconds", "Day"];
    return new Date(t)["get" + (local ? "" : "UTC") + names[f]]();
  },
  DateSetFields: (t, first, args, local) => {
    const names = ["FullYear", "Month", "Date", "Hours", "Minutes", "Seconds", "Milliseconds"];
    const d = new Date(t);
    return d["set" + (local ? "" : "UTC") + names[first]](...args);
  },
  DateFormat: (t, fmt, part) => {
    const d = new Date(t);
    const m = [["toUTCString"], ["toDateString", "toTimeString", "toString"], ["toISOString"], ["toLocaleDateString", "toLocaleTimeString", "toLocaleString"]][fmt];
    return d[fmt === 1 || fmt === 3 ? m[part - 1] : m[0]]();
  },
  DateParse: (s) => (check(typeof s === "string", "DateParse"), Date.parse(s)),
  DateTimezoneOffset: (t) => new Date(t).getTimezoneOffset(),
  ArrayBufferCreate: (proto, len, max) => Object.setPrototypeOf(max === undefined ? new ArrayBuffer(len) : new ArrayBuffer(len, { maxByteLength: max }), proto),
  BufferInfo: (b, f) => {
    const g = (n) => Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, n).get.call(b);
    return [g("byteLength"), g("maxByteLength"), g("resizable"), g("detached")][f];
  },
  BufferDetach: (b) => void b.transfer(),
  BufferResize: (b, n) => b.resize(n),
  BufferTransfer: (b, len, max) => (max === undefined ? ArrayBuffer.prototype.transferToFixedLength.call(b, len) : ArrayBuffer.prototype.transfer.call(b, len)),
  BufferCopy: (src, from, dst, to, count) => new Uint8Array(dst, to, count).set(new Uint8Array(src, from, count)),
  IsTypedArray: (v) => taKind(v) >= 0,
  DataViewCreate: (proto, buffer, off, len) => {
    const v = Object.setPrototypeOf(len < 0 ? new DataView(buffer, off) : new DataView(buffer, off, len), proto);
    dataViews.set(v, [buffer, off, len]);
    return v;
  },
  DataViewField: (v, f) => {
    const st = dataViews.get(v);
    if (!st) throw new TypeError("DataView object expected");
    return st[f];
  },
  DataViewGet: (v, at, kind, little) => new DataView(dataViews.get(v)[0])["get" + DV_NAMES[kind]](at, little),
  DataViewSet: (v, at, kind, n, little) => {
    check(typeof n === (kind >= 10 ? "bigint" : "number"), "DataViewSet");
    new DataView(dataViews.get(v)[0])["set" + DV_NAMES[kind]](at, n, little);
  },
  IsDataView: (v) => {
    try {
      Object.getOwnPropertyDescriptor(DataView.prototype, "buffer").get.call(v);
      return true;
    } catch {
      return false;
    }
  },
  RealmConstructor: (name) => (name === "%TypedArray%" ? Object.getPrototypeOf(Int8Array) : globalThis[name]),
  TypedArrayCreate: (proto, kind, buffer, off, len) => {
    const C = TA_CTORS[kind];
    return Object.setPrototypeOf(len < 0 ? new C(buffer, off) : new C(buffer, off, len), proto);
  },
  TypedArrayField: (o, f) => {
    const kind = taKind(o);
    if (kind < 0) throw new TypeError("not a TypedArray");
    const g = (n) => Object.getOwnPropertyDescriptor(TA_PROTO, n).get.call(o);
    const buffer = g("buffer");
    const oob = buffer.detached || (g("length") === 0 && g("byteOffset") === 0 && false);
    return [kind, buffer, g("byteOffset"), g("length"), g("byteLength"), oob, false][f];
  },
  TypedArrayGet: (o, i) => (taKind(o) >= 0 ? o[i] : undefined),
  TypedArraySet: (o, i, n) => {
    if (typeof n !== (taKind(o) >= 10 ? "bigint" : "number")) {
      throw new TypeError(taKind(o) >= 10 ? "cannot convert to bigint" : "cannot convert bigint to number");
    }
    if (Number.isInteger(i) && i >= 0) o[i] = n;
  },
  TypedArrayCopy: (src, from, dst, to, count) => {
    const vals = [];
    for (let i = 0; i < count; i++) vals.push(src[from + i]);
    for (let i = 0; i < count; i++) dst[to + i] = vals[i];
  },
  TypedArrayKindProto: (k) => TA_CTORS[k].prototype,
  Base64Encode: (o, url, omit) => Uint8Array.prototype.toBase64.call(o, { alphabet: url ? "base64url" : "base64", omitPadding: omit }),
  HexEncode: (o) => Uint8Array.prototype.toHex.call(o),
  Base64Decode: (s, url, last, max) => decodeRecord(() => {
    const t = new Uint8Array(Math.min(max, s.length));
    const r = t.setFromBase64(s, { alphabet: url ? "base64url" : "base64", lastChunkHandling: ["loose", "strict", "stop-before-partial"][last] });
    return [r.read, t.slice(0, r.written)];
  }, s, url, last),
  HexDecode: (s, max) => decodeRecord(() => {
    const t = new Uint8Array(Math.min(max, s.length));
    const r = t.setFromHex(s);
    return [r.read, t.slice(0, r.written)];
  }),
  NewSyntaxError: (m) => new SyntaxError(m),
  StrLocaleCompare: (a, b) => a.localeCompare(b),
  StrCharCodeAt: (s, i) => (check(typeof s === "string" && Number.isInteger(i) || !isFinite(i), "StrCharCodeAt"), i >= 0 && i < s.length ? s.charCodeAt(i) : NaN),
  StrCodePointAt: (s, i) => (i >= 0 && i < s.length ? s.codePointAt(i) : undefined),
  StrSubstring: (s, a, b) => (check(typeof s === "string" && Number.isInteger(a) && Number.isInteger(b) && a >= 0 && b >= 0, "StrSubstring " + a + " " + b), s.substring(a, b)),
  StrIndexOf: (s, t, from) => (check(typeof t === "string" && from >= 0, "StrIndexOf"), s.indexOf(t, from)),
  StrLastIndexOf: (s, t, from) => (check(typeof t === "string" && from >= 0, "StrLastIndexOf"), s.lastIndexOf(t, from)),
  StrToCase: (s, upper) => (upper ? s.toUpperCase() : s.toLowerCase()),
  StrTrim: (s, w) => (w === 0 ? s.trim() : w === 1 ? s.trimStart() : s.trimEnd()),
  StrIsWellFormed: (s) => s.isWellFormed(),
  StrToWellFormed: (s) => s.toWellFormed(),
  StrRepeat: (s, n) => (check(Number.isInteger(n) && n >= 0, "StrRepeat"), s.repeat(n)),
  StrFromCodeUnits: (list) => String.fromCharCode(...list),
  StrFromCodePoints: (list) => String.fromCodePoint(...list),
  StrNormalize: (s, f) => s.normalize(["NFC", "NFD", "NFKC", "NFKD"][f]),
  CompareStrings: (a, b) => (check(typeof a === "string" && typeof b === "string", "CompareStrings"), a < b ? -1 : a > b ? 1 : 0),
  StringCodePoints: (s) => [...s],
  SortNumbers: (list) => {
    const big = list.length > 0 && typeof list[0] === "bigint";
    check(Array.isArray(list) && list.every((v) => typeof v === (big ? "bigint" : "number")), "SortNumbers");
    const out = (big ? BigInt64Array : Float64Array).from(list).sort();
    for (let i = 0; i < out.length; i++) list[i] = out[i];
  },
  SortPrimitives: (list) => {
    check(Array.isArray(list), "SortPrimitives");
    if (list.some((v) => isObject(v) || typeof v === "symbol")) return false;
    const undef = list.filter((v) => v === undefined).length;
    const keyed = list.filter((v) => v !== undefined).map((v) => [String(v), v]);
    keyed.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    const out = keyed.map((p) => p[1]).concat(new Array(undef).fill(undefined));
    for (let i = 0; i < out.length; i++) list[i] = out[i];
    return true;
  },
};
for (const name of ["abs", "acos", "acosh", "asin", "asinh", "atan", "atanh", "atan2", "cbrt", "hypot", "ceil", "clz32", "cos", "cosh", "exp", "expm1", "floor", "fround", "imul", "log", "log1p", "log10", "log2", "round", "sign", "sin", "sinh", "sqrt", "tan", "tanh", "trunc"]) {
  I["F64" + name[0].toUpperCase() + name.slice(1)] = (x, y) => {
    check(typeof x === "number", "F64" + name);
    return Math[name](x, y);
  };
}

PRELUDE;

// ---- comparisons --------------------------------------------------------------------

function show(v, seen = new Set()) {
  if (typeof v === "symbol") return v.toString();
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v === "number") return Object.is(v, -0) ? "-0" : String(v);
  if (typeof v === "function") return "function " + v.name;
  if (!isObject(v)) return String(v);
  if (seen.has(v)) return "<cycle>";
  seen.add(v);
  const proto = Object.getPrototypeOf(v);
  const tag = Array.isArray(v) ? "Array" : proto && proto.constructor ? proto.constructor.name : "null";
  const keys = Reflect.ownKeys(v).map((k) => {
    const d = Object.getOwnPropertyDescriptor(v, k);
    return String(k) + ("value" in d ? "=" + show(d.value, seen) : "=<accessor>");
  });
  return tag + "{" + keys.join(",") + "}";
}

// Methods whose spec steps use Get where qjs probes with HasProperty first and skips the
// Get of a missing element (JS_TryGetPropertyInt64): their trap logs are not compared,
// only the results.
let ignoreHas = false;
let sameSetFailures = false;
function outcome(fn) {
  const log = [];
  let result;
  try {
    result = "return " + show(fn(log));
  } catch (e) {
    result = "throw " + (isObject(e) ? e.constructor.name + ": " + e.message : show(e));
  }
  const shown = ignoreHas ? [] : log;
  if (sameSetFailures) {
    // A failed [[Set]] (strict) of a proxy without a set trap: qjs reports the target's
    // reason; ours (behind a native shell, and djs) report the proxy's.
    result = result.replace(/TypeError: (object is not extensible|'\w+' is read-only|proxy: cannot set property|proxy: defineProperty exception)$/, "TypeError: <set failed>");
  }
  return shown.join(" ") + " => " + result;
}

// A proxy around `target` that logs has/get/defineProperty traps.
function logged(target, log) {
  return new Proxy(target, {
    has(t, k) {
      log.push("has:" + String(k));
      return Reflect.has(t, k);
    },
    get(t, k, r) {
      log.push("get:" + String(k));
      return Reflect.get(t, k, r);
    },
    defineProperty(t, k, d) {
      log.push("define:" + String(k));
      return Reflect.defineProperty(t, k, d);
    },
  });
}

let count = 0;
let failures = 0;
function same(name, native, run) {
  count++;
  const ours = installed[name].f;
  const a = outcome((log) => run(native, log));
  const b = outcome((log) => run(ours, log));
  if (a !== b) {
    failures++;
    console.log("MISMATCH " + name + " #" + count + "\n  native: " + a + "\n  ours:   " + b);
  }
}

const AP = Array.prototype;
const toPrim = (v) => ({
  [Symbol.toPrimitive](hint) {
    return v + ":" + hint;
  },
});
const valueOfObj = (v, log) => ({
  valueOf() {
    log.push("valueOf");
    return v;
  },
  toString() {
    log.push("toString");
    return "s" + v;
  },
});
class MyArray extends Array {}
const subjects = (log) => [
  [1, 2, 3],
  [1, , 3],
  [],
  ["a", null, undefined, NaN, -0, 0, { x: 1 }],
  { length: "2", 0: "a", 1: "b" },
  { length: 3.7, 0: 1, 1: 2, 2: 3 },
  { length: -5, 0: 1 },
  { length: valueOfObj(2, log), 0: "v", 1: "w" },
  "str",
  logged([10, , 30], log),
  logged({ length: 2, 0: "p", 1: "q" }, log),
  MyArray.of(1, 2),
];
const nSubjects = subjects([]).length;

for (let i = 0; i < nSubjects; i++) {
  for (const [name, native] of [
    ["Array.prototype.map", AP.map],
    ["Array.prototype.forEach", AP.forEach],
  ]) {
    same(name, native, (f, log) =>
      f.call(subjects(log)[i], function (v, k, o) {
        log.push("cb:" + show(v) + "@" + k + (this === undefined ? "" : ":this=" + show(this)));
        return k * 2;
      }),
    );
    same(name, native, (f, log) => f.call(subjects(log)[i], (v) => v, { thisArg: 1 }));
    same(name, native, (f, log) => f.call(subjects(log)[i], "not callable"));
  }
  for (const [name, native] of [
    ["Array.prototype.indexOf", AP.indexOf],
    ["Array.prototype.includes", AP.includes],
  ]) {
    for (const search of [1, 3, "a", "b", NaN, 0, -0, undefined, null, 30]) {
      for (const from of [undefined, 0, 1, -1, -100, 100, Infinity, -Infinity, "1", NaN, 2.9]) {
        same(name, native, (f, log) => f.call(subjects(log)[i], search, from));
      }
    }
    same(name, native, (f, log) => f.call(subjects(log)[i], 2, valueOfObj(1, log)));
  }
  for (const sep of [undefined, ",", "", " - ", null, 1, toPrim("sep")]) {
    same("Array.prototype.join", AP.join, (f, log) => f.call(subjects(log)[i], sep));
  }
}

// Edge cases outside the subject matrix.
for (const name of ["map", "forEach", "indexOf", "join", "includes"]) {
  const native = AP[name];
  const full = "Array.prototype." + name;
  same(full, native, (f) => f.call(undefined, () => 1));
  same(full, native, (f) => f.call(null, () => 1));
  same(full, native, (f) => f.call(5, () => 1));
  // Mutation while iterating.
  same(full, native, (f, log) => {
    const a = [1, 2, 3, 4];
    return [
      f.call(a, (v, k) => {
        log.push(v);
        if (k === 0) a.push(9);
        if (k === 1) delete a[2];
        return v;
      }),
      a,
    ];
  });
  same(full, native, () => installed[full].length === native.length);
}
// Elements with valueOf/toString/Symbol.toPrimitive, and failing conversions.
same("Array.prototype.join", AP.join, (f, log) => f.call([valueOfObj(1, log), toPrim("t"), [1, [2, 3]]], "|"));
same("Array.prototype.join", AP.join, (f) => f.call([{ toString: () => ({}), valueOf: () => ({}) }]));
same("Array.prototype.join", AP.join, (f) => f.call([{ toString: null, valueOf: () => "v" }]));
same("Array.prototype.join", AP.join, (f) => f.call([{ [Symbol.toPrimitive]: () => ({}) }]));
same("Array.prototype.join", AP.join, (f) => f.call([{ [Symbol.toPrimitive]: 5 }]));
same("Array.prototype.join", AP.join, (f) => f.call([Symbol("s")]));
same("Array.prototype.join", AP.join, (f) => f.call({ length: 2, 0: 1, 1: 2 }, { toString: () => "!" }));
// Species: constructor and Symbol.species lookups.
same("Array.prototype.map", AP.map, (f) => f.call(MyArray.of(1, 2), (v) => v) instanceof MyArray);
same("Array.prototype.map", AP.map, (f) => {
  const a = [1, 2];
  a.constructor = { [Symbol.species]: null };
  return f.call(a, (v) => v + 1);
});
same("Array.prototype.map", AP.map, (f) => {
  const a = [1];
  a.constructor = { [Symbol.species]: 5 };
  return f.call(a, (v) => v);
});
same("Array.prototype.map", AP.map, (f) => {
  const a = [1];
  a.constructor = { [Symbol.species]: () => [] };
  return f.call(a, (v) => v);
});
same("Array.prototype.map", AP.map, (f) => {
  const a = [1];
  a.constructor = 5;
  return f.call(a, (v) => v);
});
same("Array.prototype.map", AP.map, (f) => f.call({ length: 4294967296 }, (v) => v));

// ---- 0031: Object, Function, Error, Array, Math, String -------------------------------

const keyish = (log) => [
  "x",
  1,
  Symbol.iterator,
  valueOfObj("k", log),
  toPrim("p"),
  { toString: () => ({}), valueOf: () => ({}) },
];
const descs = (log) => [
  { value: 1 },
  { value: 2, writable: true, enumerable: 1, configurable: "" },
  { get() {} },
  { get: undefined, set: undefined },
  { get: 5 },
  { set: {} },
  { get() {}, value: 1 },
  { set() {}, writable: false },
  logged({ value: 3, enumerable: true }, log),
  logged({ get() {}, configurable: true }, log),
  5,
  null,
];
for (let i = 0; i < keyish([]).length; i++) {
  for (let j = 0; j < descs([]).length; j++) {
    same("Object.defineProperty", Object.defineProperty, (f, log) => {
      const o = {};
      return [f(o, keyish(log)[i], descs(log)[j]), Object.getOwnPropertyDescriptor(o, "x")];
    });
  }
  same("Object.getOwnPropertyDescriptor", Object.getOwnPropertyDescriptor, (f, log) =>
    f({ x: 1, 1: 2, [Symbol.iterator]: 3, kk: 4 }, keyish(log)[i]),
  );
  same("Object.prototype.hasOwnProperty", Object.prototype.hasOwnProperty, (f, log) =>
    f.call({ x: 1, 1: 2, [Symbol.iterator]: 3 }, keyish(log)[i]),
  );
  same("Object.prototype.propertyIsEnumerable", Object.prototype.propertyIsEnumerable, (f, log) =>
    f.call(Object.defineProperty({ x: 1 }, 1, { value: 1 }), keyish(log)[i]),
  );
}
for (const o of [{}, [1, 2], { b: 1, a: 2, 1: 3, [Symbol.iterator]: 4 }, Object.freeze({ x: 1 })]) {
  same("Object.getOwnPropertyNames", Object.getOwnPropertyNames, (f) => f(o));
  same("Object.defineProperty", Object.defineProperty, (f) => f(o, "x", { value: 9 }));
  same("Object.defineProperty", Object.defineProperty, (f) => f(o, "length", { value: 1 }));
}
for (const v of [undefined, null, 1]) {
  same("Object.getOwnPropertyNames", Object.getOwnPropertyNames, (f) => f(v));
  same("Object.defineProperty", Object.defineProperty, (f) => f(v, "x", {}));
  same("Object.prototype.hasOwnProperty", Object.prototype.hasOwnProperty, (f) => f.call(v, "x"));
}
// Array lengths through defineProperty.
for (const len of [0, 2, 5, -1, 1.5, 4294967295, 4294967296, "3", toPrim("x")]) {
  same("Object.defineProperty", Object.defineProperty, (f) => f([1, 2, 3], "length", { value: len }));
}
same("Object.defineProperty", Object.defineProperty, (f) =>
  f(Object.defineProperty([1, 2, 3], "length", { writable: false }), "length", { value: 1 }),
);

const FP = Function.prototype;
function probe(a, b) {
  return [this === undefined ? "u" : show(this), a, b, arguments.length];
}
for (const [thisArg, args] of [
  [undefined, []],
  [1, [2]],
  [{ o: 1 }, [3, 4, 5]],
]) {
  same("Function.prototype.call", FP.call, (f) => f.call(probe, thisArg, ...args));
  same("Function.prototype.apply", FP.apply, (f) => f.call(probe, thisArg, args));
  same("Function.prototype.bind", FP.bind, (f) => {
    const b = f.call(probe, thisArg, ...args);
    return [b(7, 8), b.name, b.length, Reflect.ownKeys(b)];
  });
}
for (const argArray of [null, undefined, { length: 2, 0: "a" }, "str", 5, logged([1, 2], [])]) {
  same("Function.prototype.apply", FP.apply, (f) => f.call(probe, null, argArray));
}
for (const target of [5, {}, null]) {
  same("Function.prototype.call", FP.call, (f) => f.call(target));
  same("Function.prototype.apply", FP.apply, (f) => f.call(target));
  same("Function.prototype.bind", FP.bind, (f) => f.call(target));
}
same("Function.prototype.bind", FP.bind, (f) => {
  const g = function () {};
  Object.defineProperty(g, "length", { value: Infinity });
  Object.defineProperty(g, "name", { value: 42 });
  const b = f.call(g, null, 1, 2);
  return [b.length, b.name];
});
same("Function.prototype.bind", FP.bind, (f) => {
  const g = function () {};
  Object.defineProperty(g, "length", { value: 2.5 });
  return f.call(g, null).length;
});
same("Function.prototype.bind", FP.bind, (f) => {
  function C(a, b) {
    this.ab = [a, b];
  }
  const B = f.call(C, null, 1);
  const o = new B(2);
  return [o.ab, o instanceof C, o instanceof B];
});

for (const C of ["Error", "TypeError", "RangeError", "ReferenceError", "SyntaxError", "EvalError", "URIError", "InternalError"]) {
  const native = globalThis[C];
  const full = "globalThis." + C;
  for (const args of [[], ["m"], [undefined], [5, { cause: 1 }], ["m", { x: 1 }], ["m", logged({ cause: 2 }, [])], [toPrim("t")]]) {
    same(full, native, (f) => f(...args));
    same(full, native, (f) => Reflect.construct(f, args));
  }
  same(full, native, () => installed[full].length === native.length);
}
for (const o of [new Error("x"), new TypeError(), { name: "N" }, { message: "M" }, { name: "", message: "m" }, { name: toPrim("n"), message: 5 }, 5]) {
  same("Error.prototype.toString", Error.prototype.toString, (f) => f.call(o));
}

same("Array.isArray", Array.isArray, (f) => [f([]), f({}), f(5), f(new Proxy([], {}))]);
for (const args of [[], [3], [0], [-1], [1.5], ["3"], [1, 2], [4294967295]]) {
  same("globalThis.Array", Array, (f) => f(...args));
  same("globalThis.Array", Array, (f) => Reflect.construct(f, args));
}
const pushTargets = (log) => [[], [1], { length: 2 }, { length: "x" }, logged([5], log), 5, null];
for (let i = 0; i < pushTargets([]).length; i++) {
  same("Array.prototype.push", AP.push, (f, log) => {
    const t = pushTargets(log)[i];
    return [f.call(t, 1, 2), t];
  });
  same("Array.prototype.push", AP.push, (f, log) => {
    const t = pushTargets(log)[i];
    return [f.call(t), t];
  });
}
same("Array.prototype.push", AP.push, (f) => f.call({ length: 2 ** 53 - 1 }, 1));
same("Array.prototype.push", AP.push, (f) => f.call(Object.freeze([1]), 1));

for (const [b, e] of [[2, 10], [2, -1], [NaN, 0], [1, Infinity], [-8, 1 / 3], ["3", "2"], [toPrim("x"), 1], [Symbol(), 1]]) {
  same("Math.pow", Math.pow, (f) => f(b, e));
}
for (const args of [[], [undefined], [null], [5], [Symbol("d")], [Symbol()], [toPrim("v")], [{}], [[1, 2]]]) {
  same("globalThis.String", String, (f) => f(...args));
  same("globalThis.String", String, (f) => {
    const o = Reflect.construct(f, args);
    return [typeof o, String.prototype.valueOf.call(o), Object.keys(o)];
  });
}
for (const v of ["s", new String("w"), 5, {}, null]) {
  same("String.prototype.toString", String.prototype.toString, (f) => f.call(v));
  same("String.prototype.valueOf", String.prototype.valueOf, (f) => f.call(v));
}

// ---- 0031 part 2 ---------------------------------------------------------------------

const objs = (log) => [
  { a: 1, b: 2 },
  [1, , 3],
  { get g() { log.push("getter"); return 1; }, x: 2 },
  logged({ p: 1, q: 2 }, log),
  Object.defineProperty({ v: 1 }, "hidden", { value: 2, enumerable: false }),
  Object.freeze({ f: 1 }),
  Object.seal({ s: 1 }),
  Object.preventExtensions({ e: 1 }),
  "str",
  5,
  true,
  Symbol("s"),
  null,
  undefined,
  function named() {},
  new Number(3),
  new Boolean(false),
  Object.create(null),
];
const nObjs = objs([]).length;
const OC = Object;
for (let i = 0; i < nObjs; i++) {
  for (const name of ["keys", "values", "entries", "getPrototypeOf", "isFrozen", "isSealed", "isExtensible", "getOwnPropertySymbols", "getOwnPropertyDescriptors", "freeze", "seal", "preventExtensions"]) {
    same("Object." + name, OC[name], (f, log) => f(objs(log)[i]));
  }
  same("Object.hasOwn", OC.hasOwn, (f, log) => f(objs(log)[i], "a"));
  same("Object.assign", OC.assign, (f, log) => f({}, objs(log)[i]));
  same("Object.assign", OC.assign, (f, log) => f(objs(log)[i], { z: 1 }));
  same("Object.prototype.toString", Object.prototype.toString, (f, log) => f.call(objs(log)[i]));
  same("Object.prototype.valueOf", Object.prototype.valueOf, (f, log) => typeof f.call(objs(log)[i]));
  same("Object.prototype.toLocaleString", Object.prototype.toLocaleString, (f, log) => f.call(objs(log)[i]));
  same("Object.prototype.isPrototypeOf", Object.prototype.isPrototypeOf, (f, log) => f.call(Object.prototype, objs(log)[i]));
  same("Object.setPrototypeOf", OC.setPrototypeOf, (f, log) => f(objs(log)[i], Array.prototype));
  same("Object.setPrototypeOf", OC.setPrototypeOf, (f, log) => f({}, objs(log)[i]));
  same("Object.create", OC.create, (f, log) => f(objs(log)[i]));
  // qjs reads every descriptor object before converting any; the spec (and djs)
  // converts each right after reading it. Only the logged proxy shows the difference.
  if (i !== 3) {
    same("Object.defineProperties", OC.defineProperties, (f, log) => f({}, objs(log)[i]));
  }
  same("Object.is", OC.is, (f, log) => f(objs(log)[i], objs(log)[i]));
}
same("Object.create", OC.create, (f, log) => f({}, logged({ a: { value: 1, enumerable: true }, b: { get() {} } }, log)));
same("Object.defineProperties", OC.defineProperties, (f) => f({}, { a: { value: 1 }, b: 5 }));
same("Object.defineProperties", OC.defineProperties, (f) => f(5, {}));
same("Object.prototype.toString", Object.prototype.toString, (f) => f.call({ [Symbol.toStringTag]: "Tagged" }));
same("Object.prototype.toString", Object.prototype.toString, (f) => f.call({ [Symbol.toStringTag]: 5 }));
same("Object.prototype.toString", Object.prototype.toString, (f) => f.call(new Error("x")));
for (const [a, b] of [[NaN, NaN], [0, -0], [-0, -0], [1, "1"], [null, undefined]]) {
  same("Object.is", OC.is, (f) => f(a, b));
}

for (const v of [true, false, new Boolean(true), 1, "true", {}, null, undefined]) {
  same("globalThis.Boolean", Boolean, (f) => f(v));
  same("globalThis.Boolean", Boolean, (f) => typeof Reflect.construct(f, [v]));
  same("Boolean.prototype.toString", Boolean.prototype.toString, (f) => f.call(v));
  same("Boolean.prototype.valueOf", Boolean.prototype.valueOf, (f) => f.call(v));
}
const sym = Symbol("d");
for (const v of [sym, Object(sym), Symbol(), Symbol.iterator, "s", {}, undefined]) {
  same("Symbol.prototype.toString", Symbol.prototype.toString, (f) => f.call(v));
  same("Symbol.prototype.valueOf", Symbol.prototype.valueOf, (f) => String(f.call(v)));
  same("get Symbol.prototype.description", Object.getOwnPropertyDescriptor(Symbol.prototype, "description").get, (f) => f.call(v));
  same("Symbol.keyFor", Symbol.keyFor, (f) => f(v));
}
for (const d of [undefined, "x", 5, null, toPrim("p")]) {
  same("globalThis.Symbol", Symbol, (f) => f(d).description);
  same("globalThis.Symbol", Symbol, (f) => Reflect.construct(f, [d]));
  same("Symbol.for", Symbol.for, (f) => f(d) === Symbol.for(String(d)));
}
const numbers = [0, -0, 1, -1.5, 255, 0.1, 1e21, 123.456, 1e-7, NaN, Infinity, -Infinity, 2 ** 53, new Number(7), "5", {}, true];
for (const x of numbers) {
  for (const r of [undefined, 2, 16, 36, 1, 37, "10", 10.9]) {
    same("Number.prototype.toString", Number.prototype.toString, (f) => f.call(x, r));
  }
  for (const d of [undefined, 0, 2, 20, 100, 101, -1, 2.7, Infinity]) {
    same("Number.prototype.toFixed", Number.prototype.toFixed, (f) => f.call(x, d));
    same("Number.prototype.toExponential", Number.prototype.toExponential, (f) => f.call(x, d));
    same("Number.prototype.toPrecision", Number.prototype.toPrecision, (f) => f.call(x, d));
  }
  same("Number.prototype.valueOf", Number.prototype.valueOf, (f) => f.call(x));
  for (const name of ["isFinite", "isInteger", "isNaN", "isSafeInteger"]) {
    same("Number." + name, Number[name], (f) => f(x));
  }
  same("globalThis.isNaN", isNaN, (f) => f(x));
  same("globalThis.isFinite", isFinite, (f) => f(x));
  same("globalThis.Number", Number, (f) => f(x));
  same("globalThis.Number", Number, (f) => Number.prototype.valueOf.call(Reflect.construct(f, [x])));
}
same("globalThis.Number", Number, (f) => f());
for (const x of [0n, -5n, 2n ** 64n, Object(3n), { valueOf: () => 7n }]) {
  same("globalThis.Number", Number, (f) => f(x));
}
const bigints = [0n, 1n, -1n, 255n, -(2n ** 70n), 2n ** 64n - 1n, Object(9n), 5, 1.5, -0, NaN, Infinity, "12", " 0x1f ", "-0x1", "1.5", "", true, null, undefined, Symbol(), {}, toPrim("7"), { valueOf: () => 4n }];
for (const x of bigints) {
  same("globalThis.BigInt", BigInt, (f) => f(x));
  same("globalThis.BigInt", BigInt, (f) => Reflect.construct(f, [x]));
  for (const r of [undefined, 2, 16, 36, 1, 37, "10", 10.9]) {
    same("BigInt.prototype.toString", BigInt.prototype.toString, (f) => f.call(x, r));
  }
  same("BigInt.prototype.toLocaleString", BigInt.prototype.toLocaleString, (f) => f.call(x));
  same("BigInt.prototype.valueOf", BigInt.prototype.valueOf, (f) => f.call(x));
  for (const n of [0, 1, 8, 64, 65, -1, 2 ** 53, "3", undefined, 1.9]) {
    same("BigInt.asIntN", BigInt.asIntN, (f) => f(n, x));
    same("BigInt.asUintN", BigInt.asUintN, (f) => f(n, x));
  }
}
for (const name of ["max", "min", "hypot"]) {
  same("Math." + name, Math[name], (f) => f());
  same("Math." + name, Math[name], (f) => f(1, 2, 3, NaN, Infinity));
  same("Math." + name, Math[name], (f) => f(NaN, Infinity));
  same("Math." + name, Math[name], (f, log) => f(valueOfObj(1, log), valueOfObj(2, log)));
  same("Math." + name, Math[name], () => installed["Math." + name].length === Math[name].length);
}
function C0() {}
const bound = C0.bind(null);
for (const v of [new C0(), {}, 5, Object.create(C0.prototype)]) {
  same("Function.prototype[Symbol.hasInstance]", Function.prototype[Symbol.hasInstance], (f) => f.call(C0, v));
  same("Function.prototype[Symbol.hasInstance]", Function.prototype[Symbol.hasInstance], (f) => f.call({}, v));
}

// ---- 0034: Array ------------------------------------------------------------------------

const arrayMethods = {
  at: [[0], [-1], [5], ["1"], [-Infinity]],
  lastIndexOf: [[1], [3, -2], [2, 100], [undefined], [NaN], [1, -Infinity]],
  find: [[(v) => v === 2], [(v) => false], [5]],
  findIndex: [[(v) => v === 2], [(v) => false]],
  findLast: [[(v, k) => k < 2], [(v) => false]],
  findLastIndex: [[(v, k) => k < 2], [(v) => false]],
  every: [[(v) => v !== 3], [(v) => true], ["no"]],
  some: [[(v) => v === 3], [(v) => false]],
  filter: [[(v, k) => k % 2 === 0], [(v) => true], [null]],
  reduce: [[(a, b) => String(a) + "|" + String(b)], [(a, b) => a + 1, 10], [5]],
  reduceRight: [[(a, b) => String(a) + "|" + String(b)], [(a, b) => a + 1, 10]],
  concat: [[], [[9, 8]], [9, [7, [6]]], [{ length: 1, 0: "x", [Symbol.isConcatSpreadable]: true }], ["s"]],
  slice: [[], [1], [-2], [1, -1], [5, 1], ["1", "3"], [undefined, 2], [-Infinity, Infinity]],
  splice: [[], [1], [1, 1], [0, 0, "a", "b"], [-1, 5, "z"], [undefined], [1, undefined], [10, 1, "q"]],
  copyWithin: [[0, 1], [1, 0], [0, 1, 2], [-1, 0], [0, -2, -1]],
  fill: [[0], [7, 1], [7, -2, -1], [7, 5]],
  reverse: [[]],
  pop: [[]],
  shift: [[]],
  unshift: [[], ["a"], ["a", "b"]],
  flat: [[], [0], [Infinity], [-1]],
  flatMap: [[(v) => [v, [v]]], [(v) => v], [3]],
  sort: [[], [(a, b) => b - a], [(a, b) => 0], [(a, b) => (String(a) < String(b) ? -1 : 1)], [5]],
  toSorted: [[], [(a, b) => b - a], [5]],
  toReversed: [[]],
  toSpliced: [[], [1], [1, 1], [0, 0, "a"], [-1, 5, "z"], [undefined]],
  with: [[0, "w"], [-1, "w"], [10, "w"], [-10, "w"]],
  toString: [[]],
  toLocaleString: [[]],
  forEach: [[(v) => v]],
  map: [[(v, k) => k]],
  push: [[1, 2]],
  join: [["-"]],
};
// sort: qjs writes back only the elements that moved, the spec writes all of them, so
// the logged proxies and the read-only String object differ.
const sortSkip = new Set([8, 9, 10]);
for (const name of Object.keys(arrayMethods)) {
  const native = AP[name];
  ignoreHas = ["at", "toSorted", "toReversed", "toSpliced", "with"].includes(name);
  for (let i = 0; i < nSubjects; i++) {
    if (name === "sort" && sortSkip.has(i)) continue;
    for (const args of arrayMethods[name]) {
      same("Array.prototype." + name, native, (f, log) => {
        const o = subjects(log)[i];
        return [f.apply(o, args), o];
      });
    }
  }
  same("Array.prototype." + name, native, (f) => f.call(null));
  same("Array.prototype." + name, native, () => installed["Array.prototype." + name].length === native.length);
}
ignoreHas = false;
// Mixed-type sorts: the leaf path (primitives) and the generic one (objects).
for (const list of [
  [3, 1, 2],
  [10, 9, 1, "1", "a", true, null, undefined, NaN, -0, 0],
  [undefined, , 3, undefined, 1],
  [{ toString: () => "b" }, "a", { valueOf: () => 1, toString: () => "c" }],
  ["b", "a", "B", "\u00e9", "e"],
  [Symbol("x")],
  [Symbol("x"), 1],
]) {
  same("Array.prototype.sort", AP.sort, (f) => f.call(list.slice()));
  same("Array.prototype.toSorted", AP.toSorted, (f) => f.call(list.slice()));
}
// Stability with a comparator that ties.
same("Array.prototype.sort", AP.sort, (f) => {
  const a = [];
  for (let i = 0; i < 50; i++) a.push({ k: i % 3, i });
  return f.call(a, (x, y) => x.k - y.k).map((e) => e.i).join(",");
});
same("Array.prototype.splice", AP.splice, (f) => f.call({ length: 2 ** 53 - 1 }, 0, 0, 1));
same("Array.prototype.unshift", AP.unshift, (f) => f.call({ length: 2 ** 53 - 1 }, 1));
same("Array.prototype.concat", AP.concat, (f) => f.call(1, 2));
same("Array.prototype.toReversed", AP.toReversed, (f) => f.call({ length: 2 ** 32 }));
same("Array.prototype.with", AP.with, (f) => f.call({ length: 2 ** 32 }, 0, 1));
same("Array.prototype.flat", AP.flat, (f) => f.call([1, [2, [3, [4]]]], 2));
for (const args of [[[1, 2]], [{ length: 2, 0: "a" }], ["abc"], ["a\ud83d\ude00b"], [[1, 2], (v, k) => v * k], [null], [[1], 5], [{ length: -1 }]]) {
  same("Array.from", Array.from, (f) => f(...args));
  same("Array.from", Array.from, (f) => f.call(Object, ...args));
}
same("Array.of", Array.of, (f) => f(1, 2, 3));
same("Array.of", Array.of, (f) => f.call(Object, 1, 2));
same("Array.of", Array.of, (f) => f.call(undefined, 1));
same("get Array.Symbol(Symbol.species)", Object.getOwnPropertyDescriptor(Array, Symbol.species).get, (f) => f.call(5));

// ---- 0034: String -----------------------------------------------------------------------

const SP = String.prototype;
const strSubjects = (log) => ["abcabc", "", "Straße ΑΣ ǅ İ", " \u00a0 x\ty \n\u2028", "a\ud83d\ude00b\ud800", 12345, true, new String("wrap"), valueOfObj("vo", log), toPrim("tp")];
const nStr = strSubjects([]).length;
const stringMethods = {
  at: [[0], [-1], [99], ["1"]],
  charAt: [[0], [-1], [2.7], [99]],
  charCodeAt: [[0], [-1], [1], [99], [NaN]],
  codePointAt: [[0], [1], [2], [99], [-1]],
  indexOf: [["b"], ["c", 3], [""], ["", 99], ["zz"], ["a", -5], [undefined]],
  lastIndexOf: [["b"], ["c", 3], [""], ["a", 0], ["zz"], ["a", NaN], ["a", -Infinity]],
  includes: [["bc"], ["bc", 5], [""], ["x"]],
  startsWith: [["ab"], ["bc", 1], ["", 99], ["abcabcx"]],
  endsWith: [["bc"], ["ab", 2], ["", 0], ["x"]],
  localeCompare: [["abc"], ["b"], [""], [5]],
  slice: [[], [1], [-2], [1, -1], [5, 1], [-Infinity, Infinity], [undefined, 2]],
  substring: [[], [1], [-2], [4, 1], [1, 99], [NaN, 2], [undefined, 2]],
  substr: [[], [1], [-2, 1], [1, 2], [1, -1], [99]],
  concat: [[], ["x", 1, null], [[1, 2]]],
  repeat: [[0], [3], [-1], [Infinity], ["2"]],
  padStart: [[10], [10, "xy"], [2], [10, ""], [8, 0]],
  padEnd: [[10], [10, "xy"], [2], [10, ""]],
  toLowerCase: [[]],
  toUpperCase: [[]],
  toLocaleLowerCase: [[]],
  toLocaleUpperCase: [[]],
  trim: [[]],
  trimStart: [[]],
  trimEnd: [[]],
  isWellFormed: [[]],
  toWellFormed: [[]],
};
for (const name of Object.keys(stringMethods)) {
  const native = SP[name];
  for (let i = 0; i < nStr; i++) {
    for (const args of stringMethods[name]) {
      same("String.prototype." + name, native, (f, log) => f.apply(strSubjects(log)[i], args));
    }
  }
  same("String.prototype." + name, native, (f) => f.call(null));
  same("String.prototype." + name, native, (f) => f.call(undefined, "x"));
  same("String.prototype." + name, native, () => installed["String.prototype." + name].length === native.length);
}
for (const name of ["includes", "startsWith", "endsWith"]) {
  same("String.prototype." + name, SP[name], (f) => f.call("abc", { [Symbol.match]: true }));
  same("String.prototype." + name, SP[name], (f) => f.call("abc", { [Symbol.match]: false, toString: () => "b" }));
}
for (const form of [undefined, "NFC", "NFD", "NFKC", "NFKD", "bad"]) {
  for (const s of ["plain ascii", "caf\u00e9"]) {
    if (s !== "plain ascii" && form !== undefined && form !== "NFC" && form !== "bad") continue;
    same("String.prototype.normalize", SP.normalize, (f) => f.call(s, form));
  }
}
for (const args of [[], [65, 66.7, -1, 65536 + 67], ["97"], [0xd83d, 0xde00], [toPrim(5)]]) {
  same("String.fromCharCode", String.fromCharCode, (f) => f(...args));
}
for (const args of [[], [65, 0x1f600], [-1], [1.5], [0x110000], ["66"], [NaN], [Infinity]]) {
  same("String.fromCodePoint", String.fromCodePoint, (f) => f(...args));
}
for (const args of [[{ raw: ["a", "b", "c"] }, 1], [{ raw: ["a", "b"] }, 1, 2, 3], [{ raw: [] }], [{ raw: "xyz" }, "-"], [{}], [null]]) {
  same("String.raw", String.raw, (f) => f(...args));
}

// Array Iterators: drain an iterator from values/keys/entries/[Symbol.iterator] by
// calling next() directly, and through for-of (which also uses
// %IteratorPrototype%[Symbol.iterator]). Both are capped: an iterator over a huge
// length must not hang the test.
function drain(it, max = 10) {
  const out = [];
  for (let i = 0; i < max; i++) {
    const r = it.next();
    out.push(show(r));
    if (r.done) break;
  }
  // Exhausted iterators stay done.
  out.push(show(it.next()));
  return out.join(" ");
}
function drainForOf(it, max = 10) {
  const out = [];
  for (const v of it) {
    out.push(show(v));
    if (out.length >= max) break;
  }
  return out.join(",");
}
for (let i = 0; i < nSubjects; i++) {
  // qjs reads an Array Iterator's length with ToUint32, not ToLength: { length: -5 }
  // iterates 2^32 - 5 elements there and none per spec (and here).
  if (i === 6) continue;
  for (const [name, native] of [
    ["Array.prototype.values", AP.values],
    ["Array.prototype.keys", AP.keys],
    ["Array.prototype.entries", AP.entries],
    ["Array.prototype[Symbol.iterator]", AP[Symbol.iterator]],
  ]) {
    same(name, native, (f, log) => drain(f.call(subjects(log)[i])));
    same(name, native, (f, log) => drainForOf(f.call(subjects(log)[i])));
  }
}
for (const name of ["values", "keys", "entries"]) {
  const native = AP[name];
  const full = "Array.prototype." + name;
  same(full, native, (f) => f.call(undefined));
  same(full, native, (f) => f.call(null));
  same(full, native, (f) => drain(f.call(5)));
  same(full, native, () => installed[full].length === native.length);
  // Growth and shrinking while iterating; an exhausted iterator stays exhausted.
  same(full, native, (f) => {
    const a = [1, 2, 3];
    const it = f.call(a);
    const out = [show(it.next())];
    a.push(4);
    out.push(show(it.next()));
    a.length = 1;
    out.push(show(it.next()), show(it.next()));
    a.push(5);
    out.push(show(it.next()));
    return out.join(" ");
  });
}
same(
  "Array.prototype[Symbol.iterator]",
  AP[Symbol.iterator],
  () => installed["Array.prototype[Symbol.iterator]"].f === installed["Array.prototype.values"].f,
);
// next on the wrong receiver.
const nativeAIPNext = Object.getPrototypeOf([].values()).next;
same("%ArrayIteratorPrototype%.next", nativeAIPNext, (f) => f.call({}));
same("%ArrayIteratorPrototype%.next", nativeAIPNext, (f) => f.call(undefined));
same("%ArrayIteratorPrototype%.next", nativeAIPNext, (f) => f.call([].values.call([1]).__proto__));
// %IteratorPrototype%[Symbol.iterator] returns this.
const nativeIterProtoIter = Object.getPrototypeOf(Object.getPrototypeOf([].values()))[Symbol.iterator];
same("%IteratorPrototype%[Symbol.iterator]", nativeIterProtoIter, (f) => {
  const o = {};
  return f.call(o) === o && f.call(5);
});

// ---- 0034: iterables --------------------------------------------------------------------

const userIter = (values, log, opts = {}) => ({
  [Symbol.iterator]() {
    let i = 0;
    log.push("iter");
    return {
      next() {
        log.push("next");
        if (opts.badResult) return 1;
        return i < values.length ? { value: values[i++], done: false } : { value: undefined, done: true };
      },
      return() {
        log.push("return");
        return {};
      },
    };
  },
});
for (const values of [[["a", 1], ["b", 2]], [[1, "x"], [toPrim("k"), 2]], [5], [[Symbol.iterator, 1]], []]) {
  same("Object.fromEntries", Object.fromEntries, (f, log) => f(userIter(values, log)));
  same("Array.from", Array.from, (f, log) => f(userIter(values, log)));
  same("Array.from", Array.from, (f, log) => f(userIter(values, log), (v, k) => [v, k]));
  same("Array.from", Array.from, (f, log) => f.call(Object, userIter(values, log)));
  same("Object.groupBy", Object.groupBy, (f, log) => f(userIter(values, log), (v, k) => (k % 2 ? "odd" : "even")));
  same("globalThis.AggregateError", AggregateError, (f, log) => {
    const e = Reflect.construct(f, [userIter(values, log), "m", { cause: 1 }]);
    return [e.message, e.errors, e.cause, Object.keys(e)];
  });
}
// A non-object from next(): the spec throws without closing; qjs closes the iterator.
same("Object.fromEntries", Object.fromEntries, (f) => f(userIter([], [], { badResult: true })));
same("Array.from", Array.from, (f, log) => f(userIter([1], log), () => { throw new RangeError("m"); }));
same("Object.groupBy", Object.groupBy, (f, log) => f(userIter([1], log), () => { throw new RangeError("cb"); }));
same("Object.groupBy", Object.groupBy, (f, log) => f(userIter([1], log), 5));
same("Object.groupBy", Object.groupBy, (f) => f(null, () => 1));
same("Object.fromEntries", Object.fromEntries, (f) => f(undefined));
same("Object.fromEntries", Object.fromEntries, (f) => f(5));
same("globalThis.AggregateError", AggregateError, (f) => f(5));

// ---- 0038: Reflect ---------------------------------------------------------------------

const reflectTargets = (log) => [{ a: 1, get g() { log.push("getter:" + String(this.tag)); return 2; } }, [1, 2], logged({ p: 1 }, log), Object.freeze({ f: 1 }), function fn(a) { return [this === undefined ? "u" : typeof this, a]; }, 5, null, "s"];
const nReflect = reflectTargets([]).length;
for (let i = 0; i < nReflect; i++) {
  for (const key of ["a", "g", "p", "f", "length", 0, toPrim("a"), Symbol.iterator]) {
    same("Reflect.get", Reflect.get, (f, log) => f(reflectTargets(log)[i], key));
    same("Reflect.get", Reflect.get, (f, log) => f(reflectTargets(log)[i], key, { tag: "recv" }));
    same("Reflect.has", Reflect.has, (f, log) => f(reflectTargets(log)[i], key));
    same("Reflect.set", Reflect.set, (f, log) => { const t = reflectTargets(log)[i]; return [f(t, key, 9), t]; });
    same("Reflect.set", Reflect.set, (f, log) => { const r = {}; return [f(reflectTargets(log)[i], key, 9, r), r]; });
    same("Reflect.deleteProperty", Reflect.deleteProperty, (f, log) => f(reflectTargets(log)[i], key));
    same("Reflect.getOwnPropertyDescriptor", Reflect.getOwnPropertyDescriptor, (f, log) => f(reflectTargets(log)[i], key));
    same("Reflect.defineProperty", Reflect.defineProperty, (f, log) => f(reflectTargets(log)[i], key, { value: 3 }));
  }
  for (const name of ["getPrototypeOf", "isExtensible", "preventExtensions", "ownKeys"]) {
    same("Reflect." + name, Reflect[name], (f, log) => f(reflectTargets(log)[i]));
  }
  same("Reflect.setPrototypeOf", Reflect.setPrototypeOf, (f, log) => f(reflectTargets(log)[i], null));
  same("Reflect.apply", Reflect.apply, (f, log) => f(reflectTargets(log)[i], 1, [2]));
  same("Reflect.apply", Reflect.apply, (f, log) => f(Math.max, null, reflectTargets(log)[i]));
  same("Reflect.construct", Reflect.construct, (f, log) => f(reflectTargets(log)[i], []));
}
// (An arrow as newTarget: qjs says " is not a constructor" with an empty name; djs
// says "not a constructor".)
function RC(a) { this.a = a; this.nt = new.target === RD; }
function RD() {}
for (const [args, nt] of [[[1], undefined], [[2], RD], [{ length: 1, 0: 3 }, RD], [5, RD], [[1], 5]]) {
  same("Reflect.construct", Reflect.construct, (f) => {
    const o = nt === undefined ? f(RC, args) : f(RC, args, nt);
    return [o.a, o.nt, Object.getPrototypeOf(o) === (nt === RD ? RD.prototype : RC.prototype)];
  });
}
for (const C of [Error, TypeError, Array, Object, Boolean, Number, String, Date]) {
  same("Reflect.construct", Reflect.construct, (f) => Object.getPrototypeOf(f(C, [], RD)) === RD.prototype);
}
for (const name of Object.getOwnPropertyNames(Reflect)) {
  same("Reflect." + name, Reflect[name], () => installed["Reflect." + name].length === Reflect[name].length);
}

// ---- 0038: Map, Set, WeakMap, WeakSet ------------------------------------------------------

const entryLists = (log) => [[], [["a", 1], ["b", 2]], [[NaN, 1], [-0, 2], [0, 3]], userIter([["k", "v"]], log), [5], "ab", null, undefined];
for (let i = 0; i < entryLists([]).length; i++) {
  same("globalThis.Map", Map, (f, log) => { const m = Reflect.construct(f, [entryLists(log)[i]]); return [Map.prototype.size.call ? m.size : 0, [...m]]; });
  same("globalThis.Set", Set, (f, log) => { const s = Reflect.construct(f, [entryLists(log)[i]]); return [s.size, [...s]]; });
  same("globalThis.WeakMap", WeakMap, (f, log) => Reflect.construct(f, [entryLists(log)[i]]) instanceof WeakMap);
}
same("globalThis.Map", Map, (f) => f());
const MP = Map.prototype, SP2 = Set.prototype;
for (const name of ["get", "has", "delete"]) {
  for (const key of ["a", "zz", NaN, -0, 0, {}]) {
    same("Map.prototype." + name, MP[name], (f) => { const m = new Map([["a", 1], [NaN, 2], [0, 3]]); return [f.call(m, key), [...m]]; });
  }
  same("Map.prototype." + name, MP[name], (f) => f.call(new Set(), 1));
}
same("Map.prototype.set", MP.set, (f) => { const m = new Map(); return [f.call(m, -0, "z") === m, [...m], Object.is([...m.keys()][0], 0)]; });
same("get Map.prototype.size", Object.getOwnPropertyDescriptor(MP, "size").get, (f) => [f.call(new Map([[1, 1]])), f.call(new Map())]);
same("get Map.prototype.size", Object.getOwnPropertyDescriptor(MP, "size").get, (f) => f.call(new Set()));
same("Map.prototype.forEach", MP.forEach, (f) => {
  const m = new Map([["a", 1], ["b", 2], ["c", 3]]);
  const seen = [];
  f.call(m, function (v, k, mm) { seen.push([k, v, this.t, mm === m]); if (k === "a") { mm.delete("b"); mm.set("d", 4); } }, { t: "T" });
  return seen;
});
same("Map.prototype.forEach", MP.forEach, (f) => f.call(new Map(), 5));
for (const name of ["add", "has", "delete"]) {
  for (const key of [1, "1", NaN, -0, {}]) {
    same("Set.prototype." + name, SP2[name], (f) => { const s = new Set([1, NaN, 0]); return [f.call(s, key) === s ? "self" : "other", [...s]]; });
  }
}
same("Set.prototype.forEach", SP2.forEach, (f) => { const out = []; f.call(new Set([1, 2]), (a, b, s) => out.push(a, b, s.size)); return out; });
const setPairs = [[[1, 2, 3, 4], [3, 4, 5]], [[], [1]], [[1, 2], []], [[1, 2, 3], [1, 2, 3]], [["a", NaN], [NaN]]];
for (const [x, y] of setPairs) {
  for (const name of ["union", "intersection", "difference", "symmetricDifference", "isSubsetOf", "isSupersetOf", "isDisjointFrom"]) {
    same("Set.prototype." + name, SP2[name], (f) => { const r = f.call(new Set(x), new Set(y)); return r instanceof Set ? [...r] : r; });
    same("Set.prototype." + name, SP2[name], (f, log) => {
      const other = { size: y.length, has: (v) => { log.push("has:" + String(v)); return y.includes(v); }, keys: () => { log.push("keys"); return userIter(y, log)[Symbol.iterator](); } };
      const r = f.call(new Set(x), other);
      return r instanceof Set ? [...r] : r;
    });
  }
}
for (const bad of [{ size: NaN }, { size: -1, has() {}, keys() {} }, { size: 1, has: 1, keys() {} }, { size: 1, has() {}, keys: 1 }, { size: Infinity, has: () => true, keys: () => [][Symbol.iterator]() }]) {
  for (const name of ["union", "isSubsetOf"]) {
    same("Set.prototype." + name, SP2[name], (f) => { const r = f.call(new Set([1]), bad); return r instanceof Set ? [...r] : r; });
  }
}
same("Map.groupBy", Map.groupBy, (f) => [...f([1, 2, 3, 4, -0, 0], (v) => (v % 2 ? "odd" : v))]);
same("Map.groupBy", Map.groupBy, (f, log) => [...f(userIter([1], log), () => { throw new RangeError("cb"); })]);
same("Map.groupBy", Map.groupBy, (f) => f([1], 5));
const WMP = WeakMap.prototype, WSP = WeakSet.prototype;
for (const key of [{}, Symbol("s"), Symbol.for("reg"), Symbol.iterator, 1, "s", null]) {
  same("WeakMap.prototype.set", WMP.set, (f) => { const m = new WeakMap(); return [f.call(m, key, 1) === m, m.get(key)]; });
  same("WeakMap.prototype.get", WMP.get, (f) => f.call(new WeakMap(), key));
  same("WeakMap.prototype.has", WMP.has, (f) => f.call(new WeakMap(), key));
  same("WeakMap.prototype.delete", WMP.delete, (f) => f.call(new WeakMap(), key));
  same("WeakSet.prototype.add", WSP.add, (f) => { const s = new WeakSet(); return [f.call(s, key) === s, s.has(key)]; });
  same("WeakSet.prototype.has", WSP.has, (f) => f.call(new WeakSet(), key));
  same("WeakSet.prototype.delete", WSP.delete, (f) => f.call(new WeakSet(), key));
}
same("WeakMap.prototype.get", WMP.get, (f) => f.call(new Map(), {}));

// ---- 0038: Date ---------------------------------------------------------------------------

const DP = Date.prototype;
const dateArgs = [[0], [1e12], [NaN], [8.64e15], [8.64e15 + 1], ["2020-02-29T12:34:56.789Z"], ["Feb 29 2020"], ["nope"], [true], [toPrim(5)], [{ valueOf: () => 9 }], [new Date(77)],
  [2020, 1], [2020, 1, 29, 12, 34, 56, 789], [99, 0], [2020, 13, 40], [2020, NaN], [2020, 0, 1, 0, 0, 0, 0, 99], [-1, 0], [275760, 8, 13], [Infinity, 0]];
for (const args of dateArgs) {
  same("globalThis.Date", Date, (f) => DP.getTime.call(Reflect.construct(f, args)));
  same("Date.UTC", Date.UTC, (f) => f(...args.map((a) => (a instanceof Date ? 1 : a))));
}
same("Date.UTC", Date.UTC, (f) => f());
for (const t of [0, 1e12, -1e12, NaN, 8.64e15, -8.64e15, 951782400000, -62198755200000]) {
  for (const name of Object.getOwnPropertyNames(DP).filter((n) => /^(get|to)/.test(n) && n !== "toJSON" && n !== "getYear" && !/Locale/.test(n))) {
    same("Date.prototype." + name, DP[name], (f) => f.call(new Date(t)));
  }
  same("Date.prototype.toJSON", DP.toJSON, (f) => f.call(new Date(t)));
  for (const name of Object.getOwnPropertyNames(DP).filter((n) => /^set/.test(n) && n !== "setYear")) {
    for (const args of [[], [5], [1, 2], [1, 2, 3], [-1, 40, 70, 1000], [NaN], [Infinity, 1], [toPrim(3)], ["7"]]) {
      same("Date.prototype." + name, DP[name], (f) => { const d = new Date(t); return [f.call(d, ...args), d.getTime()]; });
    }
  }
}
for (const name of ["getTime", "setTime", "toISOString", "setMonth"]) {
  same("Date.prototype." + name, DP[name], (f) => f.call({}));
}
same("Date.prototype.setMonth", DP.setMonth, (f, log) => f.call(new Date(NaN), valueOfObj(1, log), valueOfObj(2, log)));
for (const s of ["2020-02-29T12:34:56.789Z", "2020-02-29", "2020-02-29T12:34", "2020-13-01", "Sat Feb 29 2020 12:34:56 GMT+0000", "Sat, 29 Feb 2020 12:34:56 GMT", "Feb 29 2020", "29 February 2020 10:00 PM EST", "2020/2/29 1:2:3", "12/31/1999 23:59", "+002020-02-29T00:00:00Z", "-000000-01-01T00:00:00Z", "T", "", "Thu Jan 01 1970 00:00:00 GMT+0100 (CET)", "2020-02-29T24:00", "2020-02-29T24:00:01", "1 Mar 2022 10:00 +0100", 5, toPrim("2000")]) {
  same("Date.parse", Date.parse, (f) => f(s));
}
for (const hint of ["string", "number", "default", "bogus", 1]) {
  same("Date.prototype[Symbol.toPrimitive]", DP[Symbol.toPrimitive], (f) => f.call(new Date(0), hint));
}
same("Date.prototype[Symbol.toPrimitive]", DP[Symbol.toPrimitive], (f) => f.call(1, "number"));
same("Date.prototype.toJSON", DP.toJSON, (f) => f.call({ valueOf: () => Infinity, toISOString: () => "x" }));
same("Date.prototype.toJSON", DP.toJSON, (f) => f.call({ valueOf: () => 1, toISOString: () => "iso" }));
for (const name of Object.getOwnPropertyNames(DP).filter((n) => typeof DP[n] === "function" && n !== "constructor" && installed["Date.prototype." + n])) {
  same("Date.prototype." + name, DP[name], () => installed["Date.prototype." + name].length === DP[name].length);
}

// ---- 0039: ArrayBuffer, typed arrays, base64/hex ----------------------------------------

for (const args of [[8], [0], [-1], [2 ** 53], ["4"], [4, { maxByteLength: 8 }], [4, { maxByteLength: 2 }], [undefined], [toPrim(3)]]) {
  same("globalThis.ArrayBuffer", ArrayBuffer, (f) => { const b = Reflect.construct(f, args); return [b.byteLength, b.maxByteLength, b.resizable]; });
}
same("globalThis.ArrayBuffer", ArrayBuffer, (f) => f(1));
const ABP = ArrayBuffer.prototype;
for (const [s0, e0] of [[undefined, undefined], [1, 3], [-2, undefined], [5, 1], [0, 100]]) {
  same("ArrayBuffer.prototype.slice", ABP.slice, (f) => { const b = new Uint8Array([1, 2, 3, 4]).buffer; return Array.from(new Uint8Array(f.call(b, s0, e0))); });
}
same("ArrayBuffer.prototype.slice", ABP.slice, (f) => f.call({}));
for (const n of [undefined, 2, 8, -1]) {
  same("ArrayBuffer.prototype.transfer", ABP.transfer, (f) => { const b = new Uint8Array([1, 2, 3, 4]).buffer; const r = f.call(b, n); return [b.detached, r.byteLength, Array.from(new Uint8Array(r))]; });
  same("ArrayBuffer.prototype.resize", ABP.resize, (f) => { const b = new ArrayBuffer(4, { maxByteLength: 6 }); f.call(b, n); return b.byteLength; });
}
same("ArrayBuffer.prototype.resize", ABP.resize, (f) => f.call(new ArrayBuffer(4), 2));
const DVC = installed["globalThis.DataView"].f;
for (const args of [[new ArrayBuffer(8)], [new ArrayBuffer(8), 2], [new ArrayBuffer(8), 2, 4], [new ArrayBuffer(8), 9], [new ArrayBuffer(8), 4, 5], [new ArrayBuffer(8, { maxByteLength: 16 }), 4], [{}], [new ArrayBuffer(8), -1]]) {
  same("globalThis.DataView", DataView, (f) => { const v = Reflect.construct(f, args); return [v.byteOffset, v.byteLength]; });
}
same("globalThis.DataView", DataView, (f) => f(new ArrayBuffer(1)));
for (const name of ["Int8", "Uint8", "Int16", "Uint16", "Int32", "Uint32", "Float16", "Float32", "Float64"]) {
  for (const [at, little] of [[0, false], [1, true], [4, undefined], [7, true], [-1, false], [1.5, 1], ["2", 0]]) {
    same("DataView.prototype.get" + name, DataView.prototype["get" + name], (f) => {
      const v = Reflect.construct(DVC, [new Uint8Array([1, 0x80, 0xff, 3, 0x3c, 5, 0x7f, 0xfe]).buffer]);
      return f.call(v, at, little);
    });
    same("DataView.prototype.set" + name, DataView.prototype["set" + name], (f) => {
      const v = Reflect.construct(DVC, [new ArrayBuffer(8), 1]);
      f.call(v, at, -1.5e3, little);
      return Array.from(new Uint8Array(v.buffer));
    });
  }
  same("DataView.prototype.get" + name, DataView.prototype["get" + name], (f) => f.call(new Uint8Array(8), 0));
}
same("ArrayBuffer.isView", ArrayBuffer.isView, (f) => [f(new Uint8Array(1)), f(new DataView(new ArrayBuffer(1))), f(new ArrayBuffer(1)), f([])]);
for (let k = 0; k < 10; k++) {
  const name = TA_CTORS[k].name;
  for (const args of [[3], [[1, -1, 2.5, 300, NaN, Infinity]], [new Float64Array([1.5, -2.5, 70000])], [{ length: 2, 0: "7", 1: toPrim(5) }], [new ArrayBuffer(16), 8], [new ArrayBuffer(16), 0, 1], [new ArrayBuffer(16), 1], [new ArrayBuffer(15)], [-1], [new Set([5, 6])]]) {
    same("globalThis." + name, TA_CTORS[k], (f) => { const t = Reflect.construct(f, args); return [t.length, t.byteOffset, Array.from(t)]; });
  }
  same("globalThis." + name, TA_CTORS[k], (f) => f(1));
}
for (let k = 10; k < 12; k++) {
  const name = TA_CTORS[k].name;
  for (const args of [[3], [[1n, -1n, 2n ** 64n + 3n]], [new BigInt64Array([5n, -6n])], [{ length: 2, 0: "7", 1: true }], [[1]], [[undefined]], [new Int8Array(1)], [new ArrayBuffer(16), 8], [-1], [new Set([5n])]]) {
    same("globalThis." + name, TA_CTORS[k], (f) => { const t = Reflect.construct(f, args); return [t.length, t.byteOffset, Array.from(t)]; });
  }
}
const bigTaCases = [
  ["fill", [[7n], [7], ["8", 1], [toPrim("9")]]],
  ["set", [[[1n, 2n]], [[1]], [new Int8Array(1)], [new BigUint64Array([2n ** 64n - 1n]), 1]]],
  ["includes", [[3n], [3], [9n]]],
  ["indexOf", [[1n], [1], [1n, 2]]],
  ["join", [[], ["-"]]],
  ["map", [[(v) => v * 2n], [(v) => 1]]],
  ["sort", [[], [(a, b) => (a < b ? 1 : -1)]]],
  ["toSorted", [[]]],
  ["with", [[0, 9n], [0, 9], [9, 1n]]],
  ["slice", [[1], [-2, -1]]],
  ["reverse", [[]]],
  ["toLocaleString", [[]]],
];
for (const [name, argss] of bigTaCases) {
  for (const args of argss) {
    same("%TypedArray%.prototype." + name, TA_PROTO[name], (f) => {
      const t = new BigInt64Array([3n, -1n, 4n, 1n, -(2n ** 63n)]);
      const r = f.call(t, ...args);
      return [ArrayBuffer.isView(r) ? Array.from(r) : r, Array.from(t)];
    });
  }
}
const TAP = TA_PROTO;
for (const name of ["set", "subarray", "slice", "fill"]) {
  for (const args of [[], [[9, 8]], [[9, 8], 2], [new Uint8Array([7])], [1, 3], [-2], [5, 1], [[1, 2, 3, 4, 5]], [0, 1, 2]]) {
    same("%TypedArray%.prototype." + name, TAP[name], (f) => { const t = new Uint8Array([1, 2, 3, 4]); const r = f.call(t, ...args); return [r === undefined ? "u" : Array.from(r), Array.from(t)]; });
  }
  same("%TypedArray%.prototype." + name, TAP[name], (f) => f.call([1, 2]));
}
const taCases = [
  ["at", [[0], [-1], [9], ["1"]]],
  ["copyWithin", [[0, 2], [1, 0, 2], [-1, 0], [0, 1, -1]]],
  ["every", [[(v) => v > 0], [(v) => v > 1]]],
  ["some", [[(v) => v > 3], [(v, i, a) => a !== undefined && i === 9]]],
  ["filter", [[(v) => v % 2], [() => false]]],
  ["find", [[(v) => v > 1], [(v) => v > 9]]],
  ["findIndex", [[(v) => v > 1], [(v) => v > 9]]],
  ["findLast", [[(v) => v < 4], [(v) => v > 9]]],
  ["findLastIndex", [[(v) => v < 4], [(v) => v > 9]]],
  ["includes", [[3], [3, 3], [9], [1, -10]]],
  ["indexOf", [[3], [3, 3], [9], [1, -10], ["3"]]],
  ["lastIndexOf", [[3], [3, 1], [9], [1, -10], [4, -1]]],
  ["join", [[], ["-"], [undefined]]],
  ["map", [[(v, i) => v * i], [(v) => v + 0.5]]],
  ["reduce", [[(a, b) => a + b], [(a, b) => a + "," + b, "s"]]],
  ["reduceRight", [[(a, b) => a + b], [(a, b) => a + "," + b, "s"]]],
  ["reverse", [[]]],
  ["sort", [[], [(a, b) => b - a]]],
  ["toSorted", [[], [(a, b) => b - a]]],
  ["toReversed", [[]]],
  ["with", [[0, 9], [-1, 7], [9, 1]]],
  ["toLocaleString", [[]]],
];
for (const [name, argss] of taCases) {
  for (const args of argss) {
    same("%TypedArray%.prototype." + name, TAP[name], (f) => {
      const t = new Int16Array([3, 1, 4, 1, 5]);
      const r = f.call(t, ...args);
      return [ArrayBuffer.isView(r) ? Array.from(r) : r, Array.from(t)];
    });
  }
  same("%TypedArray%.prototype." + name, TAP[name], (f) => f.call([1, 2], () => 0));
}
for (const t of [new Float64Array([3, -0, 0, NaN, -Infinity, 1]), new Uint8Array([]), new Float32Array([0.5])]) {
  same("%TypedArray%.prototype.sort", TAP.sort, (f) => Array.from(f.call(t.slice())).map((v) => Object.is(v, -0) ? "-0" : v));
  same("%TypedArray%.prototype.includes", TAP.includes, (f) => [f.call(t, NaN), f.call(t, 0)]);
  same("%TypedArray%.prototype.indexOf", TAP.indexOf, (f) => [f.call(t, NaN), f.call(t, 0)]);
}
for (const name of ["entries", "keys", "values"]) {
  same("%TypedArray%.prototype." + name, TAP[name], (f) => [...f.call(new Uint8Array([7, 8]))]);
  same("%TypedArray%.prototype." + name, TAP[name], (f) => f.call([1]));
}
same("%TypedArray%.from", Object.getPrototypeOf(Int8Array).from, (f) => Array.from(f.call(Int16Array, [1, 2, 70000], (v) => v * 2)));
same("%TypedArray%.of", Object.getPrototypeOf(Int8Array).of, (f) => Array.from(f.call(Uint8Array, 1, 2, 300)));
const blob = new Uint8Array(256).map((_, i) => i);
for (const opts of [undefined, { alphabet: "base64url" }, { omitPadding: true }, { alphabet: "nope" }, 5]) {
  for (const t of [blob, new Uint8Array([]), new Uint8Array([1]), new Uint8Array([1, 2]), new Uint8Array([1, 2, 3])]) {
    same("Uint8Array.prototype.toBase64", Uint8Array.prototype.toBase64, (f) => f.call(t, opts));
  }
}
same("Uint8Array.prototype.toHex", Uint8Array.prototype.toHex, (f) => f.call(blob));
same("Uint8Array.prototype.toHex", Uint8Array.prototype.toHex, (f) => f.call(new Int8Array(1)));
for (const s of ["", "aGk=", "aGk", "aG", "a", "aGkh", " a G k h ", "aGk=x", "aG==", "aGk==", "-_-_", "+/+/", "AA==", "AB==", "AAA=", "AAB="]) {
  for (const opts of [undefined, { alphabet: "base64url" }, { lastChunkHandling: "strict" }, { lastChunkHandling: "stop-before-partial" }, { lastChunkHandling: "bogus" }]) {
    same("Uint8Array.fromBase64", Uint8Array.fromBase64, (f) => Array.from(f(s, opts)));
  }
}
same("Uint8Array.fromBase64", Uint8Array.fromBase64, (f) => f(5));
for (const s of ["", "00ff", "0", "zz", "ABcd", "a b"]) {
  same("Uint8Array.fromHex", Uint8Array.fromHex, (f) => Array.from(f(s)));
}
same("Uint8Array.fromBase64", Uint8Array.fromBase64, (f) => Array.from(f(blob.toBase64())).join() === Array.from(blob).join());

// ---- 0045: Proxy ----------------------------------------------------------------------
// The same operations on a native proxy and on ours (installed builtins behind a native
// shell), with handlers that log every trap: the trap order and the invariant checks.

sameSetFailures = true;
const TRAPS = ["get", "set", "has", "deleteProperty", "ownKeys", "getOwnPropertyDescriptor", "defineProperty", "getPrototypeOf", "setPrototypeOf", "isExtensible", "preventExtensions", "apply", "construct"];
const showKey = (k) => (typeof k === "symbol" ? k.toString() : String(k));
// A handler that logs each trap and forwards it (or returns `override[trap]`'s result).
function loggingHandler(log, override = {}) {
  const h = {};
  for (const trap of TRAPS) {
    h[trap] = function (t, ...rest) {
      log.push(trap + (typeof rest[0] === "string" || typeof rest[0] === "symbol" ? ":" + showKey(rest[0]) : ""));
      return trap in override ? override[trap](t, ...rest) : Reflect[trap](t, ...rest);
    };
  }
  return h;
}
const frozenTarget = () => Object.freeze({ fixed: 1 });
const proxyTargets = () => [
  { a: 1, b: 2, [Symbol.iterator]: 3 },
  [1, 2, 3],
  function f(x) { return [typeof this, x]; },
  frozenTarget(),
  Object.preventExtensions({ n: 1 }),
  Object.defineProperty({}, "ro", { value: 5, configurable: false, writable: false }),
];
const proxyOps = [
  (p) => p.a,
  (p) => p.fixed,
  (p) => p[0],
  (p) => { p.a = 9; return p.a; },
  (p) => { p.ro = 6; return p.ro; },
  (p) => "a" in p,
  (p) => "zz" in p,
  (p) => delete p.a,
  (p) => delete p.fixed,
  (p) => Object.keys(p),
  (p) => Object.values(p),
  (p) => Object.entries(p),
  (p) => Reflect.ownKeys(p),
  (p) => Object.getOwnPropertyNames(p),
  (p) => Object.getOwnPropertySymbols(p),
  (p) => Object.getOwnPropertyDescriptor(p, "a"),
  (p) => Object.getOwnPropertyDescriptors(p),
  (p) => Object.defineProperty(p, "x", { value: 1, configurable: true }),
  (p) => Reflect.defineProperty(p, "y", { get() {}, enumerable: true }),
  (p) => Object.getPrototypeOf(p) === Object.prototype,
  (p) => Object.setPrototypeOf(p, null),
  (p) => Object.isExtensible(p),
  (p) => Object.preventExtensions(p),
  (p) => Object.freeze(p),
  (p) => Object.seal(p),
  (p) => Object.isFrozen(p),
  (p) => Object.isSealed(p),
  (p) => Object.assign({}, p),
  (p) => Object.prototype.hasOwnProperty.call(p, "a"),
  (p) => Object.prototype.propertyIsEnumerable.call(p, "b"),
  (p) => Object.prototype.toString.call(p),
  (p) => Array.isArray(p),
  (p) => JSON.stringify(p),
  (p) => Array.prototype.slice.call(p),
  (p) => Array.prototype.concat.call([], p),
  (p) => p(1),
  (p) => Reflect.apply(p, 1, [2]),
  (p) => new p(3) instanceof Object,
  (p) => Function.prototype.bind.call(p, null).name,
];
for (let i = 0; i < proxyTargets().length; i++) {
  for (const op of proxyOps) {
    same("globalThis.Proxy", Proxy, (P, log) => op(new P(proxyTargets()[i], loggingHandler(log))));
    same("globalThis.Proxy", Proxy, (P, log) => op(new P(proxyTargets()[i], {})));
  }
}
// Traps that break the invariants (each in turn): the checks and their order.
const lies = {
  get: () => 42,
  set: () => true,
  has: () => false,
  deleteProperty: () => true,
  ownKeys: () => ["a", "a"],
  getOwnPropertyDescriptor: () => ({ value: 7, configurable: false }),
  defineProperty: () => true,
  getPrototypeOf: () => Array.prototype,
  setPrototypeOf: () => true,
  isExtensible: () => true,
  preventExtensions: () => true,
  apply: () => 1,
  construct: () => 1,
};
const otherLies = { ownKeys: [() => [1], () => [], () => ["fixed", "extra"], () => 5], getOwnPropertyDescriptor: [() => undefined, () => 5, () => ({ value: 1, configurable: true })], getPrototypeOf: [() => 5] };
for (let i = 0; i < proxyTargets().length; i++) {
  for (const trap of TRAPS) {
    for (const lie of [lies[trap], ...(otherLies[trap] || [])]) {
      for (const op of proxyOps) {
        same("globalThis.Proxy", Proxy, (P, log) => op(new P(proxyTargets()[i], loggingHandler(log, { [trap]: lie }))));
      }
    }
  }
}
same("globalThis.Proxy", Proxy, (P) => P({}, {}));
same("globalThis.Proxy", Proxy, (P) => new P(1, {}));
same("globalThis.Proxy", Proxy, (P) => new P({}, null));
// (Proxies of proxies are in tests/corpus/builtins/proxy.js: behind native shells, the
// shells' own invariant checks would run the inner traps once more.)
same("globalThis.Proxy", Proxy, (P, log) => Object.create(new P({ a: 1 }, loggingHandler(log))).a);
same("globalThis.Proxy", Proxy, (P, log) => { const o = Object.create(new P({}, loggingHandler(log))); o.q = 1; return Object.keys(o); });
same("globalThis.Proxy", Proxy, (P, log) => "q" in Object.create(new P({}, loggingHandler(log))));
// A handler that is itself a proxy: every trap lookup is a `get`.
same("globalThis.Proxy", Proxy, (P, log) => { const p = new P({ a: 1 }, new P({}, loggingHandler(log))); return [p.a, "a" in p, Object.keys(p)]; });
same("Proxy.revocable", Proxy.revocable, (f, log) => {
  const r = f({ a: 1 }, loggingHandler(log));
  const before = r.proxy.a;
  r.revoke();
  r.revoke();
  return [before, Object.keys(r), r.revoke.name, r.revoke.length];
});
// (A shell is not revoked itself: its natively answered IsArray and IsCallable are left
// out.)
for (const op of proxyOps.filter((op) => !/isArray|p\(1\)|apply|new p/.test(String(op)))) {
  same("Proxy.revocable", Proxy.revocable, (f) => {
    const r = f([1], {});
    r.revoke();
    return op(r.proxy);
  });
}

sameSetFailures = false;

// Iterator helpers (0051): the results and the trap order on a logging iterator, drained
// through the helper's own `next` (native or ours) and closed through `return`.
function logIter(n, log) {
  let i = 0;
  return Object.create(Iterator.prototype, Object.getOwnPropertyDescriptors({
    get next() {
      log.push("get next");
      return () => {
        log.push("next");
        return i < n ? { value: i++, done: false } : { value: "end", done: true };
      };
    },
    get return() {
      log.push("get return");
      return () => {
        log.push("return");
        return {};
      };
    },
  }));
}
function drain(h, max = 10) {
  const out = [];
  for (let k = 0; k < max; k++) {
    const r = h.next();
    out.push(r.done ? "done" : r.value);
    if (r.done) break;
  }
  return out;
}
const IP = Iterator.prototype;
const helperCases = [
  ["map", (f, log) => drain(f.call(logIter(3, log), (x, i) => x * 10 + i))],
  ["map", (f, log) => drain(f.call(logIter(3, log), (x) => { if (x === 1) throw new RangeError("m"); return x; }))],
  ["map", (f, log) => f.call(logIter(3, log), 1)],
  ["map", (f, log) => f.call(1, () => 0)],
  ["map", (f, log) => { const h = f.call(logIter(3, log), (x) => x); h.next(); return [h.return(), h.next(), h.return()]; }],
  ["map", (f, log) => { const h = f.call(logIter(3, log), (x) => x); return [h.return(), h.next()]; }],
  ["filter", (f, log) => drain(f.call(logIter(6, log), (x) => x % 2))],
  ["take", (f, log) => drain(f.call(logIter(6, log), 2))],
  ["take", (f, log) => drain(f.call(logIter(2, log), Infinity))],
  ["take", (f, log) => f.call(logIter(6, log), valueOfObj(-1, log))],
  ["take", (f, log) => f.call(logIter(6, log), NaN)],
  ["drop", (f, log) => drain(f.call(logIter(4, log), 2))],
  ["drop", (f, log) => drain(f.call(logIter(4, log), 9))],
  ["drop", (f, log) => f.call(logIter(4, log), 2 ** 53)],
  ["flatMap", (f, log) => drain(f.call(logIter(3, log), (x) => [x, -x]))],
// (qjs closes an exhausted inner iterator of flatMap and closes the iterator before
// reduce's "empty iterator" TypeError; the spec does neither, and djs follows it and
// node: vm/tests.rs iterator_helpers_follow_the_spec_where_qjs_differs.)
  ["flatMap", (f, log) => drain(f.call(logIter(3, log), (x) => "s"))],
  ["flatMap", (f, log) => { const h = f.call(logIter(3, log), (x) => logIter(2, log)); h.next(); h.next(); return h.return(); }],
  ["reduce", (f, log) => f.call(logIter(4, log), (a, b, i) => a + b * i)],
  ["reduce", (f, log) => f.call(logIter(4, log), (a, b) => a + b, "s")],
  ["reduce", (f, log) => f.call(logIter(3, log), () => { throw new Error("r"); }, 0)],
  ["toArray", (f, log) => f.call(logIter(3, log))],
  ["forEach", (f, log) => f.call(logIter(3, log), (x, i) => log.push("fe " + x + i))],
  ["some", (f, log) => f.call(logIter(3, log), (x) => x === 1)],
  ["some", (f, log) => f.call(logIter(3, log), (x) => false)],
  ["every", (f, log) => f.call(logIter(3, log), (x) => x < 1)],
  ["every", (f, log) => f.call(logIter(3, log), (x) => true)],
  ["find", (f, log) => f.call(logIter(3, log), (x) => x === 2)],
  ["find", (f, log) => f.call(logIter(3, log), "nope")],
  ["chunks", (f, log) => drain(f.call(logIter(5, log), 2))],
  ["chunks", (f, log) => f.call(logIter(5, log), "2")],
  ["chunks", (f, log) => f.call(logIter(5, log), 2 ** 32)],
  ["chunks", (f, log) => { const h = f.call(logIter(1, log), 3); h.next(); return h.return(); }],
  ["windows", (f, log) => drain(f.call(logIter(4, log), 2))],
  ["windows", (f, log) => drain(f.call(logIter(2, log), 3, "allow-partial"))],
  ["windows", (f, log) => drain(f.call(logIter(2, log), 3, "only-full"))],
  ["windows", (f, log) => f.call(logIter(2, log), 3, "partial")],
  ["windows", (f, log) => { const h = f.call(logIter(4, log), 2); h.next(); return h.return(); }],
  ["includes", (f, log) => f.call(logIter(5, log), 2)],
  ["includes", (f, log) => f.call(logIter(5, log), 2, 3)],
  ["includes", (f, log) => f.call(logIter(5, log), 7, Infinity)],
  ["includes", (f, log) => f.call(logIter(5, log), 2, -1)],
  ["includes", (f, log) => f.call(logIter(5, log), 2, NaN)],
  ["includes", (f, log) => f.call(logIter(5, log), 2, "1")],
  ["join", (f, log) => f.call(logIter(3, log))],
  ["join", (f, log) => f.call(logIter(3, log), valueOfObj(1, log))],
  ["join", (f, log) => f.call(logIter(3, log), { toString() { throw new Error("j"); } })],
];
for (const [m, run] of helperCases) same("Iterator.prototype." + m, IP[m], run);
same("Iterator.from", Iterator.from, (f, log) => drain(f({ next() { log.push("next"); return { value: 1, done: false }; } }), 2));
same("Iterator.from", Iterator.from, (f, log) => { const w = f({ next() { return {}; } }); return [w.return(), w instanceof Iterator]; });
same("Iterator.from", Iterator.from, (f, log) => drain(f("ab")));
same("Iterator.from", Iterator.from, (f, log) => { const it = logIter(1, log); return f(it) === it; });
same("Iterator.from", Iterator.from, (f) => f(5));
same("Iterator.concat", Iterator.concat, (f, log) => drain(f([1, 2], { [Symbol.iterator]: () => logIter(2, log) })));
same("Iterator.concat", Iterator.concat, (f, log) => { const h = f({ [Symbol.iterator]: () => logIter(2, log) }); h.next(); return h.return(); });
same("Iterator.concat", Iterator.concat, (f) => f({}));
for (const mode of [undefined, "shortest", "longest", "strict"]) {
  same("Iterator.zip", Iterator.zip, (f, log) => drain(f([logIter(2, log), logIter(3, log)], { mode })));
  same("Iterator.zip", Iterator.zip, (f, log) => drain(f([logIter(2, log), logIter(2, log)], { mode, padding: ["a"] })));
  same("Iterator.zipKeyed", Iterator.zipKeyed, (f, log) => drain(f({ x: logIter(2, log), y: logIter(1, log), z: undefined }, { mode, padding: { y: "p" } })));
}
same("Iterator.zip", Iterator.zip, (f, log) => { const h = f([logIter(2, log), logIter(2, log)]); h.next(); return h.return(); });
same("Iterator.zip", Iterator.zip, (f, log) => f([logIter(2, log), 1]));
same("Iterator.zip", Iterator.zip, (f) => f([], { mode: 1 }));
same("Iterator.zip", Iterator.zip, (f) => f([], { mode: "longest", padding: 1 }));
same("Iterator.zipKeyed", Iterator.zipKeyed, (f) => f(1));

// upsert (0051).
for (const [name, native, mk, key] of [
  ["Map.prototype", Map.prototype, () => new Map([[1, "a"]]), 1],
  ["WeakMap.prototype", WeakMap.prototype, () => new WeakMap(), {}],
]) {
  same(name + ".getOrInsert", native.getOrInsert, (f) => { const m = mk(); return [f.call(m, key, "x"), f.call(m, key, "y")]; });
  same(name + ".getOrInsert", native.getOrInsert, (f) => f.call({}, key, 1));
  same(name + ".getOrInsert", native.getOrInsert, (f) => f.call(mk(), 1n, 1));
  same(name + ".getOrInsertComputed", native.getOrInsertComputed, (f, log) => { const m = mk(); return [f.call(m, key, (k) => { log.push("cb"); return "c"; }), f.call(m, key, () => "d")]; });
  same(name + ".getOrInsertComputed", native.getOrInsertComputed, (f) => { const m = mk(); return f.call(m, key, (k) => { m.set(k, "in"); return "out"; }); });
  same(name + ".getOrInsertComputed", native.getOrInsertComputed, (f) => f.call(mk(), 2, 1));
  same(name + ".getOrInsertComputed", native.getOrInsertComputed, (f) => f.call(mk(), -0, (k) => Object.is(k, -0)));
}

// DisposableStack (0051): each case builds its stack with the native constructor or
// with ours, and drives it with the methods of the same side.
const DSP = DisposableStack.prototype;
function dsKit(f, native) {
  if (f === native) {
    return {
      make: () => new DisposableStack(),
      m: (name) => (name === "disposed" ? Object.getOwnPropertyDescriptor(DSP, name).get : DSP[name]),
    };
  }
  return {
    make: () => I.DisposableStackCreate(DSP, false),
    m: (name) => installed[name === "disposed" ? "get DisposableStack.prototype.disposed" : "DisposableStack.prototype." + name].f,
  };
}
const res = (log, name, fail) => ({ [Symbol.dispose]() { log.push("dispose " + name); if (fail) throw new Error(name); } });
const showErr = (e) => (e instanceof SuppressedError ? "Suppressed(" + showErr(e.error) + ", " + showErr(e.suppressed) + ")" : String(e));
const dsCases = [
  ["use", (k, log) => { const st = k.make(); k.m("use").call(st, res(log, "a")); k.m("use").call(st, null); k.m("use").call(st, res(log, "b")); k.m("dispose").call(st); return k.m("use").call(st, {}); }],
  ["use", (k) => k.m("use").call(k.make(), 1)],
  ["use", (k) => k.m("use").call(k.make(), {})],
  ["use", (k) => k.m("use").call(k.make(), { [Symbol.dispose]: 1 })],
  ["use", (k) => k.m("use").call({}, null)],
  ["adopt", (k, log) => { const st = k.make(); k.m("adopt").call(st, 7, function (v) { log.push("adopted " + v + " " + arguments.length + " " + this); }); k.m("dispose").call(st); return k.m("disposed").call(st); }],
  ["adopt", (k) => k.m("adopt").call(k.make(), 7, 1)],
  ["defer", (k, log) => { const st = k.make(); k.m("defer").call(st, function () { log.push("deferred " + this); }); return [k.m("defer").call(st, () => 1), k.m("dispose").call(st)]; }],
  ["dispose", (k, log) => { const st = k.make(); k.m("use").call(st, res(log, "a", true)); k.m("use").call(st, res(log, "b")); k.m("use").call(st, res(log, "c", true)); try { k.m("dispose").call(st); } catch (e) { return showErr(e); } }],
  ["dispose", (k, log) => { const st = k.make(); k.m("defer").call(st, () => { throw 1; }); k.m("defer").call(st, () => { throw 2; }); k.m("defer").call(st, () => { throw 3; }); try { k.m("dispose").call(st); } catch (e) { return showErr(e); } }],
  ["move", (k, log) => { const st = k.make(); k.m("use").call(st, res(log, "a")); const moved = k.m("move").call(st); k.m("dispose").call(st); log.push("old disposed"); k.m("dispose").call(moved); return [k.m("disposed").call(st), k.m("disposed").call(moved)]; }],
  ["move", (k) => { const st = k.make(); k.m("dispose").call(st); return k.m("move").call(st); }],
];
for (const [name, run] of dsCases) {
  const native = DSP[name];
  same("DisposableStack.prototype." + name, native, (f, log) => {
    return run(dsKit(f, native), log);
  });
}

console.log("ok " + count + (failures ? " (" + failures + " mismatches)" : ""));
