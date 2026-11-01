// Map, Set, WeakMap, WeakSet (0038): codemode-style counting and dedup, iteration while mutating, the Set methods.
const m = new Map([["a", 1], ["b", 2], [NaN, "nan"], [-0, "zero"]]);
console.log(m.size, m.get("a"), m.get(NaN), m.get(0), m.has(+0), [...m.keys()], [...m.values()], [...m], [...m.entries()].length);
m.set("c", 3).delete("a");
const order = []; m.forEach((v, k, mm) => { order.push(k); if (k === "b") { mm.delete("c"); mm.set("d", 4); } });
console.log(order, m.size, Object.prototype.toString.call(m), Object.prototype.toString.call(m.keys()), String(m[Symbol.iterator] === m.entries));
const it = m.keys(); m.clear(); m.set("x", 1); console.log([...it], m.size);
const counts = new Map(); for (const w of ["a", "b", "a", "c", "b", "a"]) counts.set(w, (counts.get(w) || 0) + 1); console.log([...counts]);
const s = new Set([1, 2, 2, 3, "3", -0]);
console.log(s.size, [...s], s.has(0), [...s.entries()], [...s.keys()], s.add(9) === s, s.delete(1), s.delete(42));
const a = new Set([1, 2, 3, 4]), b = new Set([3, 4, 5]);
console.log([...a.union(b)], [...a.intersection(b)], [...a.difference(b)], [...a.symmetricDifference(b)], a.isSubsetOf(b), new Set([3]).isSubsetOf(b), a.isSupersetOf(new Set([1, 2])), a.isDisjointFrom(new Set([9])), a.isDisjointFrom(b));
const mapLike = { size: 2, has: (v) => v === 1, keys: () => [1, 7][Symbol.iterator]() };
console.log([...a.union(mapLike)], [...a.intersection(mapLike)], a.isSupersetOf(mapLike));
const g = Map.groupBy([1, 2, 3, 4, 5], (v) => (v % 2 ? "odd" : "even")); console.log([...g]);
const o1 = {}, o2 = {};
const wm = new WeakMap([[o1, "one"]]); wm.set(o2, "two");
console.log(wm.get(o1), wm.has(o2), wm.delete(o2), wm.has(o2), wm.get(1), wm.has("x"), Object.prototype.toString.call(wm));
const ws = new WeakSet([o1]); console.log(ws.has(o1), ws.has(o2), ws.add(o2) === ws, ws.delete(o1), ws.has(o1));
const sym = Symbol("weak"); wm.set(sym, 1); console.log(wm.get(sym));
for (const f of [() => Map(), () => Map.prototype.get.call({}), () => Set.prototype.add.call(new Map), () => new WeakMap().set(1, 1), () => new WeakSet().add(1), () => WeakMap.prototype.get.call(new Map), () => new Set().union({ size: NaN }), () => new Set().union({ size: -1, has() {}, keys() {} }), () => new Set().union({ size: 1, has: 1 }), () => new Map([1]), () => new WeakMap().set(Symbol.for("reg"), 1)]) {
  try { console.log(f()); } catch (e) { console.log(String(e)); }
}
console.log(Map.length, Set.length, WeakMap.length, Map.prototype.set.length, Map[Symbol.species] === Map);
