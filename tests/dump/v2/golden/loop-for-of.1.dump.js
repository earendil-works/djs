"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.p1 = $djs.resume(1, "later", [1]);
var gen = function* gen() { yield "g1"; yield "g2"; };
$dump.n2 = [];
$dump.i3 = $djs.advance(Array.prototype.values.call([1, 2]), 1);
$dump.a4_0 = Array.prototype.push;
$dump.Ka4 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const out = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i3, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const x = 1;

const $djs_t0 = $dump.a4_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [(await $dump.p1)]);
} } else { const x = $djs_v1; out.push(await later(x)); } } }
for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
for (const g of gen()) out.push(await later(g));
print(out.join());
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, segs: [309,49,7,328,68,25,360,96,83,444,182,48,493,233,18], scaf: [["for-of",44,104],["k",0,252],["k",0,259],["k",0,268],["site",77,279],["tmp",0,278,"call.arg@77","array.element"],["k",0,277],["site",93,299]], cmap: [["call.callee@68",0,"$djs_t0"],["call.this@68",0,"$djs_t1"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
