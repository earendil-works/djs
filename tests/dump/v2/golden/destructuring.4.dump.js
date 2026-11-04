"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 6);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
async function objects(o) {
  const { a = await later("a"), [await later("k")]: { b = await later("b") } = {}, ...rest } = o;
  return [a, b, JSON.stringify(rest)].join(" ");
}
async function arrays() {
  const [x, y = await later("y"), [z = await later("z")] = [], ...more] = letters();
  return [x, y, z, more.length].join(" ");
}
async function assigns() {
  const t = { list: [] };
  let n;
  const r = ([t.p, t.list[await later(0)] = await later("d"), n = 1] = [1]);
  ({ q: t["q" + (await later(2))] = await later("e") } = {});
  return JSON.stringify(t) + " " + n + " " + r.length;
}
async function main() {
  print("objects", await objects({ k: {}, extra: 1 }));
  print("arrays", await arrays());
  print("assigns", await assigns());
  const g = defaults({});
  const seen = [g.next().value];
  await later("g");
  seen.push(g.next("A").value);
  await later("g2");
  seen.push(g.next("C").value);
  print("defaults", seen.join(" "));
}
$dump.p1 = $djs.resume(5, "later", ["y"]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
$dump.n3 = { __proto__: $dump.b0 };
$dump.Ka2 = function* () {

(yield);
yield "c";
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: letters, at: "yield#2", segs: [25,56,10], scaf: [["site",37,17]] } });
$dump.Ka2.prototype = $dump.n3;
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a2.next();
$dump.Ka4 = async function () {
const $djs_L = $djs.L;

const $djs_t0 = $dump.a2, $djs_t1 = $dump.b1;
const x = "a", [y = (await $dump.p1), [z = await later("z")] = [], ...more] = $djs_L($djs_t0, $djs_t1);
return [x, y, z, more.length].join(" ");
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: arrays, segs: [106,38,1,128,60,27,157,89,7,194,113,40], scaf: [["site",42,111],["pattern",34,105]], cmap: [["pattern.iterator@34",0,"$djs_t0"],["pattern.next@34",0,"$djs_t1"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a5_0 = print;
$dump.Ka5 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a5_0;
$djs_R($djs_t0, undefined, ["arrays", (await $dump.a4)]);
print("assigns", await assigns());
const g = defaults({});
const seen = [g.next().value];
await later("g");
seen.push(g.next("A").value);
await later("g2");
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#2", segs: [130,117,34,165,154,23,189,180,30,220,213,17,238,233,29,268,265,18,287,286,29,317,318,34], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@88","array.element"],["site",98,111],["tmp",0,110,"call.arg@98","array.element"],["k",0,99]], cmap: [["call.callee@82",0,"$djs_t0"],["call.this@82",1,undefined],["call.arg@88",1,"arrays"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
