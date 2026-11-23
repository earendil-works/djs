"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
async function main() {
  const a = [1, 2, await later(3)];
  const b = [1, , await later(3)];
  return a.join() + " " + b.length;
}
$dump.p1 = $djs.resume(2, "later", [3]);
$dump.Ka2 = async function () {
const $djs_Q = $djs.Q;
const a = [1, 2, 3];
const $djs_t0 = [1, , ];
const b = $djs_Q($djs_t0, (await $dump.p1));
return a.join() + " " + b.length;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#2", segs: [95,68,1,134,97,33], scaf: [["site",78,116]], cmap: [["array.literal@72",0,"$djs_t0"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
