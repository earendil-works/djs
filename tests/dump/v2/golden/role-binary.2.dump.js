"djs-dump 2";
$dump.p1 = $dump.pending(2, "later", [9]);
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, bindings: { a: 5 }, operands: [["array.element[0]", 30], ["binary.left", 5]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
