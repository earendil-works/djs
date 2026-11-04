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
$dump.p1 = $djs.resume(1, "later", [2]);
$dump.p2 = $djs.resume(2, "later", [3]);
$dump.n3 = new Error("e");
$dump.p4 = $djs.resume(3, "later", [$dump.n3, true]);
$dump.p5 = $djs.resume(4, "later", [4]);
$dump.n6 = new Error("x");
$dump.p7 = $djs.resume(5, "later", [$dump.n6, true]);
$dump.p8 = $djs.resume(6, "later", ["ok"]);
$dump.p9 = $djs.resume(7, "later", ["fast"]);
$dump.p10 = Promise.all([1, $dump.p1, $dump.p2]);
$dump.p11 = Promise.allSettled([$dump.p4, $dump.p5]);
$dump.p12 = Promise.any([$dump.p7, $dump.p8]);
$dump.p13 = new Promise((res, rej) => { $dump.r14 = [res, rej]; });
$dump.p15 = Promise.prototype.then.call($dump.p9, $dump.r14[0], $dump.r14[1]);
$dump.a16_0 = print;
$dump.a16_1 = JSON.stringify;
$dump.a16_2 = JSON;
$dump.Ka16 = async function () {
const $djs_R = $djs.R;
const all = $dump.p10;
const settled = $dump.p11;
const any = $dump.p12;
const race = $dump.p13;
const $djs_t0 = $dump.a16_0, $djs_t1 = $dump.a16_1, $djs_t2 = $dump.a16_2;
$djs_R($djs_t0, undefined, [$djs_R($djs_t1, $djs_t2, [(await $dump.p10)]), JSON.stringify(await settled), await any, await race]);
};
Object.defineProperty($dump.Ka16, "$djs", { value: { f: main, segs: [290,330,29,321,361,9,332,372,10], scaf: [["k",0,215],["k",0,222],["k",0,231],["k",0,243],["k",0,250],["k",0,259],["site",318,270],["tmp",0,269,"call.arg@318","array.element"],["k",0,268],["tmp",0,243,"call.arg@303","array.element"],["tmp",0,290,"call.arg@330","array.element"],["tmp",0,321,"call.arg@361","array.element"],["tmp",0,332,"call.arg@372","array.element"],["k",0,242]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.callee@303",0,"$djs_t1"],["call.this@303",0,"$djs_t2"]] } });
$dump.a16 = $dump.Ka16.call(undefined);
Object.defineProperty($dump.n3, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n6, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
