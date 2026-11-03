"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
async function main() {
  const k = "z";
  const a = { a: 1, [k]: await later(2) };
  const b = { a: 1, c: await later(3) };
  return JSON.stringify([a, b]);
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { k: "z", a: undefined, b: undefined }, operands: [["object.literal", { a: 1 }], ["object.key", "z"]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
