// Shapes and their inline caches (0083): additions in constructors and literals, the
// prototype chain changing under a warm addition cache, receiver-shape proofs of
// prototype reads, deletes, attribute changes and dictionaries.
function P(i) { this.x = i; this.y = i * 2; }
const ps = [];
for (let i = 0; i < 5; i++) ps.push(new P(i));
console.log(ps.map((p) => p.x + p.y).join(","), Object.keys(ps[4]).join(","));

// A setter for `y` appears on the prototype: [[Set]] must call it, not add `y`.
let calls = 0;
Object.defineProperty(P.prototype, "y", { set(v) { calls += v; }, get() { return "proto y"; }, configurable: true });
const q = new P(3);
console.log(calls, q.y, Object.keys(q).join(","), Object.hasOwn(q, "y"));
delete P.prototype.y;

// A read-only `z` on Object.prototype: the addition throws (codemode bodies are strict).
const add = (o, v) => { o.z = v; return o; };
for (let i = 0; i < 4; i++) add({ w: i }, i);
Object.defineProperty(Object.prototype, "z", { value: "ro", writable: false, configurable: true });
const r = { w: 9 };
try { add(r, 1); } catch (e) { console.log(e.constructor.name); }
console.log(r.z, Object.hasOwn(r, "z"));
delete Object.prototype.z;
console.log(add({ w: 9 }, 7).z);

// A different prototype with the same receiver shape.
const withSetter = { set z(v) { calls = "setter " + v; } };
const o2 = Object.create(withSetter);
o2.w = 1;
add(o2, 5);
console.log(calls, Object.hasOwn(o2, "z"));

// Not extensible after warming.
const ne = { w: 1 };
Object.preventExtensions(ne);
try { add(ne, 3); } catch (e) { console.log(e.constructor.name); }
console.log(Object.hasOwn(ne, "z"), JSON.stringify(ne));

// Literals define properties (no setters run) even with a setter on Object.prototype.
const lit = (i) => ({ a: i, b: i + 1 });
for (let i = 0; i < 4; i++) lit(i);
Object.defineProperty(Object.prototype, "a", { set(v) { calls = "literal setter"; }, configurable: true });
const l = lit(5);
console.log(l.a, l.b, Object.hasOwn(l, "a"), calls);
delete Object.prototype.a;

// Prototype reads: a receiver with the cached shape has no own key; a receiver that
// shadows it has a different shape.
const Base = { greet() { return "base"; } };
const mk = (k) => { const o = Object.create(Base); o.k = k; return o; };
const greet = (o) => o.greet();
for (let i = 0; i < 4; i++) greet(mk(i));
const shadow = mk(9);
shadow.greet = () => "own";
console.log(greet(mk(1)), greet(shadow));
Base.greet = () => "changed";
console.log(greet(mk(2)));

// Deletes: last key, a middle key, and attribute changes keep values right.
const d = { a: 1, b: 2, c: 3, d: 4 };
delete d.d;
delete d.b;
Object.defineProperty(d, "a", { enumerable: false });
d.e = 5;
console.log(JSON.stringify(d), Object.getOwnPropertyNames(d).join(","));
for (let i = 0; i < 3; i++) { const t = { x: i, y: i + 1 }; delete t.x; t.z = i; console.log(JSON.stringify(t)); }

// Many keys (more than a shape holds) and dynamic keys.
const big = {};
for (let i = 0; i < 40; i++) big["k" + i] = i;
console.log(Object.keys(big).length, big.k0, big.k39, JSON.stringify(big).length);
const groups = {};
for (const w of "a b a c b a d e f g".split(" ")) groups[w] = (groups[w] || 0) + 1;
console.log(JSON.stringify(groups));
// Frozen and sealed objects.
const fr = Object.freeze({ a: 1, b: { c: 2 } });
try { fr.a = 2; } catch (e) { console.log(e.constructor.name); }
console.log(Object.isFrozen(fr), fr.a, Object.isSealed(Object.seal({ q: 1 })));
// Accessors in shape mode.
const acc = { get v() { return this._v ?? "none"; }, set v(x) { this._v = x; } };
acc.v = 3;
console.log(acc.v, Object.keys(acc).join(","));
