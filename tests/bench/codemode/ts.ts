// TypeScript: the JSON reshaping of json.js with types (about 2% of the bodies are
// TypeScript). Only erasable syntax, so a host can strip the types for an engine
// without TypeScript.
interface Row { ts: string; service: string; level: "info" | "warn" | "error"; status: number; durationMs: number; route: string; userId: string; message: string }
interface Group { service: string; route: string; n: number; errors: number; durations: number[] }
type Summary = { key: string; n: number; errorRate: number; p50: number; p95: number };

function percentile(xs: readonly number[], p: number): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))] ?? 0;
}
function groupBy<T, K extends string>(items: readonly T[], key: (t: T) => K): Map<K, T[]> {
  const m = new Map<K, T[]>();
  for (const it of items) {
    const k = key(it);
    const list = m.get(k);
    if (list) list.push(it);
    else m.set(k, [it]);
  }
  return m;
}
const res = (await tools.query_rows({ sql: "select * from requests where level != 'debug'", limit: 600 })) as { structuredContent: { rows: Row[] } };
const rows: Row[] = res.structuredContent.rows;
const groups: Group[] = [...groupBy(rows, (r: Row) => `${r.service} ${r.route}`)].map(([k, list]): Group => {
  const [service, route] = k.split(" ") as [string, string];
  return { service, route, n: list.length, errors: list.filter((r) => r.status >= 500).length, durations: list.map((r) => r.durationMs) };
});
const out: Summary[] = groups
  .filter((g: Group) => g.n > 2)
  .map((g): Summary => ({ key: `${g.service}${g.route}`, n: g.n, errorRate: g.errors / g.n, p50: percentile(g.durations, 50), p95: percentile(g.durations, 95) }))
  .sort((a: Summary, b: Summary) => b.p95 - a.p95);
const byLevel = groupBy(rows, (r) => r.level);
const levels = Object.fromEntries([...byLevel].map(([k, v]) => [k, v.length] as const));
text(out.slice(0, 20).map((s) => `${s.key.padEnd(40)} ${String(s.n).padStart(4)} ${(s.errorRate * 100).toFixed(1).padStart(5)}% p50 ${s.p50.toFixed(1)} p95 ${s.p95.toFixed(1)}`).join("\n"));
return { groups: out.length, levels };
