// 0038: Map/Set with object keys across collections and snapshots, iteration while
// mutating, WeakMap entries.
const keys = [];
const m = new Map();
for (let i = 0; i < 300; i++) {
  const k = { i };
  keys.push(k);
  m.set(k, i * 2);
  m.set("s" + i, i);
}
let ok = true;
for (let i = 0; i < 300; i++) {
  if (m.get(keys[i]) !== i * 2 || m.get("s" + i) !== i) ok = false;
}
console.log(m.size, ok);
for (let i = 0; i < 300; i += 2) m.delete(keys[i]);
console.log(m.size, m.has(keys[1]), m.has(keys[2]));
const s = new Set([3, 1, 3, 2, 1]);
const seen = [];
for (const v of s) {
  seen.push(v);
  if (v === 3) { s.delete(1); s.add(9); }
}
console.log(seen, [...s]);
const wm = new WeakMap();
const live = {};
wm.set(live, "kept");
for (let i = 0; i < 100; i++) wm.set({ i }, new Array(10).fill(i));
console.log(wm.get(live), wm.has({}));
const groups = Map.groupBy([1, 2, 3, 4, 5, 6], (v) => v % 3);
console.log([...groups.keys()], groups.get(0));
console.log([...new Set([1, 2, 3]).union(new Set([3, 4]))], new Set([1, 2]).isSubsetOf(new Set([1, 2, 3])));
