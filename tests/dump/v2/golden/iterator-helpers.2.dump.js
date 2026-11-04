"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(/(?:)/[Symbol.matchAll](""))["next"];
async function main() {
  const matches = "a1 b22 c333".matchAll(/[a-z](\d+)/g);
  const found = [];
  for (const m of matches) found.push(m[1] + "@" + (await later(m.index)));
  print("matchAll", found.join(" "));
  const it = "x9y8".matchAll(/\d/g);
  print("first", it.next().value[0]);
  await later("between");
  print("rest", JSON.stringify([...it].map((m) => m[0])));
  const evens = numbers()
    .filter((n) => n % 2 === 0)
    .map((n) => n * 10)
    .drop(1)
    .take(3);
  const got = [];
  for (const v of evens) got.push(await later(v));
  print("helpers", got.join(" "));
  const flat = [[1, 2], [3]].values().flatMap((a) => a);
  const seen = [flat.next().value];
  await later("flat");
  seen.push(flat.next().value, flat.next().value, flat.next().done);
  print("flatMap", seen.join(" "));
  const wrapped = Iterator.from({ i: 0, next() { return { value: this.i++, done: this.i > 3 }; } });
  const w = [wrapped.next().value];
  await later("wrapped");
  w.push(wrapped.next().value);
  print("from", w.join(" "));
  const cat = Iterator.concat(["p", "q"], new Set(["r"]));
  const c = [cat.next().value];
  await later("concat");
  c.push(...cat);
  print("concat", c.join(" "));
}
$dump.p1 = $djs.resume(2, "later", [3]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
$dump.n2 = /[a-z](\d+)/g;
$dump.n2.lastIndex = 6;
$dump.i3 = $djs.regexpIterator($dump.n2, "a1 b22 c333", 1);
$dump.n4 = ["1@0"];
$dump.n5 = ["b22", "22"];
$dump.a6_0 = Array.prototype.push;
$dump.Ka6 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const matches = $dump.i3;
const found = $dump.n4;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i3, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const m = $dump.n5;

const $djs_t0 = $dump.a6_0, $djs_t1 = $dump.n4;
$djs_R($djs_t0, $djs_t1, [("22@" + (await $dump.p1))]);
} } else { const m = $djs_v1; found.push(m[1] + "@" + (await later(m.index))); } } }
print("matchAll", found.join(" "));
const it = "x9y8".matchAll(/\d/g);
print("first", it.next().value[0]);
await later("between");
print("rest", JSON.stringify([...it].map((m) => m[0])));
const evens = numbers()
    .filter((n) => n % 2 === 0)
    .map((n) => n * 10)
    .drop(1)
    .take(3);
const got = [];
for (const v of evens) got.push(await later(v));
print("helpers", got.join(" "));
const flat = [[1, 2], [3]].values().flatMap((a) => a);
const seen = [flat.next().value];
await later("flat");
seen.push(flat.next().value, flat.next().value, flat.next().done);
print("flatMap", seen.join(" "));
const wrapped = Iterator.from({ i: 0, next() { return { value: this.i++, done: this.i > 3 }; } });
const w = [wrapped.next().value];
await later("wrapped");
w.push(wrapped.next().value);
print("from", w.join(" "));
const cat = Iterator.concat(["p", "q"], new Set(["r"]));
const c = [cat.next().value];
await later("concat");
c.push(...cat);
print("concat", c.join(" "));
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, segs: [354,108,7,373,128,48,428,179,35,464,217,34,499,254,35,535,292,23,559,318,56,616,377,106,723,486,15,739,504,48,788,555,32,821,590,54,876,647,33,910,683,20,931,706,66,998,775,33,1032,811,98,1131,912,33,1165,948,23,1189,974,29,1219,1006,27,1247,1036,56,1304,1095,29,1334,1127,22,1357,1152,15,1373,1170,29], scaf: [["for-of",103,132],["k",0,287],["k",0,294],["k",0,303],["site",153,323],["tmp",0,313,"call.arg@139","array.element"],["k",0,312],["site",176,344]], cmap: [["call.callee@128",0,"$djs_t0"],["call.this@128",0,"$djs_t1"],["binary.left@139",1,"22@"]] } });
$dump.a6 = $dump.Ka6.call(undefined);
$dump.n5.index = 3;
$dump.n5.input = "a1 b22 c333";
$dump.n5.groups = undefined;
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
