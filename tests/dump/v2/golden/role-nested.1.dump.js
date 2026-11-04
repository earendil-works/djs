"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [3]);
$dump.n2 = $dump.fn({ f(a, b) { return a + "|" + JSON.stringify(b); } });
const o = { f: $dump.n2 };
async function main() {
  const p = "x";
  return o.f(p + "!", [1, { k: `v${2}w${await later(3)}` }]);
}
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { p: "x", "call.callee@50": $dump.n2, "call.this@50": o, "call.arg@54": "x!", "array.element@64": 1, "object.literal@67": {}, "template.part@72": "v", "template.part@76": "2", "template.part@77": "w" } });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
