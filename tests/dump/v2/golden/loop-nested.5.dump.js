"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(9, "later", ["xp"]);
async function main() {
  let i = 0;
  while (i < 2) {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  }
}
$dump.i2 = $dump.advance(Array.prototype.values.call(["x", "y"]), 1);
$dump.i3 = $dump.advance(String.prototype[Symbol.iterator].call("pq"), 1);
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { i: 1, a: "x", b: "p" }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", 1], ["call.arg[1]", "x"], ["call.arg[2]", "p"]], loops: [{ kind: "for-of", iterator: $dump.i2, next: $dump.builtin("%ArrayIteratorPrototype%.next") }, { kind: "for-of", iterator: $dump.i3, next: $dump.builtin("%StringIteratorPrototype%.next") }] });
