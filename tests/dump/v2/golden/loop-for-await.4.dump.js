"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", ["a2!"]);
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
$dump.n2 = ["s1", "s2", "a1!"];
$dump.a3 = $dump.resume(agen, { at: "yield#2" });
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { out: $dump.n2, x: "a2", "for-await.iterator@102": $dump.a3, "for-await.next@102": $dump.builtin("%AsyncGeneratorPrototype%.next"), "call.callee@132": Array.prototype.push, "call.this@132": $dump.n2 } });
