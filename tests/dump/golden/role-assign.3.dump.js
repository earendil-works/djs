"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
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
$dump.p1 = $djs.resume(3, "later", [4]);
$dump.a2_0 = { r: 1, p: 1, q: 2 };
$dump.Ka2 = async function () {
const t = $dump.a2_0;
const k = "q";
let n = 1;
(n = 1 + (await $dump.p1));
t.r += await later(2);
return JSON.stringify(t) + " " + n;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#3", segs: [69,119,1,96,142,22,119,167,35], scaf: [["site",124,78]], cmap: [["assign.old@119",1,1]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
