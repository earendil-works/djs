"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 8);
globalThis.$dump = {};
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p1 = $djs.resume(2, "later", [3]);
$dump.n2 = new Error("e");
$dump.p3 = $djs.resume(3, "later", [$dump.n2, true]);
$dump.p4 = $djs.resume(4, "later", [4]);
$dump.n5 = new Error("x");
$dump.p6 = $djs.resume(5, "later", [$dump.n5, true]);
$dump.p7 = $djs.resume(6, "later", ["ok"]);
$dump.p8 = $djs.resume(7, "later", ["fast"]);
$dump.p9 = Promise.all([1, 2, $dump.p1]);
$dump.p10 = Promise.allSettled([$dump.p3, $dump.p4]);
$dump.p11 = Promise.any([$dump.p6, $dump.p7]);
$dump.p12 = new Promise((res, rej) => { $dump.r13 = [res, rej]; });
$dump.p14 = Promise.prototype.then.call($dump.p8, $dump.r13[0], $dump.r13[1]);
$dump.a15_0 = print;
$dump.a15_1 = JSON.stringify;
$dump.a15_2 = JSON;
$dump.Ka15 = async function () {
const $djs_R = $djs.R;
const all = $dump.p9;
const settled = $dump.p10;
const any = $dump.p11;
const race = $dump.p12;
const $djs_t0 = $dump.a15_0, $djs_t1 = $dump.a15_1, $djs_t2 = $dump.a15_2;
$djs_R($djs_t0, undefined, [$djs_R($djs_t1, $djs_t2, [(await $dump.p9)]), JSON.stringify(await settled), await any, await race]);
};
Object.defineProperty($dump.Ka15, "$djs", { value: { f: main, segs: [288,330,29,319,361,9,330,372,10], scaf: [["k",0,214],["k",0,221],["k",0,230],["k",0,242],["k",0,249],["k",0,258],["site",318,269],["tmp",0,268,"call.arg@318","array.element"],["k",0,267],["tmp",0,242,"call.arg@303","array.element"],["tmp",0,288,"call.arg@330","array.element"],["tmp",0,319,"call.arg@361","array.element"],["tmp",0,330,"call.arg@372","array.element"],["k",0,241]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.callee@303",0,"$djs_t1"],["call.this@303",0,"$djs_t2"]] } });
$dump.a15 = $dump.Ka15.call(undefined);
Object.defineProperty($dump.n2, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n5, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
