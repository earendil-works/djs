"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(7, "later", ["yq"]);
async function main() {
  let i = 0;
  while (i < 2) {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  }
}
$dump.i2 = $dump.advance(Array.prototype.values.call(["x", "y"]), 2);
$dump.i3 = $dump.advance(String.prototype[Symbol.iterator].call("pq"), 2);
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { i: 0, a: "y", b: "q" }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", 0], ["call.arg[1]", "y"], ["call.arg[2]", "q"]], loops: [{ kind: "for-of", iterator: $dump.i2, next: $dump.builtin("%ArrayIteratorPrototype%.next") }, { kind: "for-of", iterator: $dump.i3, next: $dump.builtin("%StringIteratorPrototype%.next") }] });
