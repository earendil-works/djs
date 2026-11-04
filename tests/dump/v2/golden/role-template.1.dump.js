"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
async function main() { return `a${1}b${await later(2)}c`; }
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { "template.part@31": "a", "template.part@35": "1", "template.part@36": "b" } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
