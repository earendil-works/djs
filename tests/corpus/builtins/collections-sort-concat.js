// 0231's fast paths against qjs: Map/Set get/has/set/add @fast entries (and where they must
// decline), the comparator sort's leaves, the default sort of strings in place, a + b of
// strings in the hot loop, join of split lines (slices) and Array.from from %Array%.
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}

// Map and Set with string, number, -0, NaN, object and rope keys.
const m = new Map();
const rope = "a long key made of pieces, " + "longer than thirty-two units";
m.set("k", 1).set(-0, "zero").set(NaN, "nan").set(rope, "rope");
console.log(m.get("k"), m.get(0), m.get(+0), m.get(NaN), m.get("a long key made of pieces, longer than thirty-two units"), m.get("x"), m.size);
console.log(m.has("k"), m.has(-0), m.has(NaN), m.has(undefined), m.has(), m.get());
m.set(); m.set("k", 2);
console.log(m.has(undefined), m.get(undefined), m.get("k"), m.size, [...m.keys()].map(String).join());
const o = {};
m.set(o, "obj");
console.log(m.get(o), m.get({}), Object.is([...m.keys()][1], 0));
const s = new Set();
console.log(s.add("a") === s, s.add("a").size, s.add(-0).has(0), s.has(NaN), s.add(NaN).has(NaN), s.has(), s.add().has(undefined), s.size);
for (let i = 0; i < 300; i++) s.add("key" + (i % 120));
console.log(s.size, s.has("key119"), s.has("key120"), [...s].slice(-2).join());
// Not a Map or Set: the builtin's TypeError.
attempt("Map.get on a Set", () => Map.prototype.get.call(s, "a"));
attempt("Map.set on {}", () => Map.prototype.set.call({}, 1, 2));
attempt("Map.has on a WeakMap", () => Map.prototype.has.call(new WeakMap(), {}));
attempt("Set.add on a Map", () => Set.prototype.add.call(m, 1));
attempt("Set.has on null", () => Set.prototype.has.call(null, 1));
// Patched methods are called, not the fast paths.
const saved = Map.prototype.set;
Map.prototype.set = function (k, v) { console.log("patched set", k, v); return saved.call(this, k, v); };
const m2 = new Map([["x", 1]]);
m2.set("y", 2);
Map.prototype.set = saved;
console.log(m2.size, m2.get("x"), m2.get("y"));
class MyMap extends Map { get(k) { return "sub " + super.get(k); } }
console.log(new MyMap([["a", 1]]).get("a"));

// The comparator sort: results, stability, inconsistent and throwing comparators,
// non-numbers from the comparator, undefined and holes.
const rows = Array.from({ length: 40 }, (_, i) => ({ k: (i * 7) % 5, i }));
console.log(rows.slice().sort((a, b) => a.k - b.k).map((r) => r.i).join());
console.log([5, 1, undefined, 4, , 3, 1, 2].sort((a, b) => a - b).join(), [3, 1, 2].sort(() => 0).join());
console.log([3, 1, 2].sort((a, b) => (a < b ? "-1" : "1")).join(), [3, 1, 2].sort((a, b) => ({ valueOf: () => b - a })).join());
console.log([3, 1, 2].sort(() => NaN).join(), [1, 2, 3, 4].sort((a, b) => b - a).join(), [2, 1].toSorted((a, b) => a - b).join());
const seen = new Set();
const shuffled = Array.from({ length: 50 }, (_, i) => (i * 17) % 50);
const sorted = shuffled.slice().sort((a, b) => { seen.add(a); seen.add(b); return Math.sin(a * b) > 0 ? 1 : -1; });
console.log(sorted.length, sorted.slice().sort((a, b) => a - b).join() === shuffled.slice().sort((a, b) => a - b).join(), seen.size);
attempt("comparator throws", () => [2, 1, 3].sort(() => { throw new Error("cmp"); }));
attempt("comparator symbol", () => [2, 1].sort(() => Symbol("s")));
attempt("not callable", () => [2, 1].sort(5));
console.log(new Float64Array([3, 0.5, 0, 1]).sort((a, b) => b - a).join(), new Int8Array([3, 1, 2]).sort().join());
const calls = [];
const objs = [3, 1, 2].map((n) => ({ toString() { calls.push(n); return String(n); } }));
console.log(objs.sort().map(String).join(), calls.length > 0);

// The default sort of strings: one- and two-byte, ropes, slices, numbers.
const text = Array.from({ length: 30 }, (_, i) => "line " + ((i * 13) % 30) + " of some text that is long enough to slice").join("\n");
const lines = text.split("\n");
console.log(lines.slice().sort().slice(0, 3).join("|"));
console.log(["b", "a\u0100", "a", "a\u00e9", "ab", "", 10, 9, 1, true, null, "\u00ff", "\u0100", "\ud800", "\uffff"].sort().map(String).join(","));

// Strings joined: slices, one-byte and two-byte, holes and null, long separators.
console.log(lines.join("\n") === text, lines.join("").length, lines.join(" :: ").length);
console.log([lines[0], null, undefined, "x", , "\u0151"].join("-"), ["a", "b"].join(lines[1]).length);

// a + b of strings in loops (the hot loop's concatenation), and ropes from it.
let acc = "";
let total = 0;
for (let i = 0; i < 200; i++) {
  const k = "svc" + (i % 7);
  const v = k + "/" + i;
  total += (k + v).length;
  acc = acc + v;
}
console.log(total, acc.length, acc.slice(0, 20), acc.slice(-12));
let two = "";
for (let i = 0; i < 5; i++) two = two + "\u0151" + i;
console.log(two, two.length, "x" + 1 + 2, 1 + 2 + "x");

// Array.from: %Array%, subclasses and other constructors, array-likes and iterables.
console.log(Array.from({ length: 4 }, (_, j) => j * j).join(), Array.from({ length: 2, 1: "b" }).join(), Array.from([1, , 3]).length, Array.from("h\u00e9!").join("|"));
class Sub extends Array {}
const sub = Sub.from({ length: 2 }, () => 1);
console.log(sub instanceof Sub, sub.length);
function Ctor(n) { this.made = n; }
const c = Array.from.call(Ctor, { length: 2, 0: "z" });
console.log(c.made, c.length, c[0], Object.keys(c).join());
attempt("Array.from huge", () => Array.from({ length: 2 ** 32 }).length);
attempt("Array.from bad map", () => Array.from([1], 5));

// A string and a primitive in the hot loop's `+`; objects and symbols take the general path.
let mixed = "";
for (let i = 0; i < 4; i++) mixed = mixed + i + "," + i / 3 + true + null + undefined + 10n + -0 + NaN + "|";
console.log(mixed, "a" + { toString() { return "T"; } }, 1 + "2", "big" + 2 ** 80 + 1e21 + 1e-7);
attempt("string + symbol", () => { let t = ""; for (let i = 0; i < 3; i++) t = "x" + Symbol("q"); return t; });
