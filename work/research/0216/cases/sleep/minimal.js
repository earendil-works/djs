"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
$djs.k(async function (ms, step, t0, results, $i) {
results.push(await $djs.k(async function (label) {
const r = await $djs.resume(2,"tools.sleep",[5000],{"due":1793822825106});
return label + ": " + r;
},step)("first"));
for (const label of $i) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;
}, $djs.source("sleep.js","b48bd687f864a8d6","ms"))(5000,async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
},1793822820106,[],["second"].values())
