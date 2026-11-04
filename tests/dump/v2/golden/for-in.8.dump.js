"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 11);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
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
$dump.p1 = $djs.resume(10, "later", ["g"]);
var walk = function* walk(o) {
  for (const k in o) yield k;
};
$dump.n2 = { p: 1, q: 2, r: 3 };
$dump.n4 = { __proto__: $dump.b0 };
$dump.a3_0 = $djs.forIn($dump.n2, ["q", "r"]);
$dump.Ka3 = function* () {
const $djs_L = $djs.L;
var o = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.a3_0, null)) { if ($djs_f1) { $djs_f1 = false; {
const k = "p";

(yield);
} } else { const k = $djs_v1; yield k; } } }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: walk, at: "yield#1", segs: [198,27,7,217,41,8], scaf: [["for-in",22,78],["site",41,179],["site",49,188]] } });
$dump.Ka3.prototype = $dump.n4;
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a3.next();
$dump.Ka5 = async function () {
const g = $dump.a3;
const seen = ["p"];
(await $dump.p1);
seen.push(g.next().value, g.next().value);
print("walk", seen.join(" "));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#3", segs: [78,185,42,121,230,30], scaf: [["site",165,61]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
