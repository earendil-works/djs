// Object.defineProperty, getOwnPropertyDescriptor, getOwnPropertyNames,
// hasOwnProperty, propertyIsEnumerable.
function show(d) {
  if (d === undefined) return "undefined";
  const keys = Object.getOwnPropertyNames(d);
  const parts = [];
  for (let i = 0; i < keys.length; i++) {
    const v = d[keys[i]];
    parts.push(keys[i] + ":" + (typeof v === "function" ? "fn" : String(v)));
  }
  return "{" + parts.join(",") + "}";
}
const o = {};
Object.defineProperty(o, "a", { value: 1 });
Object.defineProperty(o, "b", { value: 2, writable: true, enumerable: true, configurable: true });
Object.defineProperty(o, "c", { get() { return 3; }, enumerable: true });
Object.defineProperty(o, 5, { value: "five", enumerable: true });
console.log(show(Object.getOwnPropertyDescriptor(o, "a")), show(Object.getOwnPropertyDescriptor(o, "b")));
console.log(show(Object.getOwnPropertyDescriptor(o, "c")), show(Object.getOwnPropertyDescriptor(o, "zz")));
console.log(o.a, o.b, o.c, o[5], Object.getOwnPropertyNames(o));
console.log(o.hasOwnProperty("a"), o.hasOwnProperty("zz"), o.hasOwnProperty(5), o.propertyIsEnumerable("a"), o.propertyIsEnumerable("b"));
const key = { toString() { return "b"; } };
console.log(o.hasOwnProperty(key), show(Object.getOwnPropertyDescriptor(o, key)));
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
attempt("redefine non-configurable", () => Object.defineProperty(o, "a", { value: 9 }));
attempt("same value is fine", () => Object.defineProperty(o, "a", { value: 1 }) === o);
attempt("not an object", () => Object.defineProperty(1, "x", {}));
attempt("bad descriptor", () => Object.defineProperty({}, "x", 1));
attempt("bad getter", () => Object.defineProperty({}, "x", { get: 1 }));
attempt("mixed", () => Object.defineProperty({}, "x", { get() {}, value: 1 }));
attempt("getOwnPropertyNames(null)", () => Object.getOwnPropertyNames(null));
const arr = [1, 2, 3];
Object.defineProperty(arr, "length", { value: 1 });
console.log(arr.length, arr[1], Object.getOwnPropertyNames(arr));
attempt("bad length", () => Object.defineProperty([], "length", { value: -1 }));
console.log(Array.isArray(arr), Array.isArray({ length: 0 }), [].push.call({ length: 1 }, "x"));
console.log(Array(3).length, Array(1, 2).length, new Array("3").length, Array().length);
console.log(String(), String(undefined), String(null), String(12.5), String(Symbol("sym")), String(true));
console.log(Math.pow(2, 0.5), Math.pow(-8, 1 / 3), Math.pow(NaN, 0), Object.getOwnPropertyNames(Math).indexOf("pow") >= 0);
console.log(new Object() instanceof Object, Object(null) instanceof Object, typeof Object(o), Object(o) === o);
