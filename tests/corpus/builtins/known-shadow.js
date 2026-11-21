// Known builtins (D3300): names that are not the global. A local, a parameter, a closure
// variable, a class or function named like a known builtin, and a `with` object.
function local() { const Math = { floor: (x) => "local " + x }; return Math.floor(1.5); }
function param(String) { return String(5); }
function outer() { const parseInt = (s) => "outer " + s; return () => parseInt("9"); }
function named() { function isNaN(x) { return "mine " + x; } return isNaN(1); }
function klass() { class Number { static isInteger(x) { return "static " + x; } } return Number.isInteger(2); }
function destructured({ Math }) { return Math.max(1, 2); }
function caught() { try { throw { floor: () => "caught" }; } catch (Math) { return Math.floor(1); } }
function loops() { let r = ""; for (const Array of [{ isArray: () => "loop" }]) r += Array.isArray(1); return r; }
function hoisted() { const r = typeof Math; var Math; return r; }
console.log(local(), param((x) => "param " + x), outer()(), named(), klass(), destructured({ Math: { max: () => "destructured" } }), caught(), loops(), hoisted());
console.log(Math.floor(1.5), String(5), parseInt("9"), isNaN(1), Number.isInteger(2), Math.max(1, 2), Array.isArray(1));
// Reads only: typeof, a missing member, a member that is not known, an index.
console.log(typeof Math, typeof String, typeof Math.floor, Math.nothing, typeof Math.hypot, Math[0], String.name, Math.floor.name, Math.floor.length);
// A known function as a value, as a method of another object, with another `this`.
const o = { f: Math.floor, max: Math.max, s: String };
console.log(o.f(2.5), o.max(1, 5), o.s(1), Math.floor.call(null, 3.5), Math.max.apply(null, [1, 9]), Reflect.apply(String, undefined, [4]), new String(5).length, new Number(7) + 1);
console.log([1.5, 2.5].map(Math.floor).join(), ["1", "2"].map(Number).join(), [0, 1, ""].filter(Boolean).length, [[1], 2].filter(Array.isArray).length);
