"djs-dump";
"use strict";
$djs.seed(2033933741, -1539143467, 3);
globalThis.$dump = {};
$dump.p1 = $djs.resume(2, "call", ["call", "my-tool", undefined], {"id":2});
$dump.a2_0 = { all: $djs.host("ALL_TOOLS") };
$dump.Fa2 = $djs.source("codemode.js", "f593729dd2bd2a94", "", 1);
$dump.a2 = $djs.k(async (tools) => {
const $djs_D = $djs.D;
const $djs_t0 = $dump.a2_0;
return ($djs_D($djs_t0, "calls", ["dash", await $dump.p1, await tools.mcp__docs__search()]), $djs_t0);
}, { f: $dump.Fa2, at: "await#2", segs: [128,165,32], scaf: [["tmp",0,104,"array.element@116","array.element"],["site",139,112],["tmp",0,112,"array.element@139","array.element"],["tmp",0,103,"array.literal@115","array.literal"],["k",0,78],["k",0,85],["k",0,94]], cmap: [["object.literal@80",0,"$djs_t0"],["array.element@116",1,"dash"]] })($djs.host("tools"));
try { ($dump.a2); } finally { delete globalThis.$dump; delete globalThis.$djs; }
