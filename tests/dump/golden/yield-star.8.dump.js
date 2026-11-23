"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 10);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.b1 = Object.getPrototypeOf(async function* () {}).prototype;
$dump.b2 = Object.getPrototypeOf(async function* () {}).prototype["next"];
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
$dump.p1 = $djs.resume(9, "later", ["s2"]);
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
$dump.n3 = ["s1", Promise.resolve("s2")];
$dump.i4 = $djs.advance(Array.prototype.values.call($dump.n3), 2);
$dump.i5 = $djs.fromSync($dump.i4, $dump.b0);
$dump.n7 = { __proto__: $dump.b1 };
$dump.Ka6 = async function* () {
const $djs_Y = $djs.Y;

const $djs_t0 = $dump.i5, $djs_t1 = $dump.i5.next;
(yield* $djs_Y($djs_t0, $djs_t1, true));
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: aouter, at: "yield*#2", segs: [97,48,6], scaf: [["site",48,97],["site",74,134]], cmap: [["yield*.iterator@48",0,"$djs_t0"],["yield*.next@48",0,"$djs_t1"]] } });
$dump.Ka6.prototype = $dump.n7;
$dump.a6 = $dump.Ka6.call(undefined);
$dump.a6.next();
$dump.Ka8 = async function () {
const $djs_R = $djs.R, $djs_N = $djs.N, $djs_X = $djs.X, $djs_O = $djs.O;
const g = $dump.a2;
const seen = ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1", "x2", "s1", "s2"];
const ag = $dump.a6;
{ const $djs_i1 = $dump.a6, $djs_n1 = $dump.b2; let $djs_s1 = 2;
try { for (let $djs_o1 = 0; $djs_o1 < 1; $djs_o1++, $djs_s1 = 1) {
const v = "s2";
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
Object.defineProperty($dump.Ka8, "$djs", { value: { f: main, at: "await#3", segs: [761,242,11,772,253,7,809,268,43,857,314,21,879,338,62,942,403,15,958,421,53,1012,477,19,1032,499,10,1043,512,96,1140,611,22], scaf: [["for-await",242,761,0,"$djs_i1","$djs_n1"],["site",292,392],["site",311,410]] } });
$dump.a8 = $dump.Ka8.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
