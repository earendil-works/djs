"djs-dump 2";
$dump.fn = (o) => {
  const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]);
  return d.value || d.get || d.set;
};
$dump.p1 = $dump.pending(1, "later", [2]);
$dump.n2 = $dump.fn({ f(a, b, c) { return [this.k, a, b, c].join(); } });
const o = { k: 1, f: $dump.n2 };
async function main() { return o.f(1, await later(2), 3); }
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, operands: [["call.callee", $dump.n2], ["call.this", o], ["call.arg[0]", 1]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
