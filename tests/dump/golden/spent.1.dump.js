"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  await later(0);
  res("again");
  rej(new Error("late"));
  print(await p);
}
$dump.p1 = $djs.resume(1, "later", [0]);
let res = $djs.spent(0);
let rej = $djs.spent(1);
const p = Promise.resolve("first");
$dump.Ka2 = async function () {

(await $dump.p1);
res("again");
rej(new Error("late"));
print(await p);
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [39,44,13,53,60,23,77,86,15], scaf: [["site",26,22]] } });
$dump.a2 = $dump.Ka2.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
