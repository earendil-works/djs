"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [3]);
async function main() {
  const k = "z";
  const a = { a: 1, [k]: await later(2) };
  const b = { a: 1, c: await later(3) };
  return JSON.stringify([a, b]);
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, slots: { k: "z", a: { a: 1, z: 2 }, b: undefined, "object.literal@96": { a: 1 } } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
