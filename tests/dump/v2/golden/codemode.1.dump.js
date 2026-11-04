"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.p1 = $djs.resume(2, "later", [["a", "b"]]);
$dump.a2_0 = tools;
$dump.a2_1 = console;
$dump.Ka2 = async function () {
var tools = $dump.a2_0;
var console = $dump.a2_1;
let n = 0;
for (const f of (await $dump.p1)) {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
}
const total = () => n;
await later(0);
console.log("total", total());
return n;
};
$dump.Fa2 = async function (tools, console) {
// A codemode body (pi's Sandbox): tools and console are its parameters.
console.log("start");
let n = 0;
for (const f of await tools.list()) {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
}
const total = () => n;
await later(0);
console.log("total", total());
return n;

};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: $dump.Fa2, segs: [81,140,16,113,174,85,199,260,22,222,283,15,238,299,30,269,330,9], scaf: [["site",156,98],["loop",140,81]] } });
$dump.a2 = $dump.Ka2.call(globalThis);
try { ($dump.a2); } finally { delete globalThis.$dump; delete globalThis.$djs; }
