"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
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
$dump.p1 = $djs.resume(1, "later", ["a"]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
$dump.n2 = { k: {}, extra: 1 };
$dump.Ka3 = async function () {
const $djs_P = $djs.P, $djs_S = $djs.S;
var o = $dump.n2;
const $djs_t0 = $dump.n2, $djs_t1 = ["a"];
let $djs_t2;
const { v0: a = (await $dump.p1), [($djs_t2 = $djs_P(await later("k")), $djs_t1[$djs_t1.length] = $djs_t2, "v1")]: { b = await later("b") } = {}, v2: rest } = { v0: undefined, get v1() { return $djs_t0[$djs_t2]; }, get v2() { return $djs_S($djs_t0, $djs_t1); } };
return [a, b, JSON.stringify(rest)].join(" ");
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: objects, segs: [146,38,1,187,61,16,249,80,29,284,114,4,398,128,46], scaf: [["site",42,151],["k",0,180],["k",0,168],["k",0,140]], cmap: [["pattern.source@36",0,"$djs_t0"],["pattern.excluded@36",0,"$djs_t1"],["pattern.key@60",0,"$djs_t2"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, ["objects", (await $dump.a3)]);
print("arrays", await arrays());
print("assigns", await assigns());
const g = defaults({});
const seen = [g.next().value];
await later("g");
seen.push(g.next("A").value);
await later("g2");
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, segs: [131,82,32,164,117,34,199,154,23,223,180,30,254,213,17,272,233,29,302,265,18,321,286,29,351,318,34], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@32","array.element"],["site",43,112],["tmp",0,111,"call.arg@43","array.element"],["k",0,99]], cmap: [["call.callee@26",0,"$djs_t0"],["call.this@26",1,undefined],["call.arg@32",1,"objects"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
