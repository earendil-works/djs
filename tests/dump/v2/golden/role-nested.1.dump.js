"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const p = "x";
  return o.f(p + "!", [1, { k: `v${2}w${await later(3)}` }]);
}
$dump.p1 = $djs.resume(1, "later", [3]);
$dump.n2 = $djs.fn({ f(a, b) { return a + "|" + JSON.stringify(b); } });
const o = { f: $dump.n2 };
$dump.a3_0 = o;
$dump.Ka3 = async function () {
const $djs_R = $djs.R, $djs_D = $djs.D;
const p = "x";
const $djs_t0 = $dump.n2, $djs_t1 = $dump.a3_0, $djs_t2 = {};
return $djs_R($djs_t0, $djs_t1, ["x!", [1, ($djs_D($djs_t2, "k", `v${"2"}w${(await $dump.p1)}`), $djs_t2)]]);
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, segs: [202,72,4,209,77,4,229,95,2], scaf: [["k",0,144],["k",0,151],["k",0,160],["tmp",0,170,"call.arg@54","array.element"],["tmp",0,177,"array.element@64","array.element"],["site",81,214],["k",0,181],["k",0,188],["k",0,197],["tmp",0,180,"array.element@67","array.element"],["tmp",0,176,"array.literal@63","array.literal"],["tmp",0,176,"call.arg@63","array.element"],["k",0,169]], cmap: [["call.callee@50",0,"$djs_t0"],["call.this@50",0,"$djs_t1"],["call.arg@54",1,"x!"],["array.element@64",1,1],["object.literal@67",0,"$djs_t2"],["template.part@76",1,"2"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
