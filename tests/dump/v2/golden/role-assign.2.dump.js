"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
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
$dump.p1 = $djs.resume(2, "later", [2]);
$dump.n2 = { r: 1, p: 1 };
$dump.Ka3 = async function () {
const t = $dump.n2;
const k = "q";
let n = 1;
const $djs_t0 = $dump.n2;
($djs_t0["q"] = (await $dump.p1));
n += await later(4);
t.r += await later(2);
return JSON.stringify(t) + " " + n;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, at: "await#2", segs: [127,119,20,148,142,22,171,167,35], scaf: [["site",101,109]], cmap: [["assign.object@94",0,"$djs_t0"],["assign.key@94",1,"q"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
