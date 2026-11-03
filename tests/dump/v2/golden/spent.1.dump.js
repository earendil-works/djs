"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [0]);
let res = $dump.spent(0);
let rej = $dump.spent(1);
const p = Promise.resolve("first");
async function main() {
  await later(0);
  res("again");
  rej(new Error("late"));
  print(await p);
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1 });
