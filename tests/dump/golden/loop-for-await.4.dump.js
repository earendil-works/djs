"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(async function* () {}).prototype["next"];
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
$dump.p1 = $djs.resume(4, "later", ["a2!"]);
var agen = async function* agen() {
  yield await later("a1");
  yield "a2";
};
$dump.n2 = ["s1", "s2", "a1!"];
$dump.n4 = { __proto__: $dump.b0 };
$dump.Ka3 = async function* () {

(yield);
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: agen, at: "yield#2", scaf: [["site",54,23]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.a5_0 = Array.prototype.push;
$dump.Ka5 = async function () {
const $djs_R = $djs.R, $djs_N = $djs.N, $djs_X = $djs.X, $djs_O = $djs.O;
const out = $dump.n2;
{ const $djs_i1 = $dump.a3, $djs_n1 = $dump.b1; let $djs_s1 = 2;
try { for (let $djs_o1 = 0; $djs_o1 < 1; $djs_o1++, $djs_s1 = 1) {
const x = "a2";
const $djs_t0 = $dump.a5_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [(await $dump.p1)]);
} $djs_s1 += 10; } catch ($djs_e1) { $djs_s1 = 20; await $djs_X($djs_i1); throw $djs_e1; } finally { if ($djs_s1 < 10) { const $djs_m1 = $djs_i1.return; if ($djs_m1 != null) $djs_O(await $djs_R($djs_m1, $djs_i1, [])); } }
if ($djs_s1 === 12) { const $djs_m1 = $djs_i1.return; if ($djs_m1 != null) $djs_O(await $djs_R($djs_m1, $djs_i1, [])); } else { for await (const x of $djs_N($djs_i1, $djs_n1)) out.push(await later(x + "!")); } }
print(out.join());
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, segs: [708,102,11,719,113,7,756,132,31,792,166,18], scaf: [["for-await",102,708,0,"$djs_i1","$djs_n1"],["k",0,312],["k",0,319],["k",0,328],["site",141,339],["tmp",0,338,"call.arg@141","array.element"],["k",0,337]], cmap: [["call.callee@132",0,"$djs_t0"],["call.this@132",0,"$djs_t1"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
