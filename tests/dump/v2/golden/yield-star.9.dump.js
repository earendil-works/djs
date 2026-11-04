"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 11);
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
$dump.p1 = $djs.resume(10, "later", ["x1"]);
var inner = function* inner(tag) {
  const got = yield tag + "1";
  yield tag + "2:" + got;
  return tag + " done";
};
var ainner = async function* ainner() {
  yield await later("x1");
  try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
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
$dump.n4 = { __proto__: $dump.b0 };
$dump.ga3 = () => {

};
$dump.Ka3 = async function* () {

$dump.ga3();
(yield (await $dump.p1));
try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: ainner, segs: [36,29,5,61,56,67], scaf: [["site",35,43]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.n7 = { __proto__: $dump.b0 };
$dump.ga6 = () => {

};
$dump.Ka6 = async function* () {
const $djs_Y = $djs.Y;

$dump.ga6();
const $djs_t0 = $dump.a3, $djs_t1 = $dump.b1;
(yield* $djs_Y($djs_t0, $djs_t1, true, $dump.p2));
yield* ["s1", later("s2")];
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: aouter, at: "yield*.await#5", segs: [105,29,6,155,48,27], scaf: [["site",29,105],["site",44,152]], cmap: [["yield*.iterator@29",0,"$djs_t0"],["yield*.next@29",0,"$djs_t1"]] } });
$dump.Ka6.prototype = $dump.n7;
$dump.a6 = $dump.Ka6.call(undefined);
$dump.p5 = $dump.a6.next();
$dump.a8 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), outer.prototype);
$dump.n9 = ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1", "x2", "s1", "s2"];
$dump.a10 = Object.setPrototypeOf(((g) => (g.return(), g))((async function* () {})()), aouter.prototype);
$dump.a11_0 = Array.prototype.push;
$dump.Ka11 = async function () {
const $djs_R = $djs.R;
const g = $dump.a8;
const seen = $dump.n9;
const ag = $dump.a10;
const ag2 = $dump.a6;
const $djs_t0 = $dump.a11_0, $djs_t1 = $dump.n9;
$djs_R($djs_t0, $djs_t1, [(await $dump.p5).value, (await ag2.next()).value]);
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
Object.defineProperty($dump.Ka11, "$djs", { value: { f: main, at: "await#4", segs: [221,366,6,229,374,24,257,403,15,273,421,53,327,477,19,347,499,10,358,512,96,455,611,22], scaf: [["k",0,179],["k",0,186],["k",0,195],["site",349,206],["tmp",0,205,"call.arg@348","array.element"],["tmp",0,229,"call.arg@374","array.element"],["k",0,204]], cmap: [["call.callee@338",0,"$djs_t0"],["call.this@338",0,"$djs_t1"]] } });
$dump.a11 = $dump.Ka11.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
