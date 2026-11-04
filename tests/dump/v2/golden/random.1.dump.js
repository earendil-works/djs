"djs-dump 2";
$dump.random(-29772990, 66527711);
$dump.p1 = $dump.pending(1, "later", [0]);
async function main() {
  const a = Math.random();
  await later(0);
  const b = Math.random();
  await later(1);
  print(a, b, Math.random());
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { a: 0.05279087335850807, b: undefined } });
