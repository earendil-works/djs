"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() { const t = { x: 9 }; return t[await later("x")]; }
$dump.p1 = $djs.resume(1, "later", ["x"]);
$dump.n2 = { x: 9 };
$dump.Ka3 = async function () {
const t = $dump.n2;
const $djs_t0 = $dump.n2;
return $djs_t0[(await $dump.p1)];
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, scaf: [["site",53,82]], cmap: [["member.object@51",0,"$djs_t0"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
