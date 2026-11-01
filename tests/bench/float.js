// Floating point arithmetic and Math (float_arith, math_min).
// bench: expect 12338354.1200
let x = 0.5;
let acc = 0;
for (let i = 1; i < 2_000_000; i++) {
  x = x * 1.000001 + 1 / i;
  acc += Math.sqrt(x) - Math.min(x, 0.25);
}
return acc.toFixed(4);
