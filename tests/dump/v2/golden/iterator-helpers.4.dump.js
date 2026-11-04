"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 7);
globalThis.$dump = {};
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
$dump.p1 = $djs.resume(6, "later", ["between"]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
$dump.i2 = $djs.regexpIterator(/[a-z](\d+)/g, "a1 b22 c333", 5);
$dump.n3 = /\d/g;
$dump.n3.lastIndex = 2;
$dump.i4 = $djs.regexpIterator($dump.n3, "x9y8", 1);
$dump.Ka5 = async function () {
const matches = $dump.i2;
const found = ["1@0", "22@3", "333@7"];
const it = $dump.i4;
(await $dump.p1);
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
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#2", segs: [125,318,56,182,377,106,289,486,15,305,504,48,354,555,32,387,590,54,442,647,33,476,683,20,497,706,66,564,775,33,598,811,98,697,912,33,731,948,23,755,974,29,785,1006,27,813,1036,56,870,1095,29,900,1127,22,923,1152,15,939,1170,29], scaf: [["site",292,108]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
