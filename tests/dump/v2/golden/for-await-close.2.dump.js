"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 6);
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
$dump.p1 = $djs.resume(5, "later", ["b cleanup"]);
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
var tag = "b";
{ const $djs_c1 = undefined; {

$dump.ga3();
const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["b", "closing", (await $dump.p1)]);
} return $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: source, scaf: [["finally",32,59,2,"$djs_c1"],["k",0,132],["k",0,139],["k",0,148],["tmp",0,160,"call.arg@104","array.element"],["tmp",0,165,"call.arg@109","array.element"],["site",120,177],["tmp",0,176,"call.arg@120","array.element"],["k",0,159],["site",155,197]], cmap: [["call.callee@98",0,"$djs_t0"],["call.this@98",1,undefined],["call.arg@104",1,"b"],["call.arg@109",1,"closing"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.p5 = Promise.prototype.then.call($dump.p2, () => (undefined), () => (undefined));
$dump.n6 = new Error("body failed on b1");
$dump.Ka7 = async function () {

try {

const $djs_t0 = $dump.n6;
{ (await $dump.p5); throw $djs_t0; }
} catch (e) {
    print("caught", e.message);
  }
print(await firstOf("c"));
print("end");
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: main, at: "for-await.close#3", segs: [100,234,1,103,237,37,141,277,26,168,306,13], scaf: [["try",101,21],["site",111,57,"for-await.close#3"],["site",226,92]], cmap: [["for-await.exception@111",0,"$djs_t0"]] } });
$dump.a7 = $dump.Ka7.call(undefined);
Object.defineProperty($dump.n6, "stack", { value: "    at main (src.js:25:22)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
