"djs-dump";
"use strict";
$djs.seed(1975298774, 1374804631, 4);
globalThis.$dump = {};
$dump.p1 = $djs.resume(2, "call", ["call", "delay", "2"], {"id":2});
$dump.p2 = Promise.all([1, $dump.p1, 3]);
$dump.Fa3 = $djs.source("codemode.js", "54e8cfc5dc328017", "", 1);
$dump.a3 = $djs.k(async (tools) => {
const [a, b, c] = await $dump.p2;
return { values: [a, b, c], names: Object.keys(tools) };
}, { f: $dump.Fa3, segs: [25,38,18,53,122,56], scaf: [["site",50,37]] })($djs.host("tools"));
try { ($dump.a3); } finally { delete globalThis.$dump; delete globalThis.$djs; }
