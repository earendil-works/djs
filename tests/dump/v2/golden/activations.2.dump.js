"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
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
$dump.p1 = $djs.resume(2, "later", [2]);
$dump.n2 = (function () { "use strict"; return arguments; })("a", "b");
$dump.Ka3 = async function () {
var $djs_arguments = $dump.n2;
(await $dump.p1);
return [...$djs_arguments].join("/");
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: args, segs: [69,55,11,94,75,12], scaf: [["site",24,52]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, [42, (await $dump.a3), await shared()]);
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#2", segs: [122,69,14], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@30","array.element"],["site",47,105],["tmp",0,104,"call.arg@47","array.element"],["tmp",0,122,"call.arg@69","array.element"],["k",0,99]], cmap: [["call.callee@24",0,"$djs_t0"],["call.this@24",1,undefined],["call.arg@30",1,42]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
