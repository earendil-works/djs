"djs-dump 2";
$dump.advance = (it, n) => {
  for (let i = 0; i < n; i++) it.next();
  return it;
};
$dump.p1 = $dump.pending(1, "later", [0]);
async function main() {
  const ai = [1, 2, 3].values(); ai.next();
  const si = "h\u00e9\u{1F600}x"[Symbol.iterator](); si.next(); si.next();
  const mi = new Map([["a", 1], ["b", 2]]).entries(); mi.next();
  const ti = new Set(["s1", "s2"]).values(); ti.next();
  await later(0);
  print(ai.next().value, si.next().value, JSON.stringify(mi.next().value), ti.next().value);
}
$dump.i2 = $dump.advance(Array.prototype.values.call([1, 2, 3]), 1);
$dump.i3 = $dump.advance(String.prototype[Symbol.iterator].call("hé😀x"), 2);
$dump.i4 = $dump.advance(Map.prototype.entries.call(new Map([["a", 1], ["b", 2]])), 1);
$dump.i5 = $dump.advance(Set.prototype.values.call(new Set(["s1", "s2"])), 1);
$dump.a6 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { ai: $dump.i2, si: $dump.i3, mi: $dump.i4, ti: $dump.i5 } });
