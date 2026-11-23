"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 13);
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
$dump.p1 = $djs.resume(12, "later", ["g"]);
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
var v = $dump.n2;
const $djs_t0 = $dump.n2;
const { v0: a = (yield), [("v1")]: [c = yield "need c"] = [] } = { v0: undefined, get v1() { return $djs_t0["b"]; } };
return a + c;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: defaults, at: "yield#1", segs: [71,34,1,94,57,25,178,92,13], scaf: [["site",38,76],["k",0,84],["k",0,65]], cmap: [["pattern.source@32",0,"$djs_t0"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.Ka5 = async function () {
const g = $dump.a3;
const seen = ["need a"];
(await $dump.p1);
seen.push(g.next("A").value);
await later("g2");
seen.push(g.next("C").value);
print("defaults", seen.join(" "));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#4", segs: [83,233,29,113,265,18,132,286,29,162,318,34], scaf: [["site",213,66]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
