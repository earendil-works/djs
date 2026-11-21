// Map and Set iteration without result objects (0270): Array.from and spread of Sets, Maps
// and their iterators (%IteratorFastDrain), new Set(array) and new Map(array of pairs) in
// the constructors' new entries, and the cases where each must step aside.
const s = new Set(["a", "b", "c", 1, 2]);
const m = new Map([["x", 1], ["y", 2], ["z", 3]]);
console.log(JSON.stringify([[...s], Array.from(s), [...m], Array.from(m), [...m.keys()], [...m.values()], [...s.entries()], Array.from(m.entries())]));
const it = s.values(); it.next(); console.log(JSON.stringify([...it]), JSON.stringify([...it]));
const s2 = new Set([1, 2, 3, 4, 5, 6]); s2.delete(2); const it2 = s2.values(); it2.next(); s2.delete(4); s2.add(7); console.log(JSON.stringify([...it2]));
const big = new Set(); for (let i = 0; i < 100; i++) big.add(i); for (let i = 0; i < 100; i += 3) big.delete(i);
const it3 = big.keys(); for (let i = 0; i < 10; i++) it3.next(); for (let i = 0; i < 60; i += 2) big.delete(i + 1); console.log([...it3].join());
const SetNext = Object.getPrototypeOf(new Set().values()).next;
const it4 = new Set([1, 2]).values(); it4.next = function () { const r = SetNext.call(this); if (!r.done) r.value *= 10; return r; }; console.log([...it4].join());
console.log([...new Set([3, 1, 3, 2, 1])].join(), Array.from(new Map([[1, "a"]]).entries())[0].join());
class MySet extends Set {} console.log([...new MySet([1, 2])].join());
console.log(Array.from(new Set([1, 2]), (x) => x * 2).join());
{
  const s = new Set([1, 2, 3]);
  const [a, b] = s; console.log(a, b);
  for (const x of s) { if (x === 1) s.delete(2); if (x === 3) s.add(4); console.log("of", x); }
  const m = new Map([[1, "a"], [2, "b"]]);
  for (const [k, v] of m) console.log(k, v);
  console.log([...new Set(s)].join(), [...new Map(m)].join("|"), Array.from(s, (x) => x + 1).join());
  const it = m.keys(); console.log([...it].join(), [...it].length, it.next().done);
  const orig = Set.prototype[Symbol.iterator];
  Set.prototype[Symbol.iterator] = function* () { yield "patched"; };
  console.log([...s].join(), Array.from(s).join());
  Set.prototype[Symbol.iterator] = orig;
  const SIP = Object.getPrototypeOf(new Set().values());
  const on = SIP.next; SIP.next = function () { const r = on.call(this); if (!r.done) r.value = "n" + r.value; return r; };
  console.log([...s].join(), [...s.values()].join());
  SIP.next = on;
  const own = s.values(); Object.defineProperty(own, Symbol.iterator, { value: function () { return [9][Symbol.iterator](); } });
  console.log([...own].join());
  const m2 = new Map([[1, 1]]); m2[Symbol.iterator] = function () { return [[7, 7]][Symbol.iterator](); }; console.log([...m2].join());
  Promise.all(new Set([Promise.resolve(1), 2])).then((r) => console.log("all", r.join()));
  console.log(Math.max(...new Set([3, 9, 1])));
}
{
  const SIP = Object.getPrototypeOf(new Set().values());
  SIP.return = function () { console.log("return called"); return {}; };
  function C() { return Object.freeze([]); }
  try { Array.from.call(C, new Set([1, 2])); } catch (e) { console.log(e.constructor.name); }
  function D() { const a = []; Object.defineProperty(a, "length", { writable: false }); return a; }
  try { Array.from.call(D, new Set([1, 2])); } catch (e) { console.log(e.constructor.name); }
  delete SIP.return;
  function E() { const a = [7, 8, 9, 10]; return a; }
  console.log(JSON.stringify(Array.from.call(E, new Set([1, 2]))));
  const holey = [1, , 3]; console.log(JSON.stringify([...holey, ...new Set([4])]));
}
{
const a = ["x", "y", "x", 1, -0, 0, NaN, NaN, "z"];
const s = new Set(a); console.log([...s].map(String).join(), s.size, s.has(0), Object.is([...s][4], 0));
const m = new Map([["a", 1], ["b", 2], ["a", 3], [-0, "z"]]); console.log([...m].join("|"), m.size, Object.is([...m.keys()][2], 0));
console.log(new Set([1, , 3]).size, new Set([]).size, new Map([]).size);
try { new Map([1, 2]); } catch (e) { console.log(e.constructor.name, e.message); }
try { new Map([[1]]); } catch (e) { console.log("short", e.message); } console.log(JSON.stringify([...new Map([[1]])]));
Array.prototype[1] = "proto"; console.log([...new Set([0, , 2])].join()); delete Array.prototype[1];
const orig = Set.prototype.add; let calls = 0; Set.prototype.add = function (v) { calls++; return orig.call(this, v); }; new Set([1, 2]); console.log("calls", calls); Set.prototype.add = orig;
const arr = [1, 2]; arr[Symbol.iterator] = function* () { yield 9; }; console.log([...new Set(arr)].join());
class S2 extends Set {} console.log(new S2([1, 2]) instanceof S2, new S2([1, 2]).size);
const AIP = Object.getPrototypeOf([][Symbol.iterator]()); const on = AIP.next; AIP.next = function () { const r = on.call(this); if (!r.done) r.value = "n" + r.value; return r; }; console.log([...new Set([1])].join()); AIP.next = on;
const big = Array.from({ length: 50 }, (_, i) => "k" + (i % 20)); console.log(new Set(big).size, new Map(big.map((k, i) => [k, i])).get("k5"));
}
