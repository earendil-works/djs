"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 17);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
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
$dump.p1 = $djs.resume(16, "later", ["concat"]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
$dump.i2 = $djs.regexpIterator(/[a-z](\d+)/g, "a1 b22 c333", 5);
$dump.i3 = $djs.regexpIterator(/\d/g, "x9y8", 5);
$dump.i4 = $djs.HD();
$dump.i5 = $djs.HD();
$dump.n7 = $djs.fn({ next() { return { value: this.i++, done: this.i > 3 }; } });
$dump.n6 = { i: 2, next: $dump.n7 };
$dump.i8 = Iterator.from($djs.I($dump.n6, $dump.n7));
$dump.n9 = new Set(["r"]);
$dump.n10 = ["p", "q"];
$dump.i11 = $djs.advance(Array.prototype.values.call($dump.n10), 1);
$dump.i12 = $djs.CC($dump.i11, $dump.b0, [$djs.CI(Set.prototype.keys, $dump.n9)]);
$dump.Ka13 = async function () {
const matches = $dump.i2;
const found = ["1@0", "22@3", "333@7"];
const it = $dump.i3;
const evens = $dump.i4;
const got = [20, 40, 60];
const flat = $dump.i5;
const seen = [1, 2, 3, true];
const wrapped = $dump.i8;
const w = [0, 1];
const cat = $dump.i12;
const c = ["p"];
(await $dump.p1);
c.push(...cat);
print("concat", c.join(" "));
};
Object.defineProperty($dump.Ka13, "$djs", { value: { f: main, at: "await#6", segs: [312,1152,15,328,1170,29], scaf: [["site",1127,295]] } });
$dump.a13 = $dump.Ka13.call(undefined);
$dump.a14 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), numbers.prototype);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
