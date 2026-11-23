"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
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
$dump.p1 = $djs.resume(1, "later", [42]);
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.r3[0], $dump.r3[1]);
$dump.a5_0 = print;
$dump.Ka5 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a5_0;
$djs_R($djs_t0, undefined, [(await $dump.p2), await args("a", "b"), await shared()]);
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, segs: [118,47,20,140,69,14], scaf: [["k",0,72],["k",0,79],["k",0,88],["site",30,101],["tmp",0,100,"call.arg@30","array.element"],["tmp",0,118,"call.arg@47","array.element"],["tmp",0,140,"call.arg@69","array.element"],["k",0,99]], cmap: [["call.callee@24",0,"$djs_t0"],["call.this@24",1,undefined]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
