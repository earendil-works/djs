// The own properties of the constructors (0106, D2052): `prototype` comes last, as qjs
// installs it after the statics; Error's statics are in qjs's order.
console.log(Object.getOwnPropertyNames(Error).join());
console.log(Reflect.ownKeys(Error).map(String).join());
const ctors = [Array, ArrayBuffer, BigInt, Boolean, DataView, Date, Error, EvalError, Function,
  Int8Array, Iterator, Map, Number, Object, Promise, RangeError, RegExp, Set, String, Symbol,
  TypeError, Uint8Array, WeakMap, WeakRef, AggregateError, SuppressedError,
  Object.getPrototypeOf(Int8Array), (async function () {}).constructor, (function* () {}).constructor];
for (const C of ctors) {
  const keys = Reflect.ownKeys(C);
  console.log(C.name, keys[0], keys[1], keys[keys.length - 1] === "prototype");
}
console.log(Reflect.ownKeys(Proxy).map(String).join());
console.log(JSON.stringify(Object.getOwnPropertyDescriptor(Error, "prototype")), Error.prototype.constructor === Error);
// The statics and namespace members in qjs's order (0108, D2072).
for (const o of [Array, BigInt, Iterator, Number, Object, Promise, Symbol, Uint8Array, Date, String, RegExp, Map, Math, JSON, Reflect]) {
  console.log(Reflect.ownKeys(o).map(String).join());
}
