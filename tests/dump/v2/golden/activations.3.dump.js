"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
globalThis.$dump = {};
$dump.s2 = {};
async function twice(x) { return later(x * 2); }
async function args() { await later(arguments.length); return [...arguments].join("/"); }
async function shared() {
  let n = 1;
  const inc = () => ++n;
  await later(0);
  inc();
  return n;
}
async function main() { print(await twice(21), await args("a", "b"), await shared()); }
$dump.p1 = $djs.resume(3, "later", [0]);
$dump.Ka3 = async function () {
let n = 1;
Object.assign($dump.s2, { inc: () => ++n });
const inc = $dump.s2.inc;
(await $dump.p1);
inc();
return n;
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: shared, segs: [120,84,6,127,93,9], scaf: [["site",66,103]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, [42, "a/b", (await $dump.a3)]);
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#3", scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@30","array.element"],["tmp",0,104,"call.arg@47","array.element"],["site",69,112],["tmp",0,111,"call.arg@69","array.element"],["k",0,99]], cmap: [["call.callee@24",0,"$djs_t0"],["call.this@24",1,undefined],["call.arg@30",1,42],["call.arg@47",1,"a/b"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
