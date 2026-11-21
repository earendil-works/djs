// Object.fromEntries: an Array of entry arrays with string keys is built in one step
// (0262); everything that could observe the difference takes the iterator protocol.
function attempt(label, fn) {
  try {
    console.log(label, "->", JSON.stringify(fn()));
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const show = (o) => JSON.stringify(Object.getOwnPropertyNames(o).map((k) => [k, o[k]]));

// The one-step shape: duplicates keep the first place and the last value, index-like
// keys sort first, __proto__ is an own property, values of every kind.
const row = { ts: "2026", service: "api", status: 200, ok: true, nested: { a: 1 }, nil: null };
console.log(show(Object.fromEntries(Object.entries(row))));
const dup = Object.fromEntries([["b", 1], ["a", 2], ["b", 3], ["2", "two"], ["1", "one"], ["__proto__", 5]]);
console.log(show(dup), Object.getPrototypeOf(dup) === Object.prototype, Object.keys(dup).join());
console.log(show(Object.fromEntries([])), show(Object.fromEntries([["k", undefined, "extra"]])));
const many = Object.fromEntries(Array.from({ length: 300 }, (_, i) => ["k" + i, i]));
console.log(Object.keys(many).length, many.k0, many.k299);
const desc = Object.getOwnPropertyDescriptor(Object.fromEntries([["x", 1]]), "x");
console.log(JSON.stringify(desc));

// Other keys and entries: numbers, symbols, objects with toString, short entries, holes
// in the entry, entries that are not arrays, a Map, a generator.
const sym = Symbol("s");
console.log(show(Object.fromEntries([[1, "n"], [1.5, "f"], [-0, "z"]])), Object.fromEntries([[sym, 1]])[sym]);
const log = [];
const key = { toString() { log.push("toString"); return "obj"; } };
console.log(show(Object.fromEntries([["a", 1], [key, 2]])), log.join());
console.log(show(Object.fromEntries([["s"]])), show(Object.fromEntries([[, "v"]])), show(Object.fromEntries([{ 0: "o", 1: "p" }])));
console.log(show(Object.fromEntries(new Map([["m", 1], ["n", 2]]))));
console.log(show(Object.fromEntries((function* () { yield ["g", 1]; yield ["h", 2]; })())));
console.log(show(Object.fromEntries([new String("ab")])), show(Object.fromEntries([new Int8Array([1, 2])])));

// Getters on the entries and on the outer array, a Proxy entry: each read in order.
const ent = ["g", 0];
Object.defineProperty(ent, 1, { get() { log.push("get 1"); return "lazy"; } });
log.length = 0;
console.log(show(Object.fromEntries([["a", 1], ent])), log.join());
const outer = [["a", 1], ["b", 2]];
Object.defineProperty(outer, 1, { get() { log.push("outer 1"); return ["c", 3]; } });
log.length = 0;
console.log(show(Object.fromEntries(outer)), log.join());
log.length = 0;
const px = new Proxy(["p", 9], { get(t, k) { log.push("proxy " + String(k)); return t[k]; } });
console.log(show(Object.fromEntries([px])), log.join());
const sparse = [["a", 1], , ["c", 3]];
attempt("hole in the iterable", () => Object.fromEntries(sparse));

// Errors.
attempt("null", () => Object.fromEntries(null));
attempt("not iterable", () => Object.fromEntries({}));
attempt("primitive entry", () => Object.fromEntries([["a", 1], "xy"]));
attempt("number entry", () => Object.fromEntries([["a", 1], 5]));
attempt("throwing toString", () => Object.fromEntries([[{ toString() { throw new Error("key"); } }, 1]]));

// The iteration observed: a replaced Array iterator or next sees every step.
const saved = Array.prototype[Symbol.iterator];
Array.prototype[Symbol.iterator] = function* () { log.push("custom"); yield ["it", 1]; };
log.length = 0;
console.log(show(Object.fromEntries([["a", 1]])), log.join());
Array.prototype[Symbol.iterator] = saved;
const ai = Object.getPrototypeOf([][Symbol.iterator]());
const next = ai.next;
ai.next = function () { log.push("next"); return next.call(this); };
log.length = 0;
console.log(show(Object.fromEntries([["a", 1], ["b", 2]])), log.join());
ai.next = next;
const own = [["a", 1]];
own[Symbol.iterator] = function* () { yield ["own", 2]; };
console.log(show(Object.fromEntries(own)));
