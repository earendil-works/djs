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
$dump.p1 = $djs.resume(7, "later", ["fast"]);
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.r3[0], $dump.r3[1]);
$dump.n5 = Promise.resolve([1, 2, 3]);
$dump.n7 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n6 = Promise.resolve($dump.n7);
$dump.n8 = Promise.resolve("ok");
$dump.a9_0 = print;
$dump.Ka9 = async function () {
const $djs_R = $djs.R;
const all = $dump.n5;
const settled = $dump.n6;
const any = $dump.n8;
const race = $dump.p2;
const $djs_t0 = $dump.a9_0, $djs_t1 = "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]";
$djs_R($djs_t0, undefined, ["[1,2,3]", $djs_t1, "ok", (await $dump.p2)]);
};
Object.defineProperty($dump.Ka9, "$djs", { value: { f: main, at: "await#4", scaf: [["k",0,258],["k",0,265],["k",0,274],["tmp",0,286,"call.arg@303","array.element"],["tmp",0,297,"call.arg@330","array.element"],["tmp",0,306,"call.arg@361","array.element"],["site",372,313],["tmp",0,312,"call.arg@372","array.element"],["k",0,285]], cmap: [["call.callee@297",0,"$djs_t0"],["call.this@297",1,undefined],["call.arg@303",1,"[1,2,3]"],["call.arg@330",0,"$djs_t1"],["call.arg@361",1,"ok"]] } });
$dump.a9 = $dump.Ka9.call(undefined);
Object.defineProperty($dump.n7[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
