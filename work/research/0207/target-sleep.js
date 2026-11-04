"djs-dump";
$djs.seed(-1475731142, -2125276727, 3);
const step = async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
};
const a1 = (async (label, ms) => {
  const r = await $djs.resume(2, "tools.sleep", [200], {"due":1793822820316});
  return label + ": " + r;
})("first", 200);
(async (ms) => {
  const t0 = 1793822820106;
  const results = [];
  const it = $djs.advance(["first", "second"].values(), 1);
  results.push(await a1);
  for (const label of $djs.rest(it)) results.push(await step(label, ms));
  console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
  return results;
})(200);
