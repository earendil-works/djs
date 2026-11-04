"djs-dump 2";
function* letters() {
  yield "a";
  yield "b";
}
async function main() {
  const m = new Map([["x", 1], ["y", 2]]);
  for (const [k, v] of m) {
    for (const l of letters()) {
      print(k, v, l, await later(k + l));
    }
  }
  print("end");
}
$dump.m = new Map([["x", 1], ["y", 2]]);
$dump.outer = $dump.m[Symbol.iterator]();
$dump.outer.next();
$dump.outer.next();
$dump.inner = $dump.resume(letters, { at: "yield#1" });
$dump.handle("main", $dump.resume(main, {
  at: "await#1",
  awaiting: $dump.pending(3, "later", ["ya"]),
  // In another order than the frame's: the loader writes each slot by its name.
  slots: {
    "call.arg@146": "a",
    "call.arg@140": "y",
    "call.arg@143": 2,
    "call.callee@134": print,
    "call.this@134": undefined,
    "for-of.iterator@99": $dump.inner,
    "for-of.next@99": $dump.inner.next,
    "for-of.next@69": $dump.outer.next,
    "for-of.iterator@69": $dump.outer,
    l: "a",
    v: 2,
    k: "y",
    m: $dump.m,
  },
}));
