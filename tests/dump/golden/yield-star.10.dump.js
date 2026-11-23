"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 12);
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
$dump.p1 = $djs.resume(11, "later", [2]);
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
$dump.a3 = Object.setPrototypeOf(((g) => (g.return(), g))((async function* () {})()), aouter.prototype);
$dump.n5 = { __proto__: $dump.b0 };
$dump.Ka4 = async function* () {

try {

(yield);
} finally {
    print("ainner closed");
  }
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: ainner, at: "yield#2", segs: [48,90,33], scaf: [["try",56,22],["site",66,30],["site",81,39]] } });
$dump.Ka4.prototype = $dump.n5;
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a4.next();
$dump.n7 = { __proto__: $dump.b0 };
$dump.Ka6 = async function* () {
const $djs_Y = $djs.Y;

const $djs_t0 = $dump.a4, $djs_t1 = $dump.b1;
(yield* $djs_Y($djs_t0, $djs_t1, true));
yield* ["s1", later("s2")];
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: aouter, at: "yield*#1", segs: [92,29,6,132,48,27], scaf: [["site",29,92],["site",44,129]], cmap: [["yield*.iterator@29",0,"$djs_t0"],["yield*.next@29",0,"$djs_t1"]] } });
$dump.Ka6.prototype = $dump.n7;
$dump.a6 = $dump.Ka6.call(undefined);
$dump.a6.next();
$dump.Ka8 = async function () {
const g = $dump.a2;
const seen = ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1", "x2", "s1", "s2", "x1", "x2"];
const ag = $dump.a3;
const ag2 = $dump.a6;
(await $dump.p1);
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
Object.defineProperty($dump.Ka8, "$djs", { value: { f: main, at: "await#6", segs: [218,421,53,272,477,19,292,499,10,303,512,96,400,611,22], scaf: [["site",403,201]] } });
$dump.a8 = $dump.Ka8.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
