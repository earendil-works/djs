"djs-dump 2";
const sleep = $dump.host("sleep", true);
async function main() {
  await sleep(100);
  print("woke");
}
$dump.p2 = $dump.pending(2, "sleep", [100]);
$dump.run = $dump.resume(main, {
  at: "await#1",
  awaiting: $dump.p2,
  this: undefined,
  args: [],
  slots: {},
});
$dump.handle("run", $dump.run);
