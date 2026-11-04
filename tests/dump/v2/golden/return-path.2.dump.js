"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(async function* () {}).prototype;
async function main() {
  const it = g();
  print(JSON.stringify(await it.next()));
  print(JSON.stringify(await it.return("done")));
  print("end");
}
$dump.p1 = $djs.resume(4, "later", ["g cleanup"]);
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
const o = {};
{ const $djs_c1 = "done"; {

$dump.ga3();
const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["g finally", (await $dump.p1)]);
} return $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: g, scaf: [["finally",40,58,2,"$djs_c1"],["k",0,128],["k",0,135],["k",0,144],["tmp",0,156,"call.arg@133","array.element"],["site",146,170],["tmp",0,169,"call.arg@146","array.element"],["k",0,155],["site",176,190]], cmap: [["call.callee@127",0,"$djs_t0"],["call.this@127",1,undefined],["call.arg@133",1,"g finally"]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p2 = $dump.a3.next();
$dump.a5_0 = print;
$dump.a5_1 = JSON.stringify;
$dump.a5_2 = JSON;
$dump.Ka5 = async function () {
const $djs_R = $djs.R;
const it = $dump.a3;
const $djs_t0 = $dump.a5_0, $djs_t1 = $dump.a5_1, $djs_t2 = $dump.a5_2;
$djs_R($djs_t0, undefined, [$djs_R($djs_t1, $djs_t2, [(await $dump.p2)])]);
print("end");
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#2", segs: [212,136,13], scaf: [["k",0,136],["k",0,143],["k",0,152],["k",0,164],["k",0,171],["k",0,180],["site",107,191],["tmp",0,190,"call.arg@107","array.element"],["k",0,189],["tmp",0,164,"call.arg@92","array.element"],["k",0,163]], cmap: [["call.callee@86",0,"$djs_t0"],["call.this@86",1,undefined],["call.callee@92",0,"$djs_t1"],["call.this@92",0,"$djs_t2"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
