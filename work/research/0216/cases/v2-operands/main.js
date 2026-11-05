"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 18);
globalThis.$dump = {};
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
$dump.p1 = $djs.resume(1, "later", [2]);
$dump.p2 = $djs.resume(2, "later", [2]);
$dump.p3 = $djs.resume(3, "later", [5]);
$dump.p4 = $djs.resume(4, "later", [2]);
$dump.p5 = $djs.resume(5, "later", [2]);
$dump.p6 = $djs.resume(6, "later", ["x"]);
$dump.p7 = $djs.resume(7, "later", [2]);
$dump.p8 = $djs.resume(8, "later", [3]);
$dump.p9 = $djs.resume(9, "later", [3]);
$dump.p10 = $djs.resume(10, "later", [3]);
$dump.p11 = $djs.resume(11, "later", [6]);
$dump.p12 = $djs.resume(12, "later", [1]);
$dump.p13 = $djs.resume(13, "later", [2]);
$dump.p14 = $djs.resume(14, "later", [4]);
$dump.p15 = $djs.resume(15, "later", [2]);
$dump.p16 = $djs.resume(16, "later", [2]);
$dump.p17 = $djs.resume(17, "later", ["m"]);
class Thing {
  constructor(...a) {
    this.a = a;
  }
}
$dump.n18 = $djs.fn({ f(a, b, c) { return [this.k, a, b, c].join(); } });
const o = { k: 1, f: $dump.n18 };
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
$dump.a19_0 = o;
$dump.a19 = $djs.k(async function () {
const $djs_R = $djs.R;
const $djs_t0 = $dump.n18, $djs_t1 = $dump.a19_0;
return $djs_R($djs_t0, $djs_t1, [1, await $dump.p1, 3]);
}, { f: call3, segs: [145,55,1], scaf: [["site",39,129]], cmap: [["call.callee@32",0,"$djs_t0"],["call.this@32",0,"$djs_t1"],["call.arg@36",1,1]] })();
$dump.a20_0 = o;
$dump.a20 = $djs.k(async function () {
const $djs_R = $djs.R, $djs_Q = $djs.Q;
const $djs_t0 = $dump.n18, $djs_t1 = $dump.a20_0, $djs_t2 = [1];
return $djs_R($djs_t0, $djs_t1, $djs_Q($djs_t2, await $dump.p2, 3));
}, { f: spread, segs: [189,61,1], scaf: [["site",45,173]], cmap: [["call.callee@33",0,"$djs_t0"],["call.this@33",0,"$djs_t1"],["call.args@36",0,"$djs_t2"]] })();
$dump.a21 = $djs.k(async function () {
return Math.max(1, await $dump.p3);
}, { f: plain, segs: [27,32,18,53,58,2], scaf: [["site",44,39]], cmap: [["call.arg@41",1,1]] })();
$dump.a22_0 = Thing;
$dump.a22 = $djs.k(async function () {
const $djs_C = $djs.C;
const $djs_t0 = $dump.a22_0;
return $djs_C($djs_t0, [1, await $dump.p4]).a.join();
}, { f: newer, segs: [115,60,10], scaf: [["site",45,99]], cmap: [["new.callee@32",0,"$djs_t0"],["new.arg@42",1,1]] })();
$dump.a23_0 = Thing;
$dump.a23 = $djs.k(async function () {
const $djs_C = $djs.C, $djs_Q = $djs.Q;
const $djs_t0 = $dump.a23_0, $djs_t1 = [1];
return $djs_C($djs_t0, $djs_Q($djs_t1, await $dump.p5)).a.join();
}, { f: newSpread, segs: [159,69,10], scaf: [["site",54,143]], cmap: [["new.callee@36",0,"$djs_t0"],["new.args@45",0,"$djs_t1"]] })();
$dump.n24 = { x: 9 };
$dump.a25 = $djs.k(async function () {
const $djs_t0 = $dump.n24;
return $djs_t0[(await $dump.p6)];
}, { f: member, scaf: [["site",55,63]], cmap: [["member.object@53",0,"$djs_t0"]] })();
$dump.a26_0 = { a: 1 };
$dump.a26 = $djs.k(async function () {
const $djs_D = $djs.D;
const $djs_t0 = $dump.a26_0;
return JSON.stringify(($djs_D($djs_t0, "z", await $dump.p7), $djs_t0));
}, { f: key, segs: [79,45,15,141,89,2], scaf: [["site",73,116]], cmap: [["object.literal@60",0,"$djs_t0"],["object.key@68",1,"z"]] })();
$dump.a27_0 = { a: 1 };
$dump.a27 = $djs.k(async function () {
const $djs_D = $djs.D;
const $djs_t0 = $dump.a27_0;
return JSON.stringify(($djs_D($djs_t0, "c", await $dump.p8), $djs_t0));
}, { f: value, segs: [79,32,15,141,74,2], scaf: [["site",58,116]], cmap: [["object.literal@47",0,"$djs_t0"]] })();
$dump.a28 = $djs.k(async function () {
return [1, 2, await $dump.p9].join();
}, { f: elements, segs: [49,57,8], scaf: [["site",42,34]], cmap: [["array.element@36",1,1],["array.element@39",1,2]] })();
$dump.a29 = $djs.k(async function () {
const $djs_Q = $djs.Q;
const $djs_t0 = [1, , ];
return $djs_Q($djs_t0, await $dump.p10).length;
}, { f: hole, segs: [107,52,8], scaf: [["site",37,91]], cmap: [["array.literal@31",0,"$djs_t0"]] })();
$dump.a30 = $djs.k(async function () {
return (5 * (await $dump.p11));
}, { f: left, scaf: [["site",49,33]], cmap: [["binary.left@44",1,5]] })();
$dump.n31 = {};
$dump.a32 = $djs.k(async function (t) {
const $djs_t0 = $dump.n31;
($djs_t0.p = await $dump.p12);
return JSON.stringify(t);
}, { f: prop, segs: [57,40,10,79,60,25], scaf: [["site",44,61]], cmap: [["assign.object@38",0,"$djs_t0"]] })($dump.n31);
$dump.n33 = {};
$dump.a34 = $djs.k(async function (t) {
const $djs_t0 = $dump.n33;
($djs_t0["q"] = await $dump.p13);
return JSON.stringify(t);
}, { f: index, segs: [82,71,25], scaf: [["site",55,64]], cmap: [["assign.object@48",0,"$djs_t0"],["assign.key@48",1,"q"]] })($dump.n33);
$dump.a35 = $djs.k(async function (n) {
(n = 1 + await $dump.p14);
return n;
}, { f: compound, segs: [22,39,1,48,60,9], scaf: [["site",44,30]], cmap: [["assign.old@39",1,1]] })(1);
$dump.n36 = { r: 1 };
$dump.a37 = $djs.k(async function (t) {
const $djs_t0 = $dump.n36;
($djs_t0.r = 1 + await $dump.p15);
return t.r;
}, { f: compoundProp, segs: [57,54,1,83,75,11], scaf: [["site",59,65]], cmap: [["assign.object@52",0,"$djs_t0"],["assign.old@52",1,1]] })($dump.n36);
$dump.a38 = $djs.k(async function () {
return `a${"1"}b${await $dump.p16}c`;
}, { f: template, segs: [27,35,4,34,40,10,53,58,4], scaf: [["site",44,38]], cmap: [["template.part@39",1,"1"]] })();
$dump.n39 = { __proto__: Child.prototype };
$dump.a40_0 = Base.prototype;
$dump.a40 = $djs.k(({ __proto__: Base.prototype, async K(x) {
return super[(await $dump.p17)](x);
} }).K, { f: Child.prototype.m, segs: [20,24,6,44,47,4], scaf: [["site",30,27]] }).call($dump.n39, 7);
$dump.p41 = Promise.all([$dump.a19, $dump.a20, $dump.a21, $dump.a22, $dump.a23, $dump.a25, $dump.a26, $dump.a27, $dump.a28, $dump.a29, $dump.a30, $dump.a32, $dump.a34, $dump.a35, $dump.a37, $dump.a38, $dump.a40]);
$dump.a42 = $djs.k(async function () {
const rs = await $dump.p41;
for (const r of rs) print(r);
}, { f: main, segs: [26,32,11,48,252,31], scaf: [["site",37,31]] })();
$djs.keep(Thing, []);
$djs.keep(Thing.prototype, []);
$djs.keep(Base, []);
$djs.keep(Base.prototype, ["m"]);
$djs.keep(Child, []);
$djs.keep(Child.prototype, ["m"]);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
