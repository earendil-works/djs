// Garbage followed by large allocations near the heap cap.
const out = [];
for (let round = 0; round < 40; round++) {
  let g = [];
  for (let i = 0; i < 3000; i++) g.push({ i, s: "v" + i });
  g = null;
  try {
    const big = new Array(60000).fill(round);
    out.push(big.length > 0 ? "." : "?");
  } catch (e) {
    out.push("E");
  }
}
console.log(out.join(""));
