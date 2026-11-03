"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
async function main() { return `a${1}b${await later(2)}c`; }
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, operands: [["template.part[0]", "a"], ["template.part[1]", "1"], ["template.part[2]", "b"]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
