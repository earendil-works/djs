"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [6]);
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { a: 5, "binary.left@49": 5 } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
