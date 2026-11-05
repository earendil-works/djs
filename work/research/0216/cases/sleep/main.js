"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 3);
globalThis.$dump = {};
$dump.p1 = $djs.resume(2, "tools.sleep", [5000], {"due":1793822825106});
$dump.n2 = async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
};
$dump.a3 = $djs.k(async function (label) {
const r = await $dump.p1;
return label + ": " + r;
}, { f: $dump.n2, segs: [31,83,10,51,112,26], scaf: [["site",87,35]] })("first");
$dump.n4 = [];
$dump.i5 = $djs.advance(Array.prototype.values.call(["first", "second"]), 1);
$dump.Fa6 = $djs.source("sleep.js", "b48bd687f864a8d6", "ms");
$dump.a6 = $djs.k(async function (ms, step, t0, results) {
const $djs_I = $djs.I;
const $djs_i1 = $dump.i5;
results.push(await $dump.a3);
for (const label of $djs_I($djs_i1)) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;
}, { f: $dump.Fa6, segs: [90,250,19,117,284,3,120,205,16,157,246,174], scaf: [["for-of",205,120,0,"$djs_i1"],["site",263,103]] })(5000, $dump.n2, 1793822820106, $dump.n4);
try { ($dump.a6); } finally { delete globalThis.$dump; delete globalThis.$djs; }
