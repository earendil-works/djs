// Nested objects and arrays use compact JSON.
console.log({ a: 1, b: { c: [1, 2, { d: "x" }] } });
console.log([1, [2, [3, [4, [5, [6]]]]]]);
console.log({}, [], [[]], [{}], { a: {} });
// Integer-like keys come first, in numeric order, then strings in insertion order.
console.log({ b: 1, 2: "two", a: 2, 1: "one", "-1": "neg", "01": "lead" });
// undefined, functions and symbols are dropped from objects and become null in arrays.
console.log({ u: undefined, f() {}, s: Symbol("x"), n: null, k: 1 });
console.log([undefined, function () {}, Symbol("x"), null, 1]);
// Symbol keys and non-enumerable properties are not serialized.
const o = { visible: 1, [Symbol("k")]: 2 };
Object.defineProperty(o, "hidden", { value: 3, enumerable: false });
console.log(o);
// Getters are called.
console.log({ get g() { return "got"; } });
// Prototype properties are not own properties.
const child = Object.create({ inherited: 1 });
child.own = 2;
console.log(child);
console.log(Object.create(null));
