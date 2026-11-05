"djs-dump";
"use strict";
$djs.seed(2033933741,-1539143467);
$djs.k(async (tools) => {
return { all: ALL_TOOLS, calls: ["dash", await $djs.resume(2,"call",["call","my-tool",undefined],{"id":2}), await tools.mcp__docs__search()] };
},$djs.source("codemode.js","f593729dd2bd2a94","",1),2)($djs.host("tools"))
