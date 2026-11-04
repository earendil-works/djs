"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 9);
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
$dump.p1 = $djs.resume(8, "later", ["d"]);
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
$dump.Ka4 = async function () {
const $djs_E = $djs.E;
const t = $dump.n2;
let n = undefined;
const $djs_t0 = $dump.n3, $djs_t1 = [1];
const r = (([$djs_t0[0] = (await $dump.p1), n = 1] = $djs_E()), $djs_t1);
({ q: t["q" + (await later(2))] = await later("e") } = {});
return JSON.stringify(t) + " " + n + " " + r.length;
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: assigns, at: "await#2", segs: [129,70,1,167,124,5,197,141,59,257,203,52], scaf: [["site",106,150],["pattern",75,135]], cmap: [["assign.object@81",0,"$djs_t0"],["assign.key@81",1,0],["assign.value@75",0,"$djs_t1"]] } });
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
