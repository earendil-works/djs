// for-in key snapshots (0278): only enumerable keys are read, shadowing by lower objects,
// deletion during the loop, prototypes changed by the program.
const out = [];
const keys = (o) => { const r = []; for (const k in o) r.push(k); return r.join(","); };
out.push(keys({ b: 1, a: 2, 1: 3, 0: 4 }));
out.push(keys([1, , 3]));
out.push(keys(new String("ab")));
out.push(keys("xyz"));
out.push(keys(new Uint8Array(3)));
out.push(keys(function f(a, b) {}));
const p = { inh: 1, sh: 2 }; Object.defineProperty(p, "hid", { value: 1, enumerable: false });
const c = Object.create(p); c.own = 1; Object.defineProperty(c, "sh", { value: 3, enumerable: false });
out.push(keys(c));
(function () { out.push(keys(arguments)); })(7, 8);
Object.prototype.extra = 1; out.push(keys({ z: 1 })); delete Object.prototype.extra;
Array.prototype.ex = 2; out.push(keys([5])); delete Array.prototype.ex;
const d = { a: 1, b: 2, c: 3 }; const seen = []; for (const k in d) { seen.push(k); delete d.b; } out.push(seen.join());
const big = {}; for (let i = 0; i < 40; i++) big["k" + i] = i; delete big.k3; out.push(keys(big).split(",").length);
class K { constructor() { this.x = 1; } m() {} } out.push(keys(new K()));
const sym = { [Symbol("s")]: 1, q: 2 }; out.push(keys(sym));
out.push(keys(Object.assign(Object.create(null), { n: 1 })));
out.push(keys(null), keys(undefined), keys(5));
const m = new Map([[1, 2]]); m.prop = 1; out.push(keys(m));
const e = new Error("x"); e.code = 1; out.push(keys(e));
console.log(out.join(" | "));
