// A codemode-like body: tool results as data, then map/filter/sort/split/format/JSON.
const LINES = Array.from({ length: 120 }, (_, i) => (i % 7 === 0 ? `  throw new Error("case ${i}");` : `  const value${i} = compute(${i}, "text");`));
function search() { return Array.from({ length: 60 }, (_, i) => ({ path: `src/module${i}/file${i}.ts`, score: 1 / (i + 1) })); }
function read(a) { const n = Number(/\d+/.exec(a.path)[0]); return JSON.parse(JSON.stringify({ path: a.path, text: LINES.slice(0, 20 + n).join("\n") })); }
let out = [];
for (let rep = 0; rep < 150; rep++) {
  const hits = JSON.parse(JSON.stringify(search()));
  const files = [];
  for (let i = 0; i < hits.length; i += 10) {
    const batch = hits.slice(i, i + 10).map((hit) => read({ path: hit.path }));
    files.push(...batch);
  }
  const matches = files
    .map((file) => ({ path: file.path, lines: file.text.split("\n").filter((line) => line.includes("throw")) }))
    .filter((file) => file.lines.length > 0)
    .sort((a, b) => b.lines.length - a.lines.length);
  out = [];
  for (const file of matches.slice(0, 10)) out.push(file.path.padEnd(40) + String(file.lines.length).padStart(4));
  const summary = { files: files.length, matching: matches.length, top: matches[0]?.path, seen: matches.map((f) => f.path).slice(0, 20) };
  out.push(JSON.stringify(summary));
}
return out.length;
