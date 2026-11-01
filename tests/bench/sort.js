// Array.prototype.sort with a comparator and on strings (0063: native SortList).
// bench: expect 250008
const xs = [];
for (let i = 0; i < 60_000; i++) xs.push((i * 7919) % 100_003);
const a = xs.slice().sort((x, y) => x - y);
const s = xs.slice(0, 20_000).map(String).sort();
return a[30_000] + a[59_999] + s[10_000].length + Number(s[19_999]);
