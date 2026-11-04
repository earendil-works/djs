"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 6);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.p1 = $djs.resume(5, "later", ["bb"]);
$dump.i2 = $djs.advance(Array.prototype.values.call(["a", "b"]), 2);
$dump.a3_0 = tools;
$dump.a3_1 = console;
$dump.Ka3 = async function () {
const $djs_L = $djs.L;
var tools = $dump.a3_0;
var console = $dump.a3_1;
let n = 2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i2, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const f = "b";

{

const body = (await $dump.p1);
n += body.length;
console.log(f, body);
}
} } else { const f = $djs_v1; {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
} } } }
const total = () => n;
await later(0);
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
Object.defineProperty($dump.Ka3, "$djs", { value: { f: $dump.Fa3, at: "await#2", segs: [237,186,4,262,216,17,280,236,21,315,145,7,334,176,83,424,260,22,447,283,15,463,299,30,494,330,9], scaf: [["for-of",140,126],["site",193,245],["site",259,303],["site",259,305]] } });
$dump.a3 = $dump.Ka3.call(globalThis);
try { ($dump.a3); } finally { delete globalThis.$dump; delete globalThis.$djs; }
