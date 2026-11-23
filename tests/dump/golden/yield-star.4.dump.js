"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(async function* () {}).prototype["next"];
async function main() {
  const g = outer();
  const seen = [g.next().value];
  await later(0);
  seen.push(g.next("v").value, g.next().value);
  await later(1);
  seen.push(g.next().value, JSON.stringify(g.next()));
  const ag = aouter();
  for await (const v of ag) {
    seen.push(v);
    await later(v);
  }
  const ag2 = aouter();
  seen.push((await ag2.next()).value, (await ag2.next()).value);
  await later(2);
  seen.push(JSON.stringify(await ag2.return("early")));
  const g2 = outer();
  g2.next();
  try {
    g2.throw(new Error("boom"));
  } catch (e) {
    seen.push("caught " + e.message);
  }
  print(seen.join(" "));
}
$dump.p1 = $djs.resume(4, "later", ["x1"]);
var inner = function* inner(tag) {
  const got = yield tag + "1";
  yield tag + "2:" + got;
  return tag + " done";
};
var outer = function* outer() {
  const r = yield* inner("i");
  yield* ["a1", "a2"];
  return r;
};
var aouter = async function* aouter() {
  yield* ainner();
  yield* ["s1", later("s2")];
};
var ainner = async function* ainner() {
  yield await later("x1");
  try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
};
$dump.a2 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), outer.prototype);
$dump.n4 = { __proto__: $dump.b0 };
$dump.Ka3 = async function* () {

(yield);
try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: ainner, at: "yield#1", segs: [31,56,67], scaf: [["site",29,23]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.n6 = { __proto__: $dump.b0 };
$dump.Ka5 = async function* () {
const $djs_Y = $djs.Y;

const $djs_t0 = $dump.a3, $djs_t1 = $dump.b1;
(yield* $djs_Y($djs_t0, $djs_t1, true));
yield* ["s1", later("s2")];
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: aouter, at: "yield*#1", segs: [92,29,6,132,48,27], scaf: [["site",29,92],["site",44,129]], cmap: [["yield*.iterator@29",0,"$djs_t0"],["yield*.next@29",0,"$djs_t1"]] } });
$dump.Ka5.prototype = $dump.n6;
$dump.a5 = $dump.Ka5.call(undefined);
$dump.a5.next();
$dump.Ka7 = async function () {
const $djs_R = $djs.R, $djs_N = $djs.N, $djs_X = $djs.X, $djs_O = $djs.O;
const g = $dump.a2;
const seen = ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1"];
const ag = $dump.a5;
{ const $djs_i1 = $dump.a5, $djs_n1 = $dump.b1; let $djs_s1 = 2;
try { for (let $djs_o1 = 0; $djs_o1 < 1; $djs_o1++, $djs_s1 = 1) {
const v = "x1";
{

(await $dump.p1);
}
} $djs_s1 += 10; } catch ($djs_e1) { $djs_s1 = 20; await $djs_X($djs_i1); throw $djs_e1; } finally { if ($djs_s1 < 10) { const $djs_m1 = $djs_i1.return; if ($djs_m1 != null) $djs_O(await $djs_R($djs_m1, $djs_i1, [])); } }
if ($djs_s1 === 12) { const $djs_m1 = $djs_i1.return; if ($djs_m1 != null) $djs_O(await $djs_R($djs_m1, $djs_i1, [])); } else { for await (const v of $djs_N($djs_i1, $djs_n1)) {
    seen.push(v);
    await later(v);
  } } }
const ag2 = aouter();
seen.push((await ag2.next()).value, (await ag2.next()).value);
await later(2);
seen.push(JSON.stringify(await ag2.return("early")));
const g2 = outer();
g2.next();
try {
    g2.throw(new Error("boom"));
  } catch (e) {
    seen.push("caught " + e.message);
  }
print(seen.join(" "));
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: main, at: "await#3", segs: [743,242,11,754,253,7,791,268,43,839,314,21,861,338,62,924,403,15,940,421,53,994,477,19,1014,499,10,1025,512,96,1122,611,22], scaf: [["for-await",242,743,0,"$djs_i1","$djs_n1"],["site",292,374],["site",311,392]] } });
$dump.a7 = $dump.Ka7.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
