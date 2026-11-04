"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() { return o.f(...[1, 2], await later(3), 4); }
$dump.p1 = $djs.resume(1, "later", [3]);
$dump.n2 = $djs.fn({ f(...a) { return a.join(); } });
const o = { f: $dump.n2 };
$dump.a3_0 = o;
$dump.Ka3 = async function () {
const $djs_R = $djs.R, $djs_Q = $djs.Q;

const $djs_t0 = $dump.n2, $djs_t1 = $dump.a3_0, $djs_t2 = [1, 2];
return $djs_R($djs_t0, $djs_t1, $djs_Q($djs_t2, (await $dump.p1), 4));
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, segs: [193,62,1], scaf: [["k",0,134],["k",0,141],["k",0,150],["site",46,176]], cmap: [["call.callee@31",0,"$djs_t0"],["call.this@31",0,"$djs_t1"],["call.args@34",0,"$djs_t2"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
