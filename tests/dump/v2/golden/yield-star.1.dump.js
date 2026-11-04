"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
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
$dump.p1 = $djs.resume(1, "later", [0]);
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
$dump.n3 = { __proto__: $dump.b0 };
$dump.Ka2 = function* () {
var tag = "i";
const got = (yield);
yield tag + "2:" + got;
return tag + " done";
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: inner, at: "yield#1", segs: [36,31,3,51,56,23,75,82,21], scaf: [["site",37,43]] } });
$dump.Ka2.prototype = $dump.n3;
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a2.next();
$dump.n5 = { __proto__: $dump.b0 };
$dump.Ka4 = function* () {
const $djs_Y = $djs.Y;

const $djs_t0 = $dump.a2, $djs_t1 = $dump.b1;
const r = (yield* $djs_Y($djs_t0, $djs_t1, false));
yield* ["a1", "a2"];
return r;
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: outer, at: "yield*#1", segs: [91,28,1,96,32,6,137,53,20,158,76,9], scaf: [["site",32,96],["site",49,134]], cmap: [["yield*.iterator@32",0,"$djs_t0"],["yield*.next@32",0,"$djs_t1"]] } });
$dump.Ka4.prototype = $dump.n5;
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a4.next();
$dump.Ka6 = async function () {
const g = $dump.a4;
const seen = ["i1"];
(await $dump.p1);
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
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, segs: [79,98,45,125,146,15,141,164,52,194,219,20,215,242,69,285,314,21,307,338,62,370,403,15,386,421,53,440,477,19,460,499,10,471,512,96,568,611,22], scaf: [["site",80,62]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
