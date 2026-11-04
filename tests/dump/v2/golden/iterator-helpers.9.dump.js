"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 15);
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
$dump.p1 = $djs.resume(14, "later", ["wrapped"]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
$dump.i2 = $djs.regexpIterator(/[a-z](\d+)/g, "a1 b22 c333", 5);
$dump.i3 = $djs.regexpIterator(/\d/g, "x9y8", 5);
$dump.i4 = $djs.HD();
$dump.i5 = $djs.HD();
$dump.n7 = $djs.fn({ next() { return { value: this.i++, done: this.i > 3 }; } });
$dump.n6 = { i: 1, next: $dump.n7 };
$dump.i8 = Iterator.from($djs.I($dump.n6, $dump.n7));
$dump.Ka9 = async function () {
const matches = $dump.i2;
const found = ["1@0", "22@3", "333@7"];
const it = $dump.i3;
const evens = $dump.i4;
const got = [20, 40, 60];
const flat = $dump.i5;
const seen = [1, 2, 3, true];
const wrapped = $dump.i8;
const w = [0];
(await $dump.p1);
w.push(wrapped.next().value);
print("from", w.join(" "));
const cat = Iterator.concat(["p", "q"], new Set(["r"]));
const c = [cat.next().value];
await later("concat");
c.push(...cat);
print("concat", c.join(" "));
};
Object.defineProperty($dump.Ka9, "$djs", { value: { f: main, at: "await#5", segs: [269,974,29,299,1006,27,327,1036,56,384,1095,29,414,1127,22,437,1152,15,453,1170,29], scaf: [["site",948,252]] } });
$dump.a9 = $dump.Ka9.call(undefined);
$dump.a10 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), numbers.prototype);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
