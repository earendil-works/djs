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
$dump.p1 = $djs.resume(6, "later", ["ok"]);
$dump.p2 = $djs.resume(7, "later", ["fast"]);
$dump.n3 = new Error("x");
$dump.p4 = Promise.any([$djs.rejected($dump.n3), $dump.p1]);
$dump.p5 = new Promise((res, rej) => { $dump.r6 = [res, rej]; });
$dump.p7 = Promise.prototype.then.call($dump.p2, $dump.r6[0], $dump.r6[1]);
$dump.n8 = Promise.resolve([1, 2, 3]);
$dump.n10 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n9 = Promise.resolve($dump.n10);
$dump.a11_0 = print;
$dump.Ka11 = async function () {
const $djs_R = $djs.R;
const all = $dump.n8;
const settled = $dump.n9;
const any = $dump.p4;
const race = $dump.p5;
const $djs_t0 = $dump.a11_0, $djs_t1 = "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]";
$djs_R($djs_t0, undefined, ["[1,2,3]", $djs_t1, (await $dump.p4), await race]);
};
Object.defineProperty($dump.Ka11, "$djs", { value: { f: main, at: "await#3", segs: [325,372,10], scaf: [["k",0,259],["k",0,266],["k",0,275],["tmp",0,287,"call.arg@303","array.element"],["tmp",0,298,"call.arg@330","array.element"],["site",361,308],["tmp",0,307,"call.arg@361","array.element"],["tmp",0,325,"call.arg@372","array.element"],["k",0,286]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.arg@303",1,"[1,2,3]"],["call.arg@330",0,"$djs_t1"]] } });
$dump.a11 = $dump.Ka11.call(undefined);
Object.defineProperty($dump.n3, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n10[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
