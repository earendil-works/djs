"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
async function main() {
  const k = "z";
  const a = { a: 1, [k]: await later(2) };
  const b = { a: 1, c: await later(3) };
  return JSON.stringify([a, b]);
}
$dump.p1 = $djs.resume(2, "later", [3]);
$dump.a2_0 = { a: 1, z: 2 };
$dump.a2_1 = { a: 1 };
$dump.Ka2 = async function () {
const $djs_D = $djs.D;
const k = "z";
const a = $dump.a2_0;
const $djs_t0 = $dump.a2_1;
const b = ($djs_D($djs_t0, "c", (await $dump.p1)), $djs_t0);
return JSON.stringify([a, b]);
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#2", segs: [114,92,1,169,127,30], scaf: [["site",107,141],["k",0,119],["k",0,126],["k",0,135]], cmap: [["object.literal@96",0,"$djs_t0"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
