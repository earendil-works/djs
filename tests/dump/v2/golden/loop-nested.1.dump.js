"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["xp"]);
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
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { i: 0, a: "x", b: "p", "for-of.iterator@59": $dump.i2, "for-of.next@59": $dump.builtin("%ArrayIteratorPrototype%.next"), "for-of.iterator@95": $dump.i3, "for-of.next@95": $dump.builtin("%StringIteratorPrototype%.next"), "call.callee@117": print, "call.this@117": undefined, "call.arg@123": 0, "call.arg@126": "x", "call.arg@129": "p" } });
