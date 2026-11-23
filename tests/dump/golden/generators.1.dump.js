"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype;
async function main() {
  const mid = g(1); mid.next();
  const fresh = g(5);
  const done = g(0); done.next(); done.next(); done.next();
  const it = ag();
  const r1 = it.next(), r2 = it.next();
  await later(0);
  print(mid.next(10).value, fresh.next().value, done.next().done);
  print((await r1).value, (await r2).value);
}
$dump.p1 = $djs.resume(1, "later", ["q1"]);
$dump.p2 = $djs.resume(2, "later", [0]);
var g = function* g(a) { const x = yield a; yield x + 1; };
var ag = async function* ag() { yield await later("q1"); yield "q2"; };
$dump.n6 = { __proto__: $dump.b0 };
$dump.ga5 = () => {

};
$dump.Ka5 = async function* () {

$dump.ga5();
(yield (await $dump.p1));
yield "q2";
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: ag, segs: [36,23,5,61,48,11], scaf: [["site",29,43]] } });
$dump.Ka5.prototype = $dump.n6;
$dump.a5 = $dump.Ka5.call(undefined);
$dump.p3 = $dump.a5.next();
$dump.p4 = $dump.a5.next(undefined);
$dump.n8 = { __proto__: $dump.b1 };
$dump.Ka7 = function* () {
var a = 1;
const x = (yield);
yield x + 1;
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: g, at: "yield#1", segs: [32,23,1,45,36,12], scaf: [["site",27,37]] } });
$dump.Ka7.prototype = $dump.n8;
$dump.a7 = $dump.Ka7.call(undefined);
$dump.a7.next();
$dump.a9 = Reflect.apply(g, undefined, [5]);
$dump.a10 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), g.prototype);
$dump.Ka11 = async function () {
const mid = $dump.a7;
const fresh = $dump.a9;
const done = $dump.a10;
const it = $dump.a5;
const r1 = $dump.p3;
const r2 = $dump.p4;
(await $dump.p2);
print(mid.next(10).value, fresh.next().value, done.next().done);
print((await r1).value, (await r2).value);
};
Object.defineProperty($dump.Ka11, "$djs", { value: { f: main, segs: [171,217,64,236,284,42], scaf: [["site",199,154]] } });
$dump.a11 = $dump.Ka11.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
