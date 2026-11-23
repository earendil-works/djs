"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(async function* () {}).prototype;
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
$dump.p1 = $djs.resume(2, "later", [0]);
var g = function* g(a) { const x = yield a; yield x + 1; };
var ag = async function* ag() { yield await later("q1"); yield "q2"; };
$dump.n3 = { __proto__: $dump.b0 };
$dump.Ka2 = function* () {
var a = 1;
const x = (yield);
yield x + 1;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: g, at: "yield#1", segs: [32,23,1,45,36,12], scaf: [["site",27,37]] } });
$dump.Ka2.prototype = $dump.n3;
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a2.next();
$dump.a4 = Reflect.apply(g, undefined, [5]);
$dump.a5 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), g.prototype);
$dump.n7 = { __proto__: $dump.b1 };
$dump.Ka6 = async function* () {

(yield);
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: ag, at: "yield#2", scaf: [["site",48,23]] } });
$dump.Ka6.prototype = $dump.n7;
$dump.a6 = $dump.Ka6.call(undefined);
$dump.a6.next();
$dump.n8 = Promise.resolve({ value: "q1", done: false });
$dump.n9 = Promise.resolve({ value: "q2", done: false });
$dump.Ka10 = async function () {
const mid = $dump.a2;
const fresh = $dump.a4;
const done = $dump.a5;
const it = $dump.a6;
const r1 = $dump.n8;
const r2 = $dump.n9;
(await $dump.p1);
print(mid.next(10).value, fresh.next().value, done.next().done);
print((await r1).value, (await r2).value);
};
Object.defineProperty($dump.Ka10, "$djs", { value: { f: main, segs: [170,217,64,235,284,42], scaf: [["site",199,153]] } });
$dump.a10 = $dump.Ka10.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
