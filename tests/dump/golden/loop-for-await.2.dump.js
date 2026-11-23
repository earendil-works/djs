"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(async function* () {}).prototype["next"];
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
$dump.p1 = $djs.resume(2, "later", ["a1"]);
var agen = async function* agen() {
  yield await later("a1");
  yield "a2";
};
$dump.n4 = { __proto__: $dump.b0 };
$dump.ga3 = () => {

};
$dump.Ka3 = async function* () {

$dump.ga3();
(yield (await $dump.p1));
yield "a2";
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: agen, segs: [36,27,5,61,54,11], scaf: [["site",33,43]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.Ka5 = async function () {
const $djs_N = $djs.N;
const out = ["s1", "s2"];
{ const $djs_i1 = $dump.a3, $djs_n1 = $dump.b1; for await (const x of $djs_N($djs_i1, $djs_n1, $dump.p2)) out.push(await later(x + "!")); }
print(out.join());
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "for-await.next#2", segs: [117,102,11,128,113,7,175,132,31,209,166,18], scaf: [["for-await",102,117,0,"$djs_i1","$djs_n1"],["site",102,117]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
