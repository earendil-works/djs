"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
async function call3() { return o.f(1, await later(2), 3); }
async function spread() { return o.f(...[1], await later(2), 3); }
async function plain() { return Math.max(1, await later(5)); }
async function newer() { return new Thing(1, await later(2)).a.join(); }
async function newSpread() { return new Thing(...[1], await later(2)).a.join(); }
async function member() { const t = { x: 9 }; return t[await later("x")]; }
async function key() { const k = "z"; return JSON.stringify({ a: 1, [k]: await later(2) }); }
async function value() { return JSON.stringify({ a: 1, c: await later(3) }); }
async function elements() { return [1, 2, await later(3)].join(); }
async function hole() { return [1, , await later(3)].length; }
async function left() { const a = 5; return a * (await later(6)); }
async function prop() { const t = {}; t.p = await later(1); return JSON.stringify(t); }
async function index() { const t = {}, k = "q"; t[k] = await later(2); return JSON.stringify(t); }
async function compound() { let n = 1; n += await later(4); return n; }
async function compoundProp() { const t = { r: 1 }; t.r += await later(2); return t.r; }
async function template() { return `a${1}b${await later(2)}c`; }
async function main() {
  const rs = await Promise.all([
    call3(), spread(), plain(), newer(), newSpread(), member(), key(), value(), elements(),
    hole(), left(), prop(), index(), compound(), compoundProp(), template(), new Child().m(7),
  ]);
  for (const r of rs) print(r);
}
class Thing {
  constructor(...a) {
    this.a = a;
  }
}
const o = { k: 1, f(a, b, c) { return [this.k, a, b, c].join(); } };
class Base {
  m(x) {
    return "base " + x;
  }
}
let Child = ((Base) => class Child extends Base {
  async m(x) {
    return super[await later("m")](x);
  }
})(Base);
$djs.k(async function () {
const rs = await Promise.all([$djs.k(async function () {
return o.f(1, await $djs.resume(1,"later",[2]), 3);
},call3)(),$djs.k(async function () {
return o.f(...[1], await $djs.resume(2,"later",[2]), 3);
},spread)(),$djs.k(async function () {
return Math.max(1, await $djs.resume(3,"later",[5]));
},plain)(),$djs.k(async function () {
return new Thing(1, await $djs.resume(4,"later",[2])).a.join();
},newer)(),$djs.k(async function () {
return new Thing(...[1], await $djs.resume(5,"later",[2])).a.join();
},newSpread)(),$djs.k(async function (t) {
return t[await $djs.resume(6,"later",["x"])];
},member)({ x: 9 }),$djs.k(async function (k) {
return JSON.stringify({ a: 1, [k]: await $djs.resume(7,"later",[2]) });
},key)("z"),$djs.k(async function () {
return JSON.stringify({ a: 1, c: await $djs.resume(8,"later",[3]) });
},value)(),$djs.k(async function () {
return [1, 2, await $djs.resume(9,"later",[3])].join();
},elements)(),$djs.k(async function () {
return [1, , await $djs.resume(10,"later",[3])].length;
},hole)(),$djs.k(async function (a) {
return a * (await $djs.resume(11,"later",[6]));
},left)(5),$djs.k(async function (t) {
t.p = await $djs.resume(12,"later",[1]);
return JSON.stringify(t);
},prop)({}),$djs.k(async function (t, k) {
t[k] = await $djs.resume(13,"later",[2]);
return JSON.stringify(t);
},index)({}, "q"),$djs.k(async function (n) {
n += await $djs.resume(14,"later",[4]);
return n;
},compound)(1),$djs.k(async function (t) {
t.r += await $djs.resume(15,"later",[2]);
return t.r;
},compoundProp)({ r: 1 }),$djs.k(async function () {
return `a${1}b${await $djs.resume(16,"later",[2])}c`;
},template)(),$djs.k(({ __proto__: Base.prototype, async K(x) {
return super[await $djs.resume(17,"later",["m"])](x);
} }).K,Child.prototype.m).call({__proto__:Child.prototype},7)]);
for (const r of rs) print(r);
},main)();
