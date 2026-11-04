"djs-dump";
"use strict";
$djs.seed(-29772990, 66527711, 2);
globalThis.$dump = {};
async function main() {
  const a = Math.random();
  await later(0);
  const b = Math.random();
  await later(1);
  print(a, b, Math.random());
}
$dump.p1 = $djs.resume(1, "later", [0]);
$dump.Ka2 = async function () {
const a = 0.05279087335850807;
(await $dump.p1);
const b = Math.random();
await later(1);
print(a, b, Math.random());
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [69,71,24,94,98,15,110,116,27], scaf: [["site",53,52]] } });
$dump.a2 = $dump.Ka2.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
