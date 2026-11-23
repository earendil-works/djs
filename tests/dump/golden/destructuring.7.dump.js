"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 10);
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
$dump.p1 = $djs.resume(9, "later", [2]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
$dump.n2 = { list: ["d"], p: 1 };
$dump.n3 = {};
$dump.Ka4 = async function () {
const t = $dump.n2;
let n = 1;
const r = [1];
const $djs_t0 = $dump.n3, $djs_t1 = $dump.n2, $djs_t2 = $dump.n3;
(({ v0: $djs_t1[("q" + (await $dump.p1))] = await later("e") } = { get v0() { return $djs_t0["q"]; } }), $djs_t2);
return JSON.stringify(t) + " " + n + " " + r.length;
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: assigns, at: "await#3", segs: [176,175,16,247,203,52], scaf: [["tmp",0,140,"assign.object@147","assign.object"],["tmp",0,140,"assign.key@147","assign.key"],["site",156,156],["k",0,134]], cmap: [["pattern.source@142",0,"$djs_t0"],["member.object@147",0,"$djs_t1"],["binary.left@149",1,"q"],["assign.value@142",0,"$djs_t2"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a5_0 = print;
$dump.Ka5 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a5_0;
$djs_R($djs_t0, undefined, ["assigns", (await $dump.a4)]);
const g = defaults({});
const seen = [g.next().value];
await later("g");
seen.push(g.next("A").value);
await later("g2");
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#3", segs: [131,154,23,155,180,30,186,213,17,204,233,29,234,265,18,253,286,29,283,318,34], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@123","array.element"],["site",134,112],["tmp",0,111,"call.arg@134","array.element"],["k",0,99]], cmap: [["call.callee@117",0,"$djs_t0"],["call.this@117",1,undefined],["call.arg@123",1,"assigns"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
