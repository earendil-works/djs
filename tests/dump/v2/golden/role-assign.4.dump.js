"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
globalThis.$dump = {};
async function main() {
  const t = { r: 1 }, k = "q";
  let n = 1;
  t.p = await later(1);
  t[k] = await later(2);
  n += await later(4);
  t.r += await later(2);
  return JSON.stringify(t) + " " + n;
}
$dump.p1 = $djs.resume(4, "later", [2]);
$dump.n2 = { r: 1, p: 1, q: 2 };
$dump.Ka3 = async function () {
const t = $dump.n2;
const k = "q";
let n = 5;
const $djs_t0 = $dump.n2;
($djs_t0.r = 1 + (await $dump.p1));
return JSON.stringify(t) + " " + n;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, at: "await#4", segs: [101,144,1,128,167,35], scaf: [["site",149,110]], cmap: [["assign.object@142",0,"$djs_t0"],["assign.old@142",1,1]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
