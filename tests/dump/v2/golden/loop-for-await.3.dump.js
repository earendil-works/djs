"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(3, "later", ["a1!"]);
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
$dump.n2 = ["s1", "s2"];
$dump.a3 = $dump.resume(agen, { at: "yield#1" });
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { out: $dump.n2, x: "a1" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n2]], loops: [{ kind: "for-await", iterator: $dump.a3, next: $dump.builtin("%AsyncGeneratorPrototype%.next") }] });
