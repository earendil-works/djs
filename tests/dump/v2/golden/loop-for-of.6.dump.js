"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 7);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.p1 = $djs.resume(6, "later", ["g2"]);
var gen = function* gen() { yield "g1"; yield "g2"; };
$dump.n2 = [1, 2, "a1", "b2", "g1"];
$dump.n4 = { __proto__: $dump.b0 };
$dump.Ka3 = function* () {

(yield);
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: gen, at: "yield#2", scaf: [["site",30,17]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.a5_0 = Array.prototype.push;
$dump.Ka5 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const out = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.a3, $dump.b1)) { if ($djs_f1) { $djs_f1 = false; {
const g = "g2";

const $djs_t0 = $dump.a5_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [(await $dump.p1)]);
} } else { const g = $djs_v1; out.push(await later(g)); } } }
print(out.join());
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#3", segs: [312,187,7,331,205,25,363,233,18], scaf: [["for-of",182,104],["k",0,255],["k",0,262],["k",0,271],["site",214,282],["tmp",0,281,"call.arg@214","array.element"],["k",0,280],["site",230,302]], cmap: [["call.callee@205",0,"$djs_t0"],["call.this@205",0,"$djs_t1"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
