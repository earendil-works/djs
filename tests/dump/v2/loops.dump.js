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
  bindings: { m: $dump.m, k: "y", v: 2, l: "a" },
  loops: [
    { kind: "for-of", iterator: $dump.outer, next: $dump.outer.next },
    { kind: "for-of", iterator: $dump.inner, next: $dump.inner.next },
  ],
  operands: [
    ["call.callee", print],
    ["call.this", undefined],
    ["call.arg[0]", "y"],
    ["call.arg[1]", 2],
    ["call.arg[2]", "a"],
  ],
}));
