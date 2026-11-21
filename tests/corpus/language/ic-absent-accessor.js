// Property reads through the caches' kinds for absent keys and accessors (0276, D3470):
// each loop reads one site many times while what the cache relies on changes under it
// (a key appears on the receiver, on its prototype or further up; an accessor is
// redefined, becomes data, loses its getter, is deleted; the prototype is replaced).
const out = [];
function read(o) { return o.k; }           // one site, many receivers
function readG(o) { return o.g; }
function readM(o) { return o.m; }

// Absent, then present at each level.
const top = { topOnly: 1 };
const mid = Object.create(top);
const leaf = Object.create(mid);
leaf.own = 1;
for (let i = 0; i < 12; i++) {
  if (i === 3) top.k = "top";
  if (i === 5) mid.k = "mid";
  if (i === 7) leaf.k = "leaf";
  if (i === 9) delete leaf.k;
  if (i === 10) delete mid.k;
  if (i === 11) delete top.k;
  out.push(String(read(leaf)));
}
console.log("levels", out.join(" "));
out.length = 0;

// Absent with the same shape but another prototype, and a prototype swap.
const pa = { tag: "a" }, pb = { tag: "b", k: "from-b" };
const a1 = Object.create(pa), a2 = Object.create(pb);
a1.x = 1; a2.x = 1;
for (let i = 0; i < 8; i++) {
  out.push(String(read(i & 1 ? a2 : a1)));
  if (i === 5) Object.setPrototypeOf(a1, pb);
}
console.log("protos", out.join(" "));
out.length = 0;

// Object.prototype gets the key and loses it again; null-prototype and dictionary receivers.
const plain = { a: 1 }, bare = Object.create(null), dict = { a: 1 };
for (let i = 0; i < 40; i++) dict["p" + i] = i;
delete dict.p3;
for (let i = 0; i < 9; i++) {
  if (i === 3) Object.prototype.k = "inherited";
  if (i === 6) delete Object.prototype.k;
  out.push([read(plain), read(bare), read(dict)].join("/"));
}
console.log("object.prototype", out.join(" "));
out.length = 0;

// Accessors on the prototype: redefined, turned into data, setter only, deleted, shadowed.
let calls = 0;
class P { get g() { calls++; return "g1:" + this.id; } }
const p1 = new P(), p2 = new P();
p1.id = 1; p2.id = 2;
for (let i = 0; i < 14; i++) {
  if (i === 3) Object.defineProperty(P.prototype, "g", { get() { calls++; return "g2:" + this.id; }, configurable: true });
  if (i === 5) Object.defineProperty(P.prototype, "g", { value: "data", configurable: true, writable: true });
  if (i === 7) Object.defineProperty(P.prototype, "g", { set(v) {}, configurable: true });
  if (i === 9) Object.defineProperty(P.prototype, "g", { get() { calls++; return "g3:" + this.id; }, configurable: true });
  if (i === 11) Object.defineProperty(p1, "g", { value: "own", configurable: true });
  if (i === 12) delete p1.g;
  if (i === 13) delete P.prototype.g;
  out.push(String(readG(i & 1 ? p2 : p1)));
}
console.log("accessor", out.join(" "), calls);
out.length = 0;

// Own accessors, a getter that throws, a getter that changes its holder, primitives.
let n = 0;
const own = { get g() { return ++n; }, v: 1 };
const thrower = { get g() { throw new RangeError("no"); } };
const changer = {};
Object.defineProperty(changer, "g", { get() { Object.defineProperty(changer, "g", { value: "now data", configurable: true }); return "was getter"; }, configurable: true });
Object.defineProperty(String.prototype, "g", { get() { return "str:" + this.length; }, configurable: true });
Object.defineProperty(Number.prototype, "g", { get() { return "num:" + (this + 1); }, configurable: true });
for (let i = 0; i < 4; i++) {
  let t;
  try { t = readG(thrower); } catch (e) { t = e.name; }
  out.push([readG(own), t, readG(changer), readG("abc"), readG(7), readG(true), readG(Symbol.iterator)].join("/"));
}
delete String.prototype.g;
delete Number.prototype.g;
out.push([readG("abc"), readG(7)].join("/"));
console.log("own", out.join(" "), n);
out.length = 0;

// Methods found through the same kinds: absent, then a method, then an accessor returning one.
const obj = {};
for (let i = 0; i < 9; i++) {
  if (i === 3) Object.getPrototypeOf(obj).m = function () { return "m" + i; };
  if (i === 6) Object.defineProperty(Object.prototype, "m", { get() { return () => "got" + i; }, configurable: true });
  let r;
  try { r = obj.m(); } catch (e) { r = e.name; }
  out.push(r, typeof readM(obj));
}
delete Object.prototype.m;
console.log("method", out.join(" "));

// Builtin accessors and absent keys on builtins, in a loop with a collection-heavy body.
let acc = 0;
const m = new Map([[1, 2]]), st = new Set([1, 2, 3]), ta = new Uint8Array(5), re = /x/gi;
for (let i = 0; i < 300; i++) {
  acc += m.size + st.size + ta.length + ta.byteLength + (re.global ? 1 : 0) + (m.nothing === undefined ? 1 : 0) + re.flags.length;
  if (i % 50 === 0) m.set(i, [i]);
}
console.log("builtins", acc);
