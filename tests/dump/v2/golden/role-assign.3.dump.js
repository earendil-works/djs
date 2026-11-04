"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(3, "later", [4]);
async function main() {
  const t = { r: 1 }, k = "q";
  let n = 1;
  t.p = await later(1);
  t[k] = await later(2);
  n += await later(4);
  t.r += await later(2);
  return JSON.stringify(t) + " " + n;
}
$dump.a2 = $dump.resume(main, { at: "await#3", awaiting: $dump.p1, slots: { t: { r: 1, p: 1, q: 2 }, k: "q", n: 1, "assign.old@119": 1 } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
