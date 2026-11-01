// Allocations near the heap cap inside native builtins: map's result array grows by
// doubling while its callback makes garbage, so a growth can hit the cap between two
// collections. Each failing allocation collects and retries (D1900, as D1700 does for
// bytecode) instead of throwing while garbage remains.
const out = [];
const src = [];
for (let i = 0; i < 70000; i++) src.push(i);
for (let round = 0; round < 6; round++) {
  try {
    const m = src.map((x) => {
      const t = { x, s: "g" + x };
      return t.x + round;
    });
    const f = m.filter((x) => x % 3 !== 0);
    out.push(m.length + f.length > 0 ? "." : "?");
  } catch (e) {
    out.push("E");
  }
}
console.log(out.join(""));
