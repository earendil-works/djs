"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 14);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
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
$dump.p1 = $djs.resume(13, "later", ["g2"]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
$dump.n2 = {};
$dump.n4 = { __proto__: $dump.b0 };
$dump.Ka3 = function* () {
const $djs_E = $djs.E;
var v = $dump.n2;
const a = "A", { v0: [c = (yield)] = $djs_E() } = { v0: undefined };
return a + c;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: defaults, at: "yield#2", segs: [78,58,1,125,92,13], scaf: [["site",62,83],["pattern",57,77],["k",0,71]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.Ka5 = async function () {
const g = $dump.a3;
const seen = ["need a", "need c"];
(await $dump.p1);
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#5", segs: [93,286,29,123,318,34], scaf: [["site",265,76]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
