// A value JSON.stringify cannot serialize falls back to String().
const o = { name: "self" };
o.self = o;
console.log(o);
console.log({ big: 1n });
console.log([1n]);
console.log("before", o, "after");
// Not a cycle: the same object twice.
const shared = { s: 1 };
console.log([shared, shared]);
// A cyclic object nested inside an array: String() of the array calls join.
console.log([1, o]);
