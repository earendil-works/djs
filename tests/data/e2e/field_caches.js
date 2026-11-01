// Property caches (0062, D1620): one site sees objects of many layouts and every change.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
function getA(o) { return o.a; }
function setA(o, v) { o.a = v; return o.a; }
function callM(o) { return o.m(); }
const objs = [
  { a: 1 },
  { b: 2, a: 3 },
  { a: 4, b: 5 },
  Object.create({ a: "proto" }),
  [1, 2, 3],
  Object.assign([1], { a: "array prop" }),
  new String("str"),
  Object.assign(new String("s"), { a: "wrapper prop" }),
  new Proxy({ a: "target" }, { get: (t, k) => "trap " + String(k) }),
  new Proxy({ a: "plain target" }, {}),
  new Uint8Array([7, 8]),
  Object.assign(function f() {}, { a: "fn prop" }),
  Object.freeze({ a: "frozen" }),
  { get a() { return "getter"; } },
];
for (let r = 0; r < 2; r++) console.log(objs.map(getA).join("|"));
// (Error names only: djs names the property in "no setter for property" messages.)
for (const o of objs) { try { console.log(setA(o, "set")); } catch (e) { console.log(e.name); } }
console.log(objs.map(getA).join("|"));
// The same object, changing under the cache.
const o = { x: 1, a: 2, y: 3 };
for (let i = 0; i < 3; i++) getA(o);
delete o.x;
console.log(getA(o));
delete o.a;
console.log(getA(o));
o.a = "back";
console.log(getA(o), setA(o, "again"));
Object.defineProperty(o, "a", { get() { return "now a getter"; }, set(v) { console.log("setter", v); }, configurable: true });
console.log(getA(o), setA(o, 9));
Object.defineProperty(o, "a", { value: "read-only", writable: false, configurable: true });
console.log(getA(o));
t(() => setA(o, 10));
// Many properties: hashed tables, and entries moving when the table grows.
const big = {};
for (let i = 0; i < 40; i++) big["p" + i] = i;
big.a = "big a";
console.log(getA(big));
for (let i = 40; i < 80; i++) big["p" + i] = i;
delete big.p3;
console.log(getA(big), setA(big, "big set"), getA(big));
// Methods.
const m1 = { m() { return "own m"; } };
const m2 = { x: 1, m() { return "own m2"; } };
class C { m() { return "class m"; } }
for (const x of [m1, m2, new C(), m1, { m: () => "arrow m" }]) console.log(callM(x));
t(() => callM({ m: 1 }));
t(() => callM(null));
// Primitives and index-like keys.
function getLen(x) { return x.length; }
console.log([[1, 2], "abc", { length: 7 }, function (a, b) {}].map(getLen).join(","));
function get0(x) { return x["0"]; }
console.log(get0([5]), get0({ 0: "zero" }), get0("q"));
