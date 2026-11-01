// Map/WeakMap getOrInsert and getOrInsertComputed (upsert, 0051).
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
const m = new Map([[1, "one"]]);
console.log(m.getOrInsert(1, "uno"), m.getOrInsert(2, "two"), [...m]);
console.log(m.getOrInsertComputed(3, (k) => k * 10), m.getOrInsertComputed(3, () => "never"));
console.log(m.getOrInsertComputed(-0, (k) => Object.is(k, -0)), m.has(0));
// The callback inserts the key itself: its result overwrites that entry.
console.log(m.getOrInsertComputed(4, (k) => { m.set(k, "inner"); return "outer"; }), m.get(4), m.size);
t(() => m.getOrInsertComputed(5));
t(() => Map.prototype.getOrInsert.call({}, 1, 2));
const w = new WeakMap();
const key = {};
console.log(w.getOrInsert(key, 1), w.getOrInsert(key, 2), w.getOrInsertComputed({}, () => "c"));
t(() => w.getOrInsert(1, 2));
t(() => w.getOrInsertComputed(1, 5));
t(() => w.getOrInsertComputed({}, 5));
t(() => WeakMap.prototype.getOrInsert.call(new Map(), {}, 1));
console.log(Map.prototype.getOrInsert.length, WeakMap.prototype.getOrInsertComputed.name);
