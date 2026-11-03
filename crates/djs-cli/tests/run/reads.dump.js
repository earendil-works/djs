"djs-dump 2";
async function main() {
  const a = await tools.read({ path: "a" });
  const b = await tools.read({ path: "b" });
  print(a + b);
  return a.length + b.length;
}
$dump.p2 = $dump.pending(2, "tools.read", [{ path: "a" }]);
$dump.run = $dump.resume(main, {
  at: "await#1",
  awaiting: $dump.p2,
  this: undefined,
  args: [],
  bindings: { a: $dump.TDZ, b: $dump.TDZ },
});
$dump.run.then((n) => print("total", n));
$dump.handle("run", $dump.run);
