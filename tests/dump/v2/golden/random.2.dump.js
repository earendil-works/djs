"djs-dump 2";
$dump.random(1473515859, 1479578119);
$dump.p1 = $dump.pending(2, "later", [1]);
async function main() {
  const a = Math.random();
  await later(0);
  const b = Math.random();
  await later(1);
  print(a, b, Math.random());
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, bindings: { a: 0.05279087335850807, b: 0.3311202810018534 } });
