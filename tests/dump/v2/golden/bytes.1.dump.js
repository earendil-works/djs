"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const u = new Uint16Array(4);
  u[1] = 513;
  await later(0);
  print(new Uint8Array(buf).join(), view.getUint8(1), Array.from(u).join());
}
$dump.p1 = $djs.resume(1, "later", [0]);
const buf = $djs.bytes("AQL6AAc=");
const view = new DataView(buf, 1, 3);
$dump.a2_0 = new Uint16Array([0, 513, 0, 0]);
$dump.Ka2 = async function () {
const u = $dump.a2_0;
(await $dump.p1);
print(new Uint8Array(buf).join(), view.getUint8(1), Array.from(u).join());
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: main, segs: [60,90,74], scaf: [["site",72,43]] } });
$dump.a2 = $dump.Ka2.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
