"djs-dump 2";
$dump.p1 = $dump.pending(1, "later", [10]);
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
$dump.a2 = $dump.resume(step, { at: "await#1", awaiting: $dump.p1, args: ["a", 10], bindings: { label: "a", ms: 10 } });
$dump.n3 = [];
$dump.i4 = $dump.advance(Array.prototype.values.call(["a", "b", "c"]), 1);
$dump.a5 = $dump.resume(main, { at: "await#1", awaiting: $dump.a2, bindings: { results: $dump.n3, label: "a" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n3]], loops: [{ kind: "for-of", iterator: $dump.i4, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
