"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
async function main() {
  const it = g();
  print(JSON.stringify(await it.next()));
  print(JSON.stringify(await it.return("done")));
  print("end");
}
$dump.p1 = $djs.resume(2, "later", ["a cleanup"]);
var source = async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
};
var g = async function* g() {
  const o = {};
  try {
    for await (o[yield "key?"] of source("a")) print("never");
  } finally {
    print("g finally", await later("g cleanup"));
  }
};
$dump.n4 = { __proto__: $dump.b0 };
$dump.ga3 = () => {

};
$dump.a3_0 = print;
$dump.Ka3 = async function* () {
const $djs_R = $djs.R;
var tag = "a";
{ const $djs_c1 = undefined; {

$dump.ga3();
const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["a", "closing", (await $dump.p1)]);
} return $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: source, scaf: [["finally",32,59,2,"$djs_c1"],["k",0,132],["k",0,139],["k",0,148],["tmp",0,160,"call.arg@104","array.element"],["tmp",0,165,"call.arg@109","array.element"],["site",120,177],["tmp",0,176,"call.arg@120","array.element"],["k",0,159],["site",155,197]], cmap: [["call.callee@98",0,"$djs_t0"],["call.this@98",1,undefined],["call.arg@104",1,"a"],["call.arg@109",1,"closing"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.n7 = { __proto__: $dump.b0 };
$dump.ga6 = () => {

};
$dump.Ka6 = async function* () {
const $djs_O = $djs.O;
const o = {};
try {

$dump.ga6();
{ $djs_O((await $dump.p2)); return "done"; }
} finally {
    print("g finally", await later("g cleanup"));
  }
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: g, at: "for-await.close#2", segs: [133,121,55], scaf: [["try",40,58],["site",50,88,"for-await.close#2"],["site",112,124]], cmap: [["completion.value@50",1,"done"]] } });
$dump.Ka6.prototype = $dump.n7;
$dump.a6 = $dump.Ka6.call(undefined);
$dump.p5 = $dump.a6.next();
$dump.a8_0 = print;
$dump.a8_1 = JSON.stringify;
$dump.a8_2 = JSON;
$dump.Ka8 = async function () {
const $djs_R = $djs.R;
const it = $dump.a6;
const $djs_t0 = $dump.a8_0, $djs_t1 = $dump.a8_1, $djs_t2 = $dump.a8_2;
$djs_R($djs_t0, undefined, [$djs_R($djs_t1, $djs_t2, [(await $dump.p5)])]);
print("end");
};
Object.defineProperty($dump.Ka8, "$djs", { value: { f: main, at: "await#2", segs: [212,136,13], scaf: [["k",0,136],["k",0,143],["k",0,152],["k",0,164],["k",0,171],["k",0,180],["site",107,191],["tmp",0,190,"call.arg@107","array.element"],["k",0,189],["tmp",0,164,"call.arg@92","array.element"],["k",0,163]], cmap: [["call.callee@86",0,"$djs_t0"],["call.this@86",1,undefined],["call.callee@92",0,"$djs_t1"],["call.this@92",0,"$djs_t2"]] } });
$dump.a8 = $dump.Ka8.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
