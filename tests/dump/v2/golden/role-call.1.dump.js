"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() { return o.f(1, await later(2), 3); }
$dump.p1 = $djs.resume(1, "later", [2]);
$dump.n2 = $djs.fn({ f(a, b, c) { return [this.k, a, b, c].join(); } });
const o = { k: 1, f: $dump.n2 };
$dump.a3_0 = o;
$dump.Ka3 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.n2, $djs_t1 = $dump.a3_0;
return $djs_R($djs_t0, $djs_t1, [1, (await $dump.p1), 3]);
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, segs: [146,54,1], scaf: [["k",0,99],["k",0,106],["k",0,115],["tmp",0,125,"call.arg@35","array.element"],["site",38,129],["tmp",0,128,"call.arg@38","array.element"],["tmp",0,146,"call.arg@54","array.element"],["k",0,124]], cmap: [["call.callee@31",0,"$djs_t0"],["call.this@31",0,"$djs_t1"],["call.arg@35",1,1]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
