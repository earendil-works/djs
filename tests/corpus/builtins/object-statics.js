// Object statics and Object.prototype.
const o = { b: 2, a: 1, 10: "ten", 2: "two" };
console.log(Object.keys(o), Object.values(o), Object.entries(o));
console.log(Object.assign({ x: 0 }, { x: 1, y: 2 }, null, undefined, { z: 3 }));
const proto = { inherited: true };
const c = Object.create(proto, { own: { value: 1, enumerable: true }, hidden: { value: 2 } });
console.log(Object.keys(c), c.inherited, c.hidden, Object.getPrototypeOf(c) === proto);
console.log(Object.getPrototypeOf(Object.create(null)), Object.getPrototypeOf(1) === Number.prototype);
const frozen = Object.freeze({ f: 1 });
console.log(Object.isFrozen(frozen), Object.isSealed(frozen), Object.isExtensible(frozen));
const sealed = Object.seal({ s: 1 });
sealed.s = 2;
console.log(sealed.s, Object.isFrozen(sealed), Object.isSealed(sealed));
const nx = Object.preventExtensions({ p: 1 });
console.log(Object.isExtensible(nx), Object.isSealed(nx), Object.isFrozen({}), Object.isFrozen(1));
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
attempt("write frozen", () => { "use strict"; frozen.f = 2; });
attempt("extend non-extensible", () => { "use strict"; nx.q = 1; });
attempt("setPrototypeOf cycle", () => { const a = {}; const b = Object.create(a); Object.setPrototypeOf(a, b); });
attempt("setPrototypeOf non-extensible", () => Object.setPrototypeOf(nx, {}));
attempt("setPrototypeOf primitive", () => Object.setPrototypeOf(1, null));
attempt("create bad proto", () => Object.create(1));
attempt("getPrototypeOf null", () => Object.getPrototypeOf(null));
attempt("keys undefined", () => Object.keys(undefined));
console.log(Object.is(NaN, NaN), Object.is(0, -0), Object.is("a", "a"), Object.hasOwn(o, "a"), Object.hasOwn(c, "inherited"));
const sym = Symbol("k");
const withSym = { [sym]: 1, s: 2 };
console.log(Object.getOwnPropertySymbols(withSym).length, Object.getOwnPropertySymbols(withSym)[0] === sym);
const descs = Object.getOwnPropertyDescriptors({ v: 1, get g() { return 2; } });
console.log(descs.v.value, descs.v.writable, typeof descs.g.get, descs.g.enumerable);
Object.defineProperties(o, { d1: { value: "one", enumerable: true }, d2: { get() { return "two"; } } });
console.log(o.d1, o.d2, Object.keys(o));
const ts = Object.prototype.toString;
console.log(ts.call(undefined), ts.call(null), ts.call([]), ts.call(1), ts.call("s"), ts.call(true),
  ts.call(function () {}), ts.call(new Error("e")), ts.call(Math), ts.call({ [Symbol.toStringTag]: "Custom" }), ts.call(Symbol()));
console.log(String({}), ({}).toLocaleString(), typeof ({}).valueOf(), Object.prototype.isPrototypeOf.call(proto, c));
console.log(Object.keys("abc"), Object.entries([7, 8]), Object.values("hi"));
