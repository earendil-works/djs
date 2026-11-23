"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 12);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.b1 = Object.getPrototypeOf(""[Symbol.iterator]())["next"];
async function main() {
  let i = 0;
  while (i < 2) {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  }
}
$dump.p1 = $djs.resume(11, "later", ["xq"]);
$dump.i2 = $djs.advance(Array.prototype.values.call(["x", "y"]), 1);
$dump.i3 = $djs.advance(String.prototype[Symbol.iterator].call("pq"), 2);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
let i = 1;
{ let $djs_f1 = true; while ($djs_f1 || (i < 2)) { if ($djs_f1) { $djs_f1 = false; {

{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.i2, $dump.b0)) { if ($djs_f3) { $djs_f3 = false; {
const a = "x";

{

{ let $djs_f5 = true; for (const $djs_v5 of $djs_L($dump.i3, $dump.b1)) { if ($djs_f5) { $djs_f5 = false; {
const b = "q";

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, [1, "x", "q", (await $dump.p1)]);
} } else { const b = $djs_v5; print(i, a, b, await later(a + b)); } } }
}
} } else { const a = $djs_v3; {
      for (const b of "pq") print(i, a, b, await later(a + b));
    } } } }
i++;
} } else {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  } } }
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, segs: [112,46,5,508,100,7,527,117,35,582,64,7,601,87,71,679,163,4,693,53,118], scaf: [["while",39,93],["for-of",59,179],["for-of",95,306],["k",0,436],["k",0,443],["k",0,452],["tmp",0,464,"call.arg@123","array.element"],["tmp",0,467,"call.arg@126","array.element"],["tmp",0,472,"call.arg@129","array.element"],["site",132,478],["tmp",0,477,"call.arg@132","array.element"],["k",0,463],["site",152,498],["site",158,570],["site",158,572],["site",171,685]], cmap: [["call.callee@117",0,"$djs_t0"],["call.this@117",1,undefined],["call.arg@123",1,1],["call.arg@126",1,"x"],["call.arg@129",1,"q"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
