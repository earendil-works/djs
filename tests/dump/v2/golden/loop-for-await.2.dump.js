"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", ["a1"]);
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
$dump.p2 = new Promise(() => {});
$dump.a3 = $dump.resume(agen, { at: "await#1", awaiting: $dump.p1, queue: [{ type: "next", value: undefined, promise: $dump.p2 }] });
$dump.a4 = $dump.resume(main, { at: "for-await.next#2", awaiting: $dump.p2, bindings: { out: ["s1", "s2"], x: undefined }, loops: [{ kind: "for-await", iterator: $dump.a3, next: $dump.builtin("%AsyncGeneratorPrototype%.next") }] });
