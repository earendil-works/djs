"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [1]);
var gen = function* gen() { yield "g1"; yield "g2"; };
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.n2 = [];
$dump.i3 = $dump.advance(Array.prototype.values.call([1, 2]), 1);
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { out: $dump.n2, x: 1, "for-of.iterator@44": $dump.i3, "for-of.next@44": $dump.builtin("%ArrayIteratorPrototype%.next"), "call.callee@68": Array.prototype.push, "call.this@68": $dump.n2 } });
