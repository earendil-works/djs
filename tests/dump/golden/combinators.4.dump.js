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
$dump.p1 = $djs.resume(4, "later", [4]);
$dump.n2 = new Error("x");
$dump.p3 = $djs.resume(5, "later", [$dump.n2, true]);
$dump.p4 = $djs.resume(6, "later", ["ok"]);
$dump.p5 = $djs.resume(7, "later", ["fast"]);
$dump.n6 = new Error("e");
$dump.p7 = Promise.allSettled([$djs.rejected($dump.n6), $dump.p1]);
$dump.p8 = Promise.any([$dump.p3, $dump.p4]);
$dump.p9 = new Promise((res, rej) => { $dump.r10 = [res, rej]; });
$dump.p11 = Promise.prototype.then.call($dump.p5, $dump.r10[0], $dump.r10[1]);
$dump.n12 = Promise.resolve([1, 2, 3]);
$dump.a13_0 = print;
$dump.a13_1 = JSON.stringify;
$dump.a13_2 = JSON;
$dump.Ka13 = async function () {
const $djs_R = $djs.R;
const all = $dump.n12;
const settled = $dump.p7;
const any = $dump.p8;
const race = $dump.p9;
const $djs_t0 = $dump.a13_0, $djs_t1 = $dump.a13_1, $djs_t2 = $dump.a13_2;
$djs_R($djs_t0, undefined, ["[1,2,3]", $djs_R($djs_t1, $djs_t2, [(await $dump.p7)]), await any, await race]);
};
Object.defineProperty($dump.Ka13, "$djs", { value: { f: main, at: "await#2", segs: [297,361,9,308,372,10], scaf: [["k",0,212],["k",0,219],["k",0,228],["tmp",0,240,"call.arg@303","array.element"],["k",0,251],["k",0,258],["k",0,267],["site",345,278],["tmp",0,277,"call.arg@345","array.element"],["k",0,276],["tmp",0,251,"call.arg@330","array.element"],["tmp",0,297,"call.arg@361","array.element"],["tmp",0,308,"call.arg@372","array.element"],["k",0,239]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.arg@303",1,"[1,2,3]"],["call.callee@330",0,"$djs_t1"],["call.this@330",0,"$djs_t2"]] } });
$dump.a13 = $dump.Ka13.call(undefined);
Object.defineProperty($dump.n2, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n6, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
