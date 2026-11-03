"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [2]);
var gen = function* gen() { yield "g1"; yield "g2"; };
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.n2 = [1];
$dump.i3 = $dump.advance(Array.prototype.values.call([1, 2]), 2);
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { out: $dump.n2, x: 2 }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n2]], loops: [{ kind: "for-of", iterator: $dump.i3, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
