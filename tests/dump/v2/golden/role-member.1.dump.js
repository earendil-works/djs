"djs-dump 2";
$dump.p1 = $dump.pending(1, "later", ["x"]);
async function main() { const t = { x: 9 }; return t[await later("x")]; }
$dump.n2 = { x: 9 };
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { t: $dump.n2 }, operands: [["member.object", $dump.n2]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
