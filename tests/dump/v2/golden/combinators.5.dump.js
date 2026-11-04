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
$dump.n1 = new Error("x");
$dump.p2 = $djs.resume(5, "later", [$dump.n1, true]);
$dump.p3 = $djs.resume(6, "later", ["ok"]);
$dump.p4 = $djs.resume(7, "later", ["fast"]);
$dump.p5 = Promise.any([$dump.p2, $dump.p3]);
$dump.p6 = new Promise((res, rej) => { $dump.r7 = [res, rej]; });
$dump.p8 = Promise.prototype.then.call($dump.p4, $dump.r7[0], $dump.r7[1]);
$dump.n9 = Promise.resolve([1, 2, 3]);
$dump.n11 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n10 = Promise.resolve($dump.n11);
$dump.a12_0 = print;
$dump.Ka12 = async function () {
const $djs_R = $djs.R;
const all = $dump.n9;
const settled = $dump.n10;
const any = $dump.p5;
const race = $dump.p6;
const $djs_t0 = $dump.a12_0, $djs_t1 = "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]";
$djs_R($djs_t0, undefined, ["[1,2,3]", $djs_t1, (await $dump.p5), await race]);
};
Object.defineProperty($dump.Ka12, "$djs", { value: { f: main, at: "await#3", segs: [326,372,10], scaf: [["k",0,260],["k",0,267],["k",0,276],["tmp",0,288,"call.arg@303","array.element"],["tmp",0,299,"call.arg@330","array.element"],["site",361,309],["tmp",0,308,"call.arg@361","array.element"],["tmp",0,326,"call.arg@372","array.element"],["k",0,287]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.arg@303",1,"[1,2,3]"],["call.arg@330",0,"$djs_t1"]] } });
$dump.a12 = $dump.Ka12.call(undefined);
Object.defineProperty($dump.n1, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n11[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
