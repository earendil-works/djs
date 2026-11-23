"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
$dump.p1 = $djs.resume(1, "later", [6]);
$dump.Ka2 = async function () {
const a = 5;
return [(5 * (await $dump.p1)), a < (await later(9))].join();
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [65,71,20,86,92,5,91,97,2], scaf: [["site",54,47],["tmp",0,41,"array.element@49","array.element"],["tmp",0,40,"array.literal@48","array.literal"]], cmap: [["binary.left@49",1,5]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
