// One big tool result (a ~2 MB log) kept in a variable while the script asks follow-ups.
const log = await tools.logs({ service: "api", lines: 20000 });
const errors = log.split("\n").filter((l) => l.includes(" ERROR "));
const byCode = new Map();
for (const line of errors) {
  const code = /code=(\w+)/.exec(line)?.[1] ?? "unknown";
  if (!byCode.has(code)) byCode.set(code, []);
  byCode.get(code).push(line);
}
const report = [];
for (const [code, lines] of [...byCode].sort((a, b) => b[1].length - a[1].length).slice(0, 5)) {
  const info = await tools.lookup({ code });
  report.push({ code, count: lines.length, owner: info.owner, sample: lines[0].slice(0, 120) });
}
console.log(report);
return { lines: log.length, errors: errors.length, report };
