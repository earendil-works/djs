// String and regex work over command output and file contents: parse grep-like output,
// group by file, scan files for markers and definitions (about 8% of the bodies).
const hits = (await tools.bash({ command: "rg -n resolveToken src lib test", timeout: 30 })).output;
const byFile = new Map();
for (const line of hits.split("\n")) {
  const m = /^([^:]+):(\d+):(.*)$/.exec(line);
  if (!m) continue;
  const list = byFile.get(m[1]) ?? [];
  list.push({ line: Number(m[2]), code: m[3].trim() });
  byFile.set(m[1], list);
}
const top = [...byFile].sort((a, b) => b[1].length - a[1].length).slice(0, 4);
const report = [];
for (const [file, list] of top) {
  const src = await tools.read({ path: file });
  const lines = src.split("\n");
  const markers = lines.map((l, i) => [i + 1, l]).filter(([, l]) => /\b(TODO|FIXME)\b/.test(l));
  const defs = [...src.matchAll(/^export (?:async )?function (\w+)\((\w+)/gm)].map((m) => `${m[1]}(${m[2]})`);
  const renamed = src.replaceAll("Error(", "SessionError(").length - src.length;
  report.push(`${file}: ${list.length} hits, ${lines.length} lines, ${defs.length} functions, +${renamed} chars renamed`);
  for (const [n, l] of markers.slice(0, 5)) report.push(`  ${String(n).padStart(4)}: ${l.trim().slice(0, 100)}`);
  report.push("  defs: " + defs.slice(0, 8).join(", "));
}
text(`${byFile.size} files, ${hits.split("\n").length - 1} hits\n` + report.join("\n"));
