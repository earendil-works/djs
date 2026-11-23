"djs-dump";
"use strict";
$djs.seed(1473515859, 1479578119, 3);
globalThis.$dump = {};
async function main() {
  const a = Math.random();
  await later(0);
  const b = Math.random();
  await later(1);
  print(a, b, Math.random());
}
$dump.p1 = $djs.resume(2, "later", [1]);
$dump.Ka2 = async function () {
const a = 0.05279087335850807;
const b = 0.3311202810018534;
(await $dump.p1);
print(a, b, Math.random());
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, at: "await#2", segs: [99,116,27], scaf: [["site",98,82]] } });
$dump.a2 = $dump.Ka2.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
