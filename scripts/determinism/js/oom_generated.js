// Allocations near the heap cap inside generated (`@rust`) builtins (D3260): large case
// conversions, pads and trims between bursts of garbage. A builtin that reaches the cap
// has the interpreter collect and calls it once more, instead of throwing while garbage
// remains.
const out = [];
const big = "Ab ".repeat(100000);
for (let round = 0; round < 8; round++) {
  try {
    let g = [];
    for (let i = 0; i < 3000; i++) g.push({ i, s: "g" + i });
    g = null;
    const l = big.toLowerCase();
    const p = big.padEnd(400000, "xy");
    const t = (" " + big + " ").trim();
    out.push(l.length === big.length && p.length === 400000 && t.length === big.length - 1 ? "." : "?");
  } catch (e) {
    out.push("E");
  }
}
console.log(out.join(""));
