"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [3]);
async function main() {
  const a = [1, 2, await later(3)];
  const b = [1, , await later(3)];
  return a.join() + " " + b.length;
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, slots: { a: [1, 2, 3], b: undefined, "array.literal@72": [1, , ] } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
