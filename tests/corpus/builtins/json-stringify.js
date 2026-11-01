// JSON.stringify: the fast path, toJSON, replacer functions and arrays, gaps, wrapper
// objects, accessors, cycles, and qjs's messages.
console.log(JSON.stringify({ a: [1, "two", null, true, false, -0, 1.5, NaN, Infinity], b: { c: {} }, d: [] }));
console.log(JSON.stringify("\u2028\ud800a\"\\\b\u0001\u007f\u00e9\u{1F600}"), JSON.stringify(1e21), JSON.stringify(-1e-7));
console.log(JSON.stringify({ a: undefined, b() {}, c: Symbol(), [Symbol()]: 1, d: [undefined, function () {}, Symbol()] }));
console.log(JSON.stringify([new Boolean(false), new String("s"), new Number(NaN), Object(Symbol()), new Number(3)]));
console.log(JSON.stringify(undefined), JSON.stringify(function () {}), JSON.stringify(Symbol()), JSON.stringify(null));
console.log(JSON.stringify({ a: [1, { b: 2 }], e: {}, f: [] }, null, 2));
console.log(JSON.stringify([[1], { x: [] }], null, "--"), JSON.stringify([1], null, "abcdefghijklmnop"));
console.log(JSON.stringify({ a: 1 }, null, 20) === JSON.stringify({ a: 1 }, null, 10), JSON.stringify({ a: 1 }, null, -3), JSON.stringify({ a: 1 }, null, 3.9));
console.log(JSON.stringify({ a: 1 }, null, new Number(1)), JSON.stringify({ a: 1 }, null, new String("ab")), JSON.stringify({ a: 1 }, null, true));
console.log(JSON.stringify({ b: 1, a: 2, 1: 3, c: { a: 4, z: 5 } }, [1, "a", new String("b"), new Number(1), {}, "c", "a"]));
console.log(JSON.stringify({ a: 1, b: [1, 2], c: "s" }, function (k, v) { return typeof v === "number" ? v * 2 : v; }));
console.log(JSON.stringify("a", (k, v) => (k === "" ? [v, v] : v + "!")));
const seen = [];
JSON.stringify({ x: { y: 1 }, z: [2] }, function (k, v) { seen.push(JSON.stringify(k) + ":" + (this === undefined ? "?" : Object.keys(this).join("/"))); return v; });
console.log(seen.join(" "));
console.log(JSON.stringify({ toJSON(k) { return "k:" + k; } }), JSON.stringify({ x: { toJSON(k) { return k + "!"; } } }), JSON.stringify({ toJSON: 1, x: 2 }));
console.log(JSON.stringify([{ toJSON(k) { return typeof k + k; } }]), JSON.stringify(new Date(0)), JSON.stringify({ d: new Date(86400000) }));
let n = 0;
const acc = { get a() { n++; return { b: n }; }, c: 1 };
Object.defineProperty(acc, "hidden", { get() { throw new Error("not enumerable"); }, enumerable: false });
console.log(JSON.stringify(acc), n);
const inherited = Object.create({ inheritedKey: 1 }); inherited.own = 2;
console.log(JSON.stringify(inherited));
const proto = Object.create(null); proto.toJSON = function () { return "from proto"; };
console.log(JSON.stringify(Object.create(proto)), JSON.stringify([Object.create(null)]));
const cyc = { a: {} }; cyc.a.self = cyc;
for (const f of [() => JSON.stringify(cyc), () => JSON.stringify(cyc, (k, v) => v), () => { const a = []; a.push(a); JSON.stringify(a, null, 1); },
  () => JSON.stringify({ toJSON() { throw new TypeError("from toJSON"); } }), () => JSON.stringify({ get x() { throw new RangeError("getter"); } })]) {
  try { console.log(f()); } catch (e) { console.log(e.name, e.message); }
}
const shared = { s: 1 };
console.log(JSON.stringify([shared, shared, { shared }]));
console.log(JSON.stringify({ a: [] }, null, 2), JSON.stringify([[]], null, 2), JSON.stringify({}, null, 2));
console.log(JSON.stringify(Object.assign([1, 2], { extra: 3 })), JSON.stringify({ 2: "b", 1: "a", x: "c" }));
console.log(JSON.stringify(new Map([[1, 2]])), JSON.stringify(new Set([1])), JSON.stringify(Object.assign(new String("ab"), { x: 1 })));
const replaced = JSON.stringify({ a: { b: 1 } }, function (k, v) { if (k === "a") return new Number(7); return v; });
console.log(replaced, JSON.stringify.length, JSON.stringify.name);
