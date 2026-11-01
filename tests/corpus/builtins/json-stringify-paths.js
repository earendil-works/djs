// JSON.stringify's mixed paths (0090, D1901): plain subtrees inside the full algorithm,
// Dates with the original toJSON in the fast path, array replacers (PropertyList).
const d = new Date(Date.UTC(2024, 0, 2));
const o = { a: 1, b: { c: [1, { d: d, e: "x" }, 3], f: { g: null } }, h: [d, { i: 2 }] };
console.log(JSON.stringify(o));
console.log(JSON.stringify(o, null, 2));
console.log(JSON.stringify(o, null, "--"));
const cyc = { a: { b: {} }, t: d };
cyc.a.b.c = cyc;
try { JSON.stringify(cyc); } catch (e) { console.log(String(e)); }
const cyc2 = { t: d, a: [{ x: 1 }] };
cyc2.a[0].y = cyc2.a;
try { JSON.stringify(cyc2); } catch (e) { console.log(String(e)); }
const tj = { toJSON() { return { z: { toJSON() { return "inner"; } }, w: [1] }; } };
console.log(JSON.stringify({ tj, d }), JSON.stringify([tj]), JSON.stringify(tj));
const back = { toJSON() { return d; } };
console.log(JSON.stringify({ back, n: new Number(3), s: new String("s"), k: { get g() { return 1; } } }));
class P { constructor() { this.x = 1; } toJSON() { return "P"; } }
console.log(JSON.stringify({ p: [new P()], q: { r: new P() }, t: d }, null, 1));
const dd = new Date(NaN);
console.log(JSON.stringify({ dd, d, arr: [d, dd] }), JSON.stringify(d), JSON.stringify([new Date(8.64e15)]));
const d2 = new Date(0); d2.toISOString = () => "own"; console.log(JSON.stringify({ d2 }));
const d3 = new Date(0); d3.valueOf = () => NaN; console.log(JSON.stringify({ d3 }));
const d4 = new Date(0); Object.defineProperty(d4, Symbol.toPrimitive, { value: () => Infinity }); console.log(JSON.stringify([d4]));
const iso = Date.prototype.toISOString; Date.prototype.toISOString = function () { return "patched"; };
console.log(JSON.stringify({ d }));
Date.prototype.toISOString = iso;
const tj2 = Date.prototype.toJSON; Date.prototype.toJSON = function () { return 1; }; console.log(JSON.stringify({ d })); Date.prototype.toJSON = tj2;
class MyDate extends Date {} console.log(JSON.stringify({ m: new MyDate(0) }));
console.log(JSON.stringify({ d }, null, 2));
const proto = { inh: "I" };
const lo = Object.create(proto); lo.a = 1; lo.b = { a: 2, c: 3, inh: [1, { a: 4 }] };
Object.defineProperty(lo, "hid", { value: "H", enumerable: false });
console.log(JSON.stringify(lo, ["a", "b", "inh", "hid", "missing"]), JSON.stringify([lo, d], ["a", "b"], 1));
const lg = { a: 1, get b() { return { a: 5 }; } };
console.log(JSON.stringify(lg, ["b", "a"]), JSON.stringify({ a: [1, 2], 1: "one" }, [1, "a"]));
console.log(JSON.stringify({ a: d, b: { toJSON() { return { a: 9, z: 0 }; } } }, ["a", "b"]));
