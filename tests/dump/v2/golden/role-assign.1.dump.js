"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [1]);
async function main() {
  const t = { r: 1 }, k = "q";
  let n = 1;
  t.p = await later(1);
  t[k] = await later(2);
  n += await later(4);
  t.r += await later(2);
  return JSON.stringify(t) + " " + n;
}
$dump.n2 = { r: 1 };
$dump.a3 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { t: $dump.n2, k: "q", n: 1, "assign.object@70": $dump.n2 } });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
