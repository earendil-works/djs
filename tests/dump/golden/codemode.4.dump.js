"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 8);
globalThis.$dump = {};
$dump.s2 = {};
$dump.p1 = $djs.resume(7, "later", [0]);
$dump.a3_0 = tools;
$dump.a3_1 = console;
$dump.Ka3 = async function () {
var tools = $dump.a3_0;
var console = $dump.a3_1;
let n = 4;
Object.assign($dump.s2, { total: () => n });
const total = $dump.s2.total;
(await $dump.p1);
console.log("total", total());
return n;
};
$dump.Fa3 = async function (tools, console) {
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
Object.defineProperty($dump.Ka3, "$djs", { value: { f: $dump.Fa3, at: "await#3", segs: [174,299,30,205,330,9], scaf: [["site",283,157]] } });
$dump.a3 = $dump.Ka3.call(globalThis);
try { ($dump.a3); } finally { delete globalThis.$dump; delete globalThis.$djs; }
