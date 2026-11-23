"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 9);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
async function firstOf(tag) {
  for await (const v of source(tag)) {
    return "first " + v;
  }
}
async function main() {
  for await (const v of source("a")) {
    print("got", v);
    break;
  }
  try {
    for await (const v of source("b")) {
      print("got", v);
      throw new Error("body failed on " + v);
    }
  } catch (e) {
    print("caught", e.message);
  }
  print(await firstOf("c"));
  print("end");
}
$dump.p1 = $djs.resume(8, "later", ["c cleanup"]);
var source = async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
};
$dump.n4 = { __proto__: $dump.b0 };
$dump.ga3 = () => {

};
$dump.a3_0 = print;
$dump.Ka3 = async function* () {
const $djs_R = $djs.R;
var tag = "c";
{ const $djs_c1 = undefined; {

$dump.ga3();
const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["c", "closing", (await $dump.p1)]);
} return $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: source, scaf: [["finally",32,59,2,"$djs_c1"],["k",0,132],["k",0,139],["k",0,148],["tmp",0,160,"call.arg@104","array.element"],["tmp",0,165,"call.arg@109","array.element"],["site",120,177],["tmp",0,176,"call.arg@120","array.element"],["k",0,159],["site",155,197]], cmap: [["call.callee@98",0,"$djs_t0"],["call.this@98",1,undefined],["call.arg@104",1,"c"],["call.arg@109",1,"closing"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.Ka5 = async function () {
const $djs_O = $djs.O;
var tag = "c";
{ $djs_O((await $dump.p2)); return "first c1"; }
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: firstOf, at: "for-await.close#2", scaf: [["site",32,68,"for-await.close#2"]], cmap: [["completion.value@32",1,"first c1"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
$dump.a6_0 = print;
$dump.Ka6 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a6_0;
$djs_R($djs_t0, undefined, [(await $dump.a5)]);
print("end");
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, segs: [120,306,13], scaf: [["k",0,72],["k",0,79],["k",0,88],["site",283,101],["tmp",0,100,"call.arg@283","array.element"],["k",0,99]], cmap: [["call.callee@277",0,"$djs_t0"],["call.this@277",1,undefined]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
