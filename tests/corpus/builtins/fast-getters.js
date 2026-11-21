// Builtin getters with fast entries (0282, D3530): Map and Set `size`, typed arrays'
// `length`, `byteLength` and `byteOffset`. Read in loops, so that the cache slot leads to
// the getter and its entry runs inside the interpreter's loop; every receiver the entry
// declines must throw from the builtin as before, and a getter that is replaced, shadowed
// or deleted must stop being used.
function show(f) {
  try {
    return String(f());
  } catch (e) {
    return e.name + ": " + e.message;
  }
}
const m = new Map([[1, 2], [3, 4]]), st = new Set([1, 2, 3]);
let acc = 0;
for (let i = 0; i < 8; i++) {
  acc += m.size + st.size;
  if (i === 3) { m.set(9, 9); st.delete(1); }
  if (i === 5) m.clear();
}
console.log("sizes", acc, m.size, st.size);
const mapSize = Object.getOwnPropertyDescriptor(Map.prototype, "size").get;
const setSize = Object.getOwnPropertyDescriptor(Set.prototype, "size").get;
console.log(typeof mapSize, mapSize.name, mapSize.length, mapSize.call(m), setSize.call(st));
for (const bad of [{}, st, 1, "s", undefined, null, new WeakMap(), Map.prototype]) {
  console.log(show(() => mapSize.call(bad)), "|", show(() => setSize.call(bad === st ? m : bad)));
}
console.log(show(() => Map.prototype.size), show(() => Set.prototype.size));
class Counted extends Map { get size() { return -super.size; } }
const c = new Counted([[1, 1], [2, 2]]);
const sizes = [];
for (let i = 0; i < 6; i++) {
  if (i === 2) Object.defineProperty(Map.prototype, "size", { get() { return "patched"; }, configurable: true });
  if (i === 4) Object.defineProperty(Map.prototype, "size", { get: mapSize, configurable: true });
  sizes.push(m.size, c.size, st.size);
}
console.log(sizes.join(" "));
// The getters as accessors of other objects: `this` is whatever the read is on.
const borrowed = Object.create(null, { size: { get: mapSize, configurable: true } });
const wrapped = Object.create(m);
for (let i = 0; i < 3; i++) console.log(show(() => borrowed.size), show(() => wrapped.size));

const ta = new Uint8Array([1, 2, 3, 4]), f64 = new Float64Array(3);
const sub = new Uint16Array(new ArrayBuffer(16), 4, 3);
acc = 0;
for (let i = 0; i < 6; i++) {
  acc += ta.length + ta.byteLength + ta.byteOffset + f64.length + f64.byteLength;
  acc += sub.length + sub.byteOffset + sub.byteLength;
}
console.log("typed", acc);
// Named and numeric keys of a typed array through the same sites.
for (let i = 0; i < 4; i++) {
  ta.extra = i;
  console.log(ta.extra, ta[1], ta["1"], ta["1.5"], ta[-1], ta["-0"], ta.length, ta.foo, typeof ta.at, typeof ta.NaN);
}
// A resizable buffer under a length-tracking view and under a fixed one.
const rab = new ArrayBuffer(8, { maxByteLength: 16 });
const tracking = new Uint8Array(rab), fixed = new Uint8Array(rab, 4, 4);
for (let i = 0; i < 6; i++) {
  if (i === 2) rab.resize(12);
  if (i === 4) rab.resize(6);
  console.log(tracking.length, tracking.byteLength, fixed.length, fixed.byteLength, fixed.byteOffset);
}
const taLength = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), "length").get;
for (const bad of [{}, [], 1, new ArrayBuffer(4), new DataView(new ArrayBuffer(4))]) console.log(show(() => taLength.call(bad)));
class T extends Uint8Array { get length() { return 99; } }
const t = new T(2);
for (let i = 0; i < 3; i++) console.log(t.length, t.byteLength);
Object.defineProperty(ta, "length", { value: 7 });
for (let i = 0; i < 3; i++) console.log(ta.length, ta.byteLength);
