"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
$dump.n2 = $dump.fn({ f(a, b, c) { return [this.k, a, b, c].join(); } });
const o = { k: 1, f: $dump.n2 };
async function main() { return o.f(1, await later(2), 3); }
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { "call.callee@31": $dump.n2, "call.this@31": o, "call.arg@35": 1 } });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
