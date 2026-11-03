// A closure's `length` and `name` come from its template until changed (D2445): every
// way of reading, redefining, deleting and enumerating them, on user closures, classes
// and builtins, keeps the spec's attributes and key order.
const out = [];
const show = (label, v) => out.push(label + "=" + JSON.stringify(v));
const desc = (o, k) => {
  const d = Object.getOwnPropertyDescriptor(o, k);
  return d && [d.value, d.writable, d.enumerable, d.configurable];
};

function f(a, b) {}
const arrow = (x) => x;
class C { static m() {} }
show("keys", [Reflect.ownKeys(f), Reflect.ownKeys(arrow), Reflect.ownKeys(C)]);
show("desc", [desc(f, "length"), desc(f, "name"), desc(arrow, "name"), desc(C, "length")]);
show("has", [Object.hasOwn(arrow, "name"), "length" in arrow, arrow.hasOwnProperty("x")]);

// Deleting one keeps the other keys' order and exposes Function.prototype's.
function g(a) {}
g.extra = 1;
delete g.name;
show("delete name", [Reflect.ownKeys(g), g.name, g.length]);
delete g.length;
show("delete length", [Reflect.ownKeys(g), g.length]);
try {
  g.name = "x";
} catch (e) {
  // Function.prototype.name is read-only too.
  show("assign after delete", [e.constructor.name, g.name, Object.hasOwn(g, "name")]);
}

// Redefining stores them: values and order as before.
function h(a, b, c) {}
h.later = 2;
Object.defineProperty(h, "name", { value: "renamed" });
show("redefine name", [Reflect.ownKeys(h), h.name, h.length, desc(h, "name")]);
function h2() {}
Object.defineProperty(h2, "length", { value: 7, writable: true });
h2.length = 8;
show("redefine length", [Reflect.ownKeys(h2), h2.length, h2.name, desc(h2, "length")]);
function h3() {}
Object.defineProperty(h3, "name", { value: "h3" });
show("same value", [Reflect.ownKeys(h3), desc(h3, "name")]);

// Non-writable: assignment fails, strict mode throws.
try {
  f.name = "nope";
} catch (e) {
  show("assign", [e.constructor.name, f.name]);
}
try {
  arrow.length = 3;
} catch (e) {
  show("assign length", [e.constructor.name, arrow.length]);
}

// Freezing, sealing.
function fr(a) {}
Object.freeze(fr);
show("frozen", [Object.isFrozen(fr), desc(fr, "name"), desc(fr, "length"), Reflect.ownKeys(fr)]);
const se = function sealed() {};
Object.seal(se);
show("sealed", [Object.isSealed(se), desc(se, "name")]);

// Classes: a static `name` method or field replaces the name.
class S { static name() { return "s"; } }
class T { static name = "t"; static length = 9; }
show("class statics", [typeof S.name, S.name(), T.name, T.length, Reflect.ownKeys(T)]);
const anon = class {};
show("anon class", [anon.name, Reflect.ownKeys(anon)]);

// Computed names, numeric names, symbols, accessors.
const k = "computed";
const obj = {
  [k]() {},
  1() {},
  [Symbol.iterator]() {},
  get g() { return 1; },
};
show("method names", [
  obj.computed.name,
  obj[1].name,
  obj[Symbol.iterator].name,
  Object.getOwnPropertyDescriptor(obj, "g").get.name,
  Reflect.ownKeys(obj[1]),
]);

// Builtins.
show("builtins", [
  Array.prototype.map.name,
  Array.prototype.map.length,
  Reflect.ownKeys(Array.prototype.map),
  desc(Math.max, "length"),
  Reflect.ownKeys(Array),
  Object.getOwnPropertyDescriptor(Map.prototype, "size").get.name,
  Array.prototype[Symbol.iterator].name,
]);
const pm = Array.prototype.push;
delete pm.name;
show("builtin delete", [pm.name, Reflect.ownKeys(pm)]);
Object.defineProperty(Array.prototype.pop, "name", { value: "popped" });
show("builtin redefine", [Array.prototype.pop.name, Reflect.ownKeys(Array.prototype.pop)]);

// Inherited reads through a closure on the prototype chain, and cached sites.
const child = Object.create(f);
const read = (o) => [o.name, o.length];
const reads = [];
for (const o of [f, child, arrow, f, child, function zz(q) {}]) reads.push(read(o));
show("inherited", reads);
const plain = {};
const set = (o) => { o.name = "set"; return Object.hasOwn(o, "name"); };
show("set sites", [set(plain), set({}), (() => { try { return set(function () {}); } catch (e) { return e.constructor.name; } })()]);

// for-in: own non-enumerable keys shadow a prototype's enumerable ones.
const proto = { name: 1, length: 2, other: 3 };
const fp = function () {};
Object.setPrototypeOf(fp, proto);
const seen = [];
for (const key in fp) seen.push(key);
show("for-in", seen);

// Bound functions read the target's.
function target(a, b, c) {}
const bound = target.bind(null, 1);
show("bind", [bound.name, bound.length]);
delete target.length;
show("bind after delete", [target.bind(null).length]);

// Spread and assign skip non-enumerable keys.
show("spread", [{ ...f }, Object.assign({}, arrow), Object.keys(C), JSON.stringify(Object.entries(f))]);

for (const line of out) console.log(line);
