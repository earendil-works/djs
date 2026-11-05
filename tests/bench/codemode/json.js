// JSON reshaping of tool results: query rows grouped and sorted, a package file parsed,
// changed and stringified for an edit (about 7% of the bodies).
const res = await tools.query_rows({ sql: "select * from requests where day >= '2026-10-01'", limit: 600 });
const rows = res.structuredContent.rows;
const by = new Map();
for (const r of rows) {
  const k = r.service + " " + r.route;
  const g = by.get(k) ?? { service: r.service, route: r.route, n: 0, errors: 0, total: 0, max: 0, users: new Set() };
  g.n++;
  if (r.status >= 500 || r.level === "error") g.errors++;
  g.total += r.durationMs;
  g.max = Math.max(g.max, r.durationMs);
  g.users.add(r.userId);
  by.set(k, g);
}
const summary = [...by.values()]
  .map((g) => ({ service: g.service, route: g.route, n: g.n, errorRate: Math.floor((g.errors * 1000) / g.n + 0.5) / 1000, avgMs: Math.floor((g.total * 10) / g.n + 0.5) / 10, maxMs: g.max, users: g.users.size }))
  .filter((g) => g.n >= 3)
  .sort((a, b) => b.errorRate - a.errorRate || b.avgMs - a.avgMs);
const pkgText = (await tools.bash({ command: "cat packages/server/package.json", timeout: 10 })).output;
const pkg = JSON.parse(pkgText);
const scripts = Object.fromEntries(Object.entries(pkg.scripts).map(([k, v]) => [k, v.replaceAll("shx rm -rf", "node ../../scripts/clean.mjs").replaceAll("shx chmod +x", "node ../../scripts/chmod.mjs")]));
const changed = Object.keys(scripts).filter((k) => scripts[k] !== pkg.scripts[k]);
if (changed.length) await tools.edit({ path: "packages/server/package.json", edits: [{ oldText: JSON.stringify(pkg.scripts, null, 2), newText: JSON.stringify(scripts, null, 2) }] });
return JSON.stringify({ top: summary.slice(0, 15), groups: summary.length, rows: rows.length, changed }, null, 2);
