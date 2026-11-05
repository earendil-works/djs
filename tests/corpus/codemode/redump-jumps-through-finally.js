// A `break`/`continue` through a finally that awaits, in a later iteration of the loop
// (K runs it as copied text, whose completion names K's statement, not the scaffold's
// constant: refused at a re-dump, 0215 from pjz); and an accessor among an array's
// elements (written as a hole, its getter lost, 0215 from pjz).
for (const x of [1, 9]) { try { continue; } finally { console.log(await tools.echo(x)); } }
const out = [];
outer: for (const y of [1, 2]) {
  for (const x of [3, 4, 5]) {
    try { if (x > 4) break; if (x > 3) continue outer; out.push(x); }
    finally { out.push(await tools.echo(y * 10 + x)); }
  }
}
console.log(out.join());
let n = 1;
const v = Object.defineProperty([5, 6], 2, { get: () => n, enumerable: true, configurable: true });
v.push(7);
const w = Object.defineProperty([5], 1, { get: () => n, set(x) { n = x; }, enumerable: false, configurable: true });
w.push(8);
await tools.echo(0);
n = 2;
console.log(JSON.stringify(v), JSON.stringify(w), Object.keys(w).join(), w.length);
w[1] = 3;
console.log(n, JSON.stringify(v));
