"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
$dump.p1 = $djs.resume(2, "later", [9]);
$dump.Ka2 = async function () {
const a = 5;
return [30, (5 < (await $dump.p1))].join();
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#2", segs: [68,92,5,73,97,2], scaf: [["tmp",0,41,"array.element@49","array.element"],["site",76,51],["tmp",0,45,"array.element@71","array.element"],["tmp",0,40,"array.literal@48","array.literal"]], cmap: [["array.element@49",1,30],["binary.left@71",1,5]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
