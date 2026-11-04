"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", [2]);
var gen = function* gen() { yield "g1"; yield "g2"; };
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.n2 = [1, 2, "a1"];
$dump.i3 = $dump.advance(Map.prototype.entries.call(new Map([["a", 1], ["b", 2]])), 2);
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, slots: { out: $dump.n2, k: "b", v: 2, "for-of.iterator@96": $dump.i3, "for-of.next@96": $dump.builtin("%MapIteratorPrototype%.next"), "call.callee@148": Array.prototype.push, "call.this@148": $dump.n2, "binary.left@157": "b" } });
