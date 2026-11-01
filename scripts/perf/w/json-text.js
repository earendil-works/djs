// JSON round trips of a record with a multi-line text field (pi-like's read()).
const LINES = Array.from({ length: 60 }, (_, i) => (i % 7 === 0 ? `  throw new Error("case ${i}");` : `  const value${i} = compute(${i}, "text");`));
let total = 0;
for (let r = 0; r < 3000; r++) {
  const rec = { path: `src/module${r % 60}/file${r % 60}.ts`, text: LINES.slice(0, 20 + (r % 40)).join("\n") };
  const back = JSON.parse(JSON.stringify(rec));
  total += back.text.length + back.path.length;
}
return total;
