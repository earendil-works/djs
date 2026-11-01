// A script in the style models write: helpers, string building, top-level return.
const raw = await tools.echo({
  stdout: "src/a.ts:12: TODO fix\nsrc/b.ts:3: TODO test\nsrc/a.ts:40: TODO docs\n",
});
const byFile = {};
for (const line of raw.stdout.trim().split("\n")) {
  const [file, lineNo, ...rest] = line.split(":");
  (byFile[file] ??= []).push({ line: Number(lineNo), text: rest.join(":").trim() });
}
let report = "";
for (const [file, items] of Object.entries(byFile).sort()) {
  report += `${file} (${items.length})\n`;
  for (const { line, text: t } of items) report += `  ${String(line).padStart(3)} ${t}\n`;
}
text(report.trimEnd());
const summary = Object.fromEntries(Object.entries(byFile).map(([k, v]) => [k, v.length]));
console.log("summary", summary);
return summary;
