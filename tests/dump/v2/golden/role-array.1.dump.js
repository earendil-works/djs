"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [3]);
async function main() {
  const a = [1, 2, await later(3)];
  const b = [1, , await later(3)];
  return a.join() + " " + b.length;
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { a: undefined, b: undefined }, operands: [["array.element[0]", 1], ["array.element[1]", 2]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
