"djs-dump 2";
$dump.p1 = $dump.pending(4, "later", [2]);
async function main() {
  const t = { r: 1 }, k = "q";
  let n = 1;
  t.p = await later(1);
  t[k] = await later(2);
  n += await later(4);
  t.r += await later(2);
  return JSON.stringify(t) + " " + n;
}
$dump.n2 = { r: 1, p: 1, q: 2 };
$dump.a3 = $dump.resume(main, { at: "await#4", awaiting: $dump.p1, bindings: { t: $dump.n2, k: "q", n: 5 }, operands: [["assign.object", $dump.n2], ["assign.old", 1]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
