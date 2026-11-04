"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["x"]);
async function main() { const t = { x: 9 }; return t[await later("x")]; }
$dump.n2 = { x: 9 };
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { t: $dump.n2, "member.object@51": $dump.n2 } });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
