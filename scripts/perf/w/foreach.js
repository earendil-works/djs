// Array.prototype.forEach, indexOf, includes, join and for-of (0063: native builtins
// and the array iterator).
// bench: expect 208843775
const xs = [];
for (let i = 0; i < 200_000; i++) xs.push(i % 977);
let total = 0;
for (let r = 0; r < 10; r++) {
  xs.forEach((x, i) => { if (x === r) total += i; });
  for (const x of xs) total += x & r;
  total += xs.indexOf(976 - r) + (xs.includes(-1) ? 1 : 0);
}
return total + xs.slice(0, 10_000).join(",").length;
