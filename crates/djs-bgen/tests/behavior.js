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

console.log("ok " + count + (failures ? " (" + failures + " mismatches)" : ""));
