"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["s2"]);
var agen = async function* agen() {
  yield await later("a1");
  yield "a2";
};
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
$dump.n2 = ["s1", $dump.p1];
$dump.i3 = $dump.advance(Array.prototype.values.call($dump.n2), 2);
$dump.p4 = Promise.prototype.then.call($dump.p1, (v) => ({ value: v, done: false }), $dump.closeThrow($dump.i3));
$dump.i5 = $dump.fromSync($dump.i3, $dump.builtin("%ArrayIteratorPrototype%.next"));
$dump.a6 = $dump.resume(main, { at: "for-await.next#1", awaiting: $dump.p4, slots: { out: ["s1"], x: "s1", "for-await.iterator@44": $dump.i5, "for-await.next@44": $dump.i5.next } });
