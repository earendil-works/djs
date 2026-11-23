"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 11);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
$dump.b2 = Object.getPrototypeOf([].values().map((x) => x))["next"];
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
$dump.p1 = $djs.resume(10, "later", [60]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
$dump.i2 = $djs.regexpIterator(/[a-z](\d+)/g, "a1 b22 c333", 5);
$dump.i3 = $djs.regexpIterator(/\d/g, "x9y8", 5);
$dump.n5 = { __proto__: $dump.b0 };
$dump.Ka4 = function* () {
let i = 7;
{ let $djs_f1 = true; while ($djs_f1 || (true)) { if ($djs_f1) { $djs_f1 = false; (yield); } else yield i++; } }
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: numbers, at: "yield#1", segs: [67,44,4,124,50,10], scaf: [["while",37,48],["site",50,109]] } });
$dump.Ka4.prototype = $dump.n5;
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a4.next();
$dump.i6 = Iterator.prototype.filter.call($djs.I($dump.a4, $dump.b1), $djs.M((n) => n % 2 === 0, 7));
$dump.i7 = Iterator.prototype.map.call($djs.I($dump.i6, $dump.b2), $djs.M((n) => n * 10, 4));
$dump.i8 = Iterator.prototype.drop.call($djs.I($dump.i7, $dump.b2), 0);
$dump.i9 = Iterator.prototype.take.call($djs.I($dump.i8, $dump.b2), 0);
$dump.n10 = [20, 40];
$dump.a11_0 = Array.prototype.push;
$dump.Ka11 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const matches = $dump.i2;
const found = ["1@0", "22@3", "333@7"];
const it = $dump.i3;
const evens = $dump.i9;
const got = $dump.n10;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i9, $dump.b2)) { if ($djs_f1) { $djs_f1 = false; {
const v = 60;

const $djs_t0 = $dump.a11_0, $djs_t1 = $dump.n10;
$djs_R($djs_t0, $djs_t1, [(await $dump.p1)]);
} } else { const v = $djs_v1; got.push(await later(v)); } } }
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
Object.defineProperty($dump.Ka11, "$djs", { value: { f: main, at: "await#3", segs: [424,509,7,443,527,25,475,555,32,508,590,54,563,647,33,597,683,20,618,706,66,685,775,33,719,811,98,818,912,33,852,948,23,876,974,29,906,1006,27,934,1036,56,991,1095,29,1021,1127,22,1044,1152,15,1060,1170,29], scaf: [["for-of",504,216],["k",0,367],["k",0,374],["k",0,383],["site",536,394],["tmp",0,393,"call.arg@536","array.element"],["k",0,392],["site",552,414]], cmap: [["call.callee@527",0,"$djs_t0"],["call.this@527",0,"$djs_t1"]] } });
$dump.a11 = $dump.Ka11.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
