// map/filter/reduce/forEach/sort over arrays: what codemode scripts do most.
// bench: expect 3334270772
const xs = [];
for (let i = 0; i < 200_000; i++) xs.push((i * 7919) % 100_003);
let total = 0;
for (let r = 0; r < 5; r++) {
  const ys = xs.map((x) => x * 2).filter((x) => x % 3 === 0);
  total += ys.reduce((a, b) => a + b, 0) % 1_000_000_007;
  xs.forEach((x) => { if (x === 42) total++; });
}
const sorted = xs.slice(0, 50_000).sort((a, b) => a - b);
return total + sorted[25_000];
