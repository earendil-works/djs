"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 9);
globalThis.$dump = {};
async function keys() {
  const proto = { inherited: 1 };
  const o = Object.create(proto);
  o.a = 1;
  o.b = 2;
  o.c = 3;
  const seen = [];
  for (const k in o) {
    seen.push(k + "=" + (await later(o[k])));
    if (k === "a") {
      delete o.b;
      o.d = 4;
    }
  }
  return seen.join(" ");
}
async function nested() {
  const out = [];
  for (const i in [10, 20]) {
    for (const c in "xy") out.push(i + c + (await later(c)));
  }
  return out.join(" ");
}
async function main() {
  print("keys", await keys());
  print("nested", await nested());
  const g = walk({ p: 1, q: 2, r: 3 });
  const seen = [g.next().value];
  await later("g");
  seen.push(g.next().value, g.next().value);
  print("walk", seen.join(" "));
}
$dump.p1 = $djs.resume(8, "later", ["1"]);
var walk = function* walk(o) {
  for (const k in o) yield k;
};
$dump.n2 = ["000", "011", "100"];
$dump.a3_0 = $djs.forIn([10, 20], []);
$dump.a3_1 = $djs.forIn(undefined, []);
$dump.a3_2 = Array.prototype.push;
$dump.Ka3 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const out = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.a3_0, null)) { if ($djs_f1) { $djs_f1 = false; {
const i = "1";

{

{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.a3_1, null)) { if ($djs_f3) { $djs_f3 = false; {
const c = "1";

const $djs_t0 = $dump.a3_2, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [("11" + (await $dump.p1))]);
} } else { const c = $djs_v3; out.push(i + c + (await later(c))); } } }
}
} } else { const i = $djs_v1; {
    for (const c in "xy") out.push(i + c + (await later(c)));
  } } } }
return out.join(" ");
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: nested, segs: [443,83,7,462,100,35,517,51,7,536,72,67,610,142,21], scaf: [["for-in",46,104],["for-in",78,229],["k",0,377],["k",0,384],["k",0,393],["site",118,412],["tmp",0,403,"call.arg@109","array.element"],["k",0,402],["site",135,433],["site",139,505],["site",139,507]], cmap: [["call.callee@100",0,"$djs_t0"],["call.this@100",0,"$djs_t1"],["binary.left@109",1,"11"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, ["nested", (await $dump.a3)]);
const g = walk({ p: 1, q: 2, r: 3 });
const seen = [g.next().value];
await later("g");
seen.push(g.next().value, g.next().value);
print("walk", seen.join(" "));
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#2", segs: [130,92,37,168,132,30,199,165,17,217,185,42,260,230,30], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@63","array.element"],["site",73,111],["tmp",0,110,"call.arg@73","array.element"],["k",0,99]], cmap: [["call.callee@57",0,"$djs_t0"],["call.this@57",1,undefined],["call.arg@63",1,"nested"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
