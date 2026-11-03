// A helper awaiting a sleep (an open call), called from a for-of that pushes each
// result: dumped during a sleep, the caller holds a half-done `results.push(` (its
// callee and `this`) and the loop's iterator.
const sleep = (ms) => later(ms);
async function step(label, ms) {
  await sleep(ms);
  print("done", label);
  return label + ":" + ms;
}
async function main() {
  const results = [];
  for (const label of ["a", "b", "c"]) {
    results.push(await step(label, 10));
  }
  print(JSON.stringify(results));
}
main();
