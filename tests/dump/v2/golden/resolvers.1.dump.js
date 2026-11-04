"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
async function main() {
  const v = await later("go");
  keep.res(v + " resolved");
  print(await held);
}
$dump.p1 = $djs.resume(1, "later", ["go"]);
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
let keep = { res: $dump.r3[0], rej: $dump.r3[1] };
const held = $dump.p2;
$dump.Ka4 = async function () {

const v = (await $dump.p1);
keep.res(v + " resolved");
print(await held);
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, segs: [27,32,1,49,57,26,76,86,18], scaf: [["site",36,32]] } });
$dump.a4 = $dump.Ka4.call(undefined);
$dump.p5 = Promise.prototype.then.call($dump.p2, (v) => print("then", v), undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
