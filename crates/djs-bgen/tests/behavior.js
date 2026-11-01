// Runs the generated builtins (builtins/gen/prelude.js, with %Name(...) rewritten to
// I.Name(...)) against qjs's native implementations. Prints one line per mismatch and
// "ok N" at the end. Used by tests/behavior.rs; PRELUDE is prepended by the test.
"use strict";

const installed = {};
const isObject = (v) => v !== null && (typeof v === "object" || typeof v === "function");
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
  IsArray: (v) => Array.isArray(v),
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
  NewNotConstructorError: (f) => new TypeError(f.name + " is not a constructor"),
  WellKnownSymbol: (name) => Symbol[name],
  InstallMethod: (target, key, f, length) => {
    installed[target + "." + key] = { f, length };
  },
  InstallIntrinsic: (name, f) => {
    installed["%" + name] = { f };
  },
  // The constructor gets the native one's prototype, so instances compare equal.
  InstallConstructor: (target, key, f, length, parent) => {
    Object.defineProperty(f, "prototype", { value: globalThis[key].prototype });
    installed[target + "." + key] = { f, length };
  },
  InstallAccessor: (target, key, f, isSetter) => {
    installed[(isSetter ? "set " : "get ") + target + "." + key] = { f };
  },
  InstallValue: (target, key, v, attrs) => {
    installed[target + "." + key] = { v, attrs };
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
};

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

function outcome(fn) {
  const log = [];
  let result;
  try {
    result = "return " + show(fn(log));
  } catch (e) {
    result = "throw " + (isObject(e) ? e.constructor.name + ": " + e.message : show(e));
  }
  return log.join(" ") + " => " + result;
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
}

console.log("ok " + count + (failures ? " (" + failures + " mismatches)" : ""));
