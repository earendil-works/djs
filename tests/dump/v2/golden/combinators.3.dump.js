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
$dump.n1 = new Error("e");
$dump.p2 = $djs.resume(3, "later", [$dump.n1, true]);
$dump.p3 = $djs.resume(4, "later", [4]);
$dump.n4 = new Error("x");
$dump.p5 = $djs.resume(5, "later", [$dump.n4, true]);
$dump.p6 = $djs.resume(6, "later", ["ok"]);
$dump.p7 = $djs.resume(7, "later", ["fast"]);
$dump.p8 = Promise.allSettled([$dump.p2, $dump.p3]);
$dump.p9 = Promise.any([$dump.p5, $dump.p6]);
$dump.p10 = new Promise((res, rej) => { $dump.r11 = [res, rej]; });
$dump.p12 = Promise.prototype.then.call($dump.p7, $dump.r11[0], $dump.r11[1]);
$dump.n13 = Promise.resolve([1, 2, 3]);
$dump.a14_0 = print;
$dump.a14_1 = JSON.stringify;
$dump.a14_2 = JSON;
$dump.Ka14 = async function () {
const $djs_R = $djs.R;
const all = $dump.n13;
const settled = $dump.p8;
const any = $dump.p9;
const race = $dump.p10;
const $djs_t0 = $dump.a14_0, $djs_t1 = $dump.a14_1, $djs_t2 = $dump.a14_2;
$djs_R($djs_t0, undefined, ["[1,2,3]", $djs_R($djs_t1, $djs_t2, [(await $dump.p8)]), await any, await race]);
};
Object.defineProperty($dump.Ka14, "$djs", { value: { f: main, at: "await#2", segs: [298,361,9,309,372,10], scaf: [["k",0,213],["k",0,220],["k",0,229],["tmp",0,241,"call.arg@303","array.element"],["k",0,252],["k",0,259],["k",0,268],["site",345,279],["tmp",0,278,"call.arg@345","array.element"],["k",0,277],["tmp",0,252,"call.arg@330","array.element"],["tmp",0,298,"call.arg@361","array.element"],["tmp",0,309,"call.arg@372","array.element"],["k",0,240]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.arg@303",1,"[1,2,3]"],["call.callee@330",0,"$djs_t1"],["call.this@330",0,"$djs_t2"]] } });
$dump.a14 = $dump.Ka14.call(undefined);
Object.defineProperty($dump.n1, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n4, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
