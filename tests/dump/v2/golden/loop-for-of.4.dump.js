"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(new Map()[Symbol.iterator]())["next"];
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.p1 = $djs.resume(4, "later", [2]);
var gen = function* gen() { yield "g1"; yield "g2"; };
$dump.n2 = [1, 2, "a1"];
$dump.i3 = $djs.advance(Map.prototype.entries.call(new Map([["a", 1], ["b", 2]])), 2);
$dump.a4_0 = Array.prototype.push;
$dump.Ka4 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const out = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i3, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const k = "b";
const v = 2;

const $djs_t0 = $dump.a4_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [("b" + (await $dump.p1))]);
} } else { const [k, v] = $djs_v1; out.push(k + (await later(v))); } } }
for (const g of gen()) out.push(await later(g));
print(out.join());
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#2", segs: [332,101,12,356,148,31,394,182,48,443,233,18], scaf: [["for-of",96,104],["k",0,267],["k",0,274],["k",0,283],["site",162,301],["tmp",0,293,"call.arg@157","array.element"],["k",0,292],["site",179,322]], cmap: [["call.callee@148",0,"$djs_t0"],["call.this@148",0,"$djs_t1"],["binary.left@157",1,"b"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
