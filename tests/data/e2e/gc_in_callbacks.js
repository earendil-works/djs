// Allocation-heavy code inside constructors, getters, setters, ToPrimitive callbacks,
// builtins (intrinsics) and spreads, for the GC stress mode: every safepoint collects
// while the outer operation is still in progress.
function F(n) {
  const tmp = [1, 2, 3].map(function (x) { return { v: x * n, s: "v" + x }; });
  this.sum = tmp[0].v + tmp[2].v;
  this.s = "s" + n + tmp[1].s;
}
const objs = [];
for (let i = 0; i < 30; i++) objs[i] = new F(i);
console.log(objs[29].sum, objs[5].s, objs.length);
function G() { return { made: [1, 2].map(function (x) { return [x, "y" + x]; }) }; }
console.log(new G().made[1][1]);
const g = {
  get big() { const a = []; for (let i = 0; i < 40; i++) a[i] = { i: i, t: "t" + i }; return a; },
  set big(v) { const copy = [...v, ...v].map(function (x) { return x * 2; }); this.last = copy.length + ":" + copy[5]; },
};
console.log(g.big.length, g.big[39].t);
g.big = [1, 2, 3];
console.log(g.last);
const tp = { valueOf: function () { const junk = [1, 2, 3].map(function (x) { return [x, { x: x }]; }); return junk.length; } };
const ts = { toString: function () { return ["a", "b"].map(function (x) { return x + x; }).join("-"); } };
console.log(tp + tp * 2, `${ts}`, tp > 2, ts + "!");
const key = { toString: function () { return ["k", "e", "y"].join(""); } };
const box = {};
box[key] = [1].map(function () { return "stored"; })[0];
console.log(box.key, key in box);
function sum(...xs) { let t = 0; for (let i = 0; i < xs.length; i++) t += xs[i]; return t; }
console.log(sum(...[1, 2, 3].map(function (x) { return x * 10; }), ...[4]), new F(...[3]).s);
const mg = { tag: "!", get m() { const k = [7].map(function (x) { return x; }); return function () { return "m" + k[0] + this.tag; }; } };
console.log(mg.m(), mg["m"]());
const nested = [1, 2, 3].map(function (x) { return new F(x).s + [x].map(function (y) { return y + tp; })[0]; });
console.log(nested.join("|"));
console.log(typeof Object.defineProperty, String({}), typeof Array.prototype.push);
