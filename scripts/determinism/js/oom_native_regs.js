// Collections at the heap cap inside native builtins whose typed locals live in Rust
// variables (D2000): map, filter, reduce, sort and indexOf keep their counters and lengths
// in registers across a collect-and-retry (D1900) of the allocations they make. Each
// round logs checksums, so a stale register shows as a wrong number, not just an error.
const out = [];
const src = [];
for (let i = 0; i < 40000; i++) src.push((i * 7919) % 40009);
for (let round = 0; round < 5; round++) {
  try {
    const m = src.map((x, i) => {
      const t = { x, s: "g" + i };
      return t.x + round + (i & 3);
    });
    const f = m.filter((x, i) => (x + i) % 3 !== 0);
    const r = f.reduce((s, x, i) => (s + x * (i & 7)) % 1000003, 0);
    const s = f.slice(0, 3000).sort((a, b) => {
      const g = { a, b };
      return g.a - g.b;
    });
    out.push(`${m.length}:${f.length}:${r}:${s[0]}:${s[1500]}:${s.indexOf(s[2999])}`);
  } catch (e) {
    out.push("E");
  }
}
console.log(out.join(" "));
