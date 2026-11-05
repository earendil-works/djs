"djs-dump";
"use strict";
$djs.seed(67108866, 0, 3);
globalThis.$dump = {};
$dump.s3 = {};
$dump.p1 = $djs.resume(1, "call", ["call", "wait", "\"a\""], {"id":1});
$dump.p2 = $djs.resume(2, "call", ["call", "wait", "\"b\""], {"id":2});
$dump.ga7 = () => {
$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.s3.$f4, undefined);
$dump.p5 = Promise.prototype.then.call($dump.p2, $dump.s3.$f6, undefined);
$dump.p6 = Promise.all([$dump.p4, $dump.p5]);
};
$dump.Fa7 = $djs.source("codemode.js", "d2f77c9ed6fbafb9", "", 1);
$dump.a7 = $djs.k(async () => {
const order = [];
Object.assign($dump.s3, { $f4: (0, () => order.push("a")), $f6: (0, () => order.push("b")) });
$dump.ga7();
await $dump.p6;
return order;
}, { f: $dump.Fa7, segs: [156,164,15], scaf: [["site",49,140]] })();
try { ($dump.a7); } finally { delete globalThis.$dump; delete globalThis.$djs; }
