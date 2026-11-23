"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 8);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
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
$dump.p1 = $djs.resume(7, "later", [0]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
$dump.n3 = [];
$dump.n2 = { list: $dump.n3, p: 1 };
$dump.n4 = [1];
$dump.i5 = $djs.advance(Array.prototype.values.call($dump.n4), 1);
$dump.Ka6 = async function () {
const $djs_I = $djs.I;
const t = $dump.n2;
let n = undefined;
const $djs_t0 = $dump.i5, $djs_t1 = $dump.b0, $djs_t2 = $dump.n3, $djs_t3 = $dump.n4;
const r = (([$djs_t2[(await $dump.p1)] = await later("d"), n = 1] = $djs_I($djs_t0, $djs_t1)), $djs_t3);
({ q: t["q" + (await later(2))] = await later("e") } = {});
return JSON.stringify(t) + " " + n + " " + r.length;
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: assigns, segs: [174,70,1,209,106,16,227,124,5,273,141,59,333,203,52], scaf: [["tmp",0,181,"assign.object@81","assign.object"],["tmp",0,181,"assign.key@81","assign.key"],["site",88,190],["pattern",75,180]], cmap: [["pattern.iterator@75",0,"$djs_t0"],["pattern.next@75",0,"$djs_t1"],["member.object@81",0,"$djs_t2"],["assign.value@75",0,"$djs_t3"]] } });
$dump.a6 = $dump.Ka6.call(undefined);
$dump.a7_0 = print;
$dump.Ka7 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a7_0;
$djs_R($djs_t0, undefined, ["assigns", (await $dump.a6)]);
const g = defaults({});
const seen = [g.next().value];
await later("g");
seen.push(g.next("A").value);
await later("g2");
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: main, at: "await#3", segs: [131,154,23,155,180,30,186,213,17,204,233,29,234,265,18,253,286,29,283,318,34], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@123","array.element"],["site",134,112],["tmp",0,111,"call.arg@134","array.element"],["k",0,99]], cmap: [["call.callee@117",0,"$djs_t0"],["call.this@117",1,undefined],["call.arg@123",1,"assigns"]] } });
$dump.a7 = $dump.Ka7.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
