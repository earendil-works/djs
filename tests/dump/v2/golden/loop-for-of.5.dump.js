"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(5, "later", ["g1"]);
var gen = function* gen() { yield "g1"; yield "g2"; };
async function main() {
  const out = [];
  for (const x of [1, 2]) out.push(await later(x));
  for (const [k, v] of new Map([["a", 1], ["b", 2]])) out.push(k + (await later(v)));
  for (const g of gen()) out.push(await later(g));
  print(out.join());
}
$dump.n2 = [1, 2, "a1", "b2"];
$dump.a3 = $dump.resume(gen, { at: "yield#1" });
$dump.a4 = $dump.resume(main, { at: "await#3", awaiting: $dump.p1, slots: { out: $dump.n2, g: "g1", "for-of.iterator@182": $dump.a3, "for-of.next@182": $dump.builtin("%GeneratorPrototype%.next"), "call.callee@205": Array.prototype.push, "call.this@205": $dump.n2 } });
