"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
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
$dump.p1 = $djs.resume(1, "later", [1]);
var walk = function* walk(o) {
  for (const k in o) yield k;
};
$dump.n2 = { inherited: 1 };
$dump.n3 = { __proto__: $dump.n2, a: 1, b: 2, c: 3 };
$dump.n4 = [];
$dump.a5_0 = $djs.forIn($dump.n3, ["b", "c", "inherited"]);
$dump.a5_1 = Array.prototype.push;
$dump.Ka5 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const proto = $dump.n2;
const o = $dump.n3;
const seen = $dump.n4;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.a5_0, null)) { if ($djs_f1) { $djs_f1 = false; {
const k = "a";

{

const $djs_t0 = $dump.a5_1, $djs_t1 = $dump.n4;
$djs_R($djs_t0, $djs_t1, [("a=" + (await $dump.p1))]);
if (k === "a") {
      delete o.b;
      o.d = 4;
    }
}
} } else { const k = $djs_v1; {
    seen.push(k + "=" + (await later(o[k])));
    if (k === "a") {
      delete o.b;
      o.d = 4;
    }
  } } } }
return seen.join(" ");
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: keys, segs: [355,217,55,424,151,7,443,165,111,561,279,22], scaf: [["for-in",146,149],["k",0,300],["k",0,307],["k",0,316],["site",192,335],["tmp",0,326,"call.arg@181","array.element"],["k",0,325],["site",276,412],["site",276,414]], cmap: [["call.callee@171",0,"$djs_t0"],["call.this@171",0,"$djs_t1"],["binary.left@181",1,"a="]] } });
$dump.a5 = $dump.Ka5.call(undefined);
$dump.a6_0 = print;
$dump.Ka6 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a6_0;
$djs_R($djs_t0, undefined, ["keys", (await $dump.a5)]);
print("nested", await nested());
const g = walk({ p: 1, q: 2, r: 3 });
const seen = [g.next().value];
await later("g");
seen.push(g.next().value, g.next().value);
print("walk", seen.join(" "));
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, segs: [128,57,32,161,92,37,199,132,30,230,165,17,248,185,42,291,230,30], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@32","array.element"],["site",40,109],["tmp",0,108,"call.arg@40","array.element"],["k",0,99]], cmap: [["call.callee@26",0,"$djs_t0"],["call.this@26",1,undefined],["call.arg@32",1,"keys"]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
