// The long tail (0082): evaluation order and scope fixes found by test262.

// Compound assignment and update: the base is checked before the key is converted,
// and the key is converted once for the get and the put.
let conversions = 0;
const key = { toString() { conversions++; return "k"; } };
const obj = { k: 1 };
obj[key] += 1;
obj[key]++;
--obj[key];
obj[key] ||= 5;
console.log(obj.k, conversions);
for (const op of ["+=", "++", "||="]) {
  const f = new Function("b", "k", op === "++" ? "b[k]++" : "b[k] " + op + " 1");
  try {
    f(null, { toString() { throw new Error("key converted"); } });
  } catch (e) {
    console.log(op, e.name, e.message);
  }
}
try {
  null[{ toString() { throw new Error("key converted"); } }];
} catch (e) {
  console.log(e.name, e.message);
}

// ToNumeric(lhs) completes, throwing for a Symbol, before the rhs is converted.
const trace = [];
try {
  ({ valueOf() { trace.push("l"); return Symbol(); } }) -
    ({ valueOf() { trace.push("r"); return 1; } });
} catch (e) {
  console.log(e.name, trace.join(""));
}

// Logical assignment names anonymous functions.
let fa;
fa ||= function () {};
let fb = 1;
fb &&= () => {};
let fc = null;
fc ??= class {};
console.log(fa.name, fb.name, fc.name);

// Bodies with non-simple parameters keep their vars apart from the parameters.
var x = "outside";
let probeParams, probeBody;
(function (_ = (probeParams = () => x)) {
  var x = "inside";
  probeBody = () => x;
})();
console.log(probeParams(), probeBody());
function sameName(a, b = () => a) {
  var a;
  const before = a;
  a = 2;
  return [before, a, b()];
}
console.log(sameName(1));

// Catch blocks with a pattern parameter have a scope of their own.
let probeCatch, probeBlock;
let y = "outside";
try {
  throw [];
} catch ([_ = (probeCatch = () => y)]) {
  probeBlock = () => y;
  let y = "inside";
}
console.log(probeCatch(), probeBlock());

// `(a?.b)()` keeps `this`.
const a = { b() { return this.v; }, v: 42 };
console.log((a?.b)(), (a?.b)?.(), (a?.["b"])());

// A computed key is converted before the value is evaluated.
let value = "bad";
const lit = { [{ toString() { value = "ok"; return "p"; } }]: value };
console.log(lit.p);

// Strict arguments objects: callee throws.
(function () {
  const d = Object.getOwnPropertyDescriptor(arguments, "callee");
  console.log(typeof d.get, d.get === d.set, d.enumerable, d.configurable);
  try {
    arguments.callee;
  } catch (e) {
    console.log(e.name);
  }
})();

// Builtins.
console.log(Math.sumPrecise([1e20, 0.1, -1e20]), Math.sumPrecise([]), Math.sumPrecise([0.1, 0.2]));
console.log(Math.f16round(1.337), Math.f16round(65520), Math.f16round(5e-8));
console.log(Error.isError(new TypeError()), Error.isError({ __proto__: Error.prototype }));
try {
  Math.sumPrecise([1, "2"]);
} catch (e) {
  console.log(e.name);
}
class O extends Object {}
console.log(new O({ a: 1 }).a, Object.getPrototypeOf(new O()) === O.prototype);
console.log(Reflect.setPrototypeOf(Object.prototype, {}), Number.prototype.toString.length);
