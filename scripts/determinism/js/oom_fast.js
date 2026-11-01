// Allocations near the heap cap inside @fast entries (D1910): slice copies and joins of
// a large array between bursts of garbage. An entry that reaches the cap declines, and
// the builtin's own path collects and retries (D1700, D1900) instead of throwing while
// garbage remains.
const out = [];
const src = [];
for (let i = 0; i < 40000; i++) src.push(i);
for (let round = 0; round < 8; round++) {
  try {
    let g = [];
    for (let i = 0; i < 3000; i++) g.push({ i, s: "g" + i });
    g = null;
    const c = src.slice();
    const s = src.join(",");
    out.push(c.length === src.length && s.length > 0 ? "." : "?");
  } catch (e) {
    out.push("E");
  }
}
console.log(out.join(""));
