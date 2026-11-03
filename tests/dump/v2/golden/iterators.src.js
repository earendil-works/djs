// Iterators in flight: array, string (code points), Map, Set, advanced.
async function main() {
  const ai = [1, 2, 3].values(); ai.next();
  const si = "h\u00e9\u{1F600}x"[Symbol.iterator](); si.next(); si.next();
  const mi = new Map([["a", 1], ["b", 2]]).entries(); mi.next();
  const ti = new Set(["s1", "s2"]).values(); ti.next();
  await later(0);
  print(ai.next().value, si.next().value, JSON.stringify(mi.next().value), ti.next().value);
}
main();
