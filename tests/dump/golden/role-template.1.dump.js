"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() { return `a${1}b${await later(2)}c`; }
$dump.p1 = $djs.resume(1, "later", [2]);
$dump.Ka2 = async function () {

return `a${"1"}b${(await $dump.p1)}c`;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [28,31,4,35,36,4,55,54,3], scaf: [["site",40,40]], cmap: [["template.part@35",1,"1"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
