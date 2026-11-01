// Array.prototype.map, filter and reduce with small callbacks (0063: native builtins).
// bench: expect 750100000
const xs = [];
for (let i = 0; i < 200_000; i++) xs.push(i % 1000);
let total = 0;
for (let r = 0; r < 5; r++) {
  const ys = xs.map((x) => x * 3 + r).filter((x) => (x & 1) === 0);
  total = (total + ys.reduce((a, b) => a + b, 0)) % 1_000_000_007;
}
return total;
