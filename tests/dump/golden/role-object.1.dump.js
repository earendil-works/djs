"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const k = "z";
  const a = { a: 1, [k]: await later(2) };
  const b = { a: 1, c: await later(3) };
  return JSON.stringify([a, b]);
}
$dump.p1 = $djs.resume(1, "later", [2]);
$dump.a2_0 = { a: 1 };
$dump.Ka2 = async function () {
const $djs_D = $djs.D;
const k = "z";
const $djs_t0 = $dump.a2_0;
const a = ($djs_D($djs_t0, "z", (await $dump.p1)), $djs_t0);
const b = { a: 1, c: await later(3) };
return JSON.stringify([a, b]);
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [92,49,1,147,86,38,186,127,30], scaf: [["site",66,119],["k",0,97],["k",0,104],["k",0,113]], cmap: [["object.literal@53",0,"$djs_t0"],["object.key@61",1,"z"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
