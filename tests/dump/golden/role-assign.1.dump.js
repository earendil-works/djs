"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
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
$dump.p1 = $djs.resume(1, "later", [1]);
$dump.n2 = { r: 1 };
$dump.Ka3 = async function () {
const t = $dump.n2;
const k = "q";
let n = 1;
const $djs_t0 = $dump.n2;
($djs_t0.p = (await $dump.p1));
t[k] = await later(2);
n += await later(4);
t.r += await later(2);
return JSON.stringify(t) + " " + n;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, segs: [101,72,1,124,94,22,147,119,20,168,142,22,191,167,35], scaf: [["site",76,106]], cmap: [["assign.object@70",0,"$djs_t0"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
