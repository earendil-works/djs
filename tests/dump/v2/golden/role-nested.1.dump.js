"djs-dump 2";
$dump.fn = (o) => {
  const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]);
  return d.value || d.get || d.set;
};
$dump.p1 = $dump.pending(1, "later", [3]);
$dump.n2 = $dump.fn({ f(a, b) { return a + "|" + JSON.stringify(b); } });
const o = { f: $dump.n2 };
async function main() {
  const p = "x";
  return o.f(p + "!", [1, { k: `v${2}w${await later(3)}` }]);
}
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { p: "x" }, operands: [["call.callee", $dump.n2], ["call.this", o], ["call.arg[0]", "x!"], ["array.element[0]", 1], ["object.literal", {}], ["template.part[0]", "v"], ["template.part[1]", "2"], ["template.part[2]", "w"]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
