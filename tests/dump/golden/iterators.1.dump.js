"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const ai = [1, 2, 3].values(); ai.next();
  const si = "h\u00e9\u{1F600}x"[Symbol.iterator](); si.next(); si.next();
  const mi = new Map([["a", 1], ["b", 2]]).entries(); mi.next();
  const ti = new Set(["s1", "s2"]).values(); ti.next();
  await later(0);
  print(ai.next().value, si.next().value, JSON.stringify(mi.next().value), ti.next().value);
}
$dump.p1 = $djs.resume(1, "later", [0]);
$dump.i2 = $djs.advance(Array.prototype.values.call([1, 2, 3]), 1);
$dump.i3 = $djs.advance(String.prototype[Symbol.iterator].call("hé😀x"), 2);
$dump.i4 = $djs.advance(Map.prototype.entries.call(new Map([["a", 1], ["b", 2]])), 1);
$dump.i5 = $djs.advance(Set.prototype.values.call(new Set(["s1", "s2"])), 1);
$dump.Ka6 = async function () {
const ai = $dump.i2;
const si = $dump.i3;
const mi = $dump.i4;
const ti = $dump.i5;
(await $dump.p1);
print(ai.next().value, si.next().value, JSON.stringify(mi.next().value), ti.next().value);
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, segs: [122,284,90], scaf: [["site",266,105]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
