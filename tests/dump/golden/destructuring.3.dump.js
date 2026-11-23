"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
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
$dump.p1 = $djs.resume(3, "later", ["b"]);
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
const $djs_S = $djs.S;
var o = $dump.n2;
const $djs_t0 = $dump.n2, $djs_t1 = ["a", "k"];
const a = "a", { v0: { v0: b = (await $dump.p1) } = { v0: undefined }, v1: rest } = { v0: undefined, get v1() { return $djs_S($djs_t0, $djs_t1); } };
return [a, b, JSON.stringify(rest)].join(" ");
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: objects, at: "await#3", segs: [136,82,1,184,114,4,259,128,46], scaf: [["site",86,141],["k",0,130],["k",0,124]], cmap: [["pattern.source@36",0,"$djs_t0"],["pattern.excluded@36",0,"$djs_t1"],["pattern.key@60",1,"k"]] } });
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
