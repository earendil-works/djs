"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype;
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
$dump.p1 = $djs.resume(2, "later", [1]);
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
$dump.i2 = $djs.advance(Array.prototype.values.call(["a1", "a2"]), 1);
$dump.n4 = { __proto__: $dump.b1 };
$dump.Ka3 = function* () {
const $djs_Y = $djs.Y;
const r = "i done";
const $djs_t0 = $dump.i2, $djs_t1 = $dump.b0;
(yield* $djs_Y($djs_t0, $djs_t1, false));
return r;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: outer, at: "yield*#2", segs: [105,53,6,146,76,9], scaf: [["site",53,105],["site",72,143]], cmap: [["yield*.iterator@53",0,"$djs_t0"],["yield*.next@53",0,"$djs_t1"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.Ka5 = async function () {
const g = $dump.a3;
const seen = ["i1", "i2:v", "a1"];
(await $dump.p1);
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
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#2", segs: [93,164,52,146,219,20,167,242,69,237,314,21,259,338,62,322,403,15,338,421,53,392,477,19,412,499,10,423,512,96,520,611,22], scaf: [["site",146,76]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
