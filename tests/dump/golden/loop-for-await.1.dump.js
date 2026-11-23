"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
$dump.p1 = $djs.resume(1, "later", ["s2"]);
var agen = async function* agen() {
  yield await later("a1");
  yield "a2";
};
$dump.n2 = ["s1", $dump.p1];
$dump.i3 = $djs.advance(Array.prototype.values.call($dump.n2), 2);
$dump.p4 = Promise.prototype.then.call($dump.p1, (v) => ({ value: v, done: false }), $djs.closeThrow($dump.i3));
$dump.i5 = $djs.fromSync($dump.i3, $dump.b0);
$dump.Ka6 = async function () {
const $djs_N = $djs.N;
const out = ["s1"];
{ const $djs_i1 = $dump.i5, $djs_n1 = $dump.i5.next; for await (const x of $djs_N($djs_i1, $djs_n1, $dump.p4)) out.push(x); }
for await (const x of agen()) out.push(await later(x + "!"));
print(out.join());
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, at: "for-await.next#1", segs: [116,44,11,127,55,7,174,87,12,189,102,61,251,166,18], scaf: [["for-await",44,116,0,"$djs_i1","$djs_n1"],["site",44,116]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
