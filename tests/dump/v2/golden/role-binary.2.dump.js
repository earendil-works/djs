"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [9]);
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, slots: { a: 5, "array.element@49": 30, "binary.left@71": 5 } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
