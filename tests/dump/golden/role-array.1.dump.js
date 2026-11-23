"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const a = [1, 2, await later(3)];
  const b = [1, , await later(3)];
  return a.join() + " " + b.length;
}
$dump.p1 = $djs.resume(1, "later", [3]);
$dump.Ka2 = async function () {

const a = [1, 2, (await $dump.p1)];
const b = [1, , await later(3)];
return a.join() + " " + b.length;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [27,32,1,57,62,32,90,97,33], scaf: [["tmp",0,32,"array.element@37","array.element"],["tmp",0,35,"array.element@40","array.element"],["site",43,39],["tmp",0,38,"array.element@43","array.element"],["tmp",0,31,"array.literal@36","array.literal"]], cmap: [["array.element@37",1,1],["array.element@40",1,2]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
