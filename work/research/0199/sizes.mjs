// sizes.mjs: sizes of today's format 2 dumps vs plain dumps of the same state (raw and
// zstd -3), per corpus; plus the plain variants without pruning / pieces, and the time the
// JS transform takes. Writes sizes.json.
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { here } from "./lib.mjs";
import { toPlain } from "./plain.mjs";

const zstd = (b) => execFileSync("zstd", ["-3", "-c", "-q"], { input: b, maxBuffer: 64 << 20 }).length;
const files = fs.readdirSync(path.join(here, "today")).filter((f) => f.endsWith(".dump.js")).sort();
const rows = [];
for (const f of files) {
  const d = fs.readFileSync(path.join(here, "today", f), "utf8");
  let p, p0, t0;
  try {
    const s = performance.now();
    p = toPlain(d).text;
    t0 = performance.now() - s;
    p0 = toPlain(d, { prune: false, pieces: false }).text;
  } catch {
    continue;
  }
  rows.push({ f, h: f.startsWith("h"), d: d.length, p: p.length, p0: p0.length, zd: zstd(d), zp: zstd(p), zp0: zstd(p0), ms: t0 });
}
fs.writeFileSync(path.join(here, "sizes.json"), JSON.stringify(rows));
const med = (a) => [...a].sort((x, y) => x - y)[a.length >> 1];
for (const [label, sel] of [["harvested", (r) => r.h], ["v2/golden", (r) => !r.h]]) {
  const rs = rows.filter(sel);
  const sum = (k) => rs.reduce((a, r) => a + r[k], 0);
  const pct = (a, b) => `${((sum(b) / sum(a) - 1) * 100).toFixed(1)}%`;
  console.log(
    `${label}: ${rs.length} dumps; raw today ${sum("d")} -> plain ${sum("p")} (${pct("d", "p")}), unpruned ${sum("p0")} (${pct("d", "p0")}); ` +
      `zstd ${sum("zd")} -> ${sum("zp")} (${pct("zd", "zp")}), unpruned ${sum("zp0")} (${pct("zd", "zp0")}); ` +
      `per dump raw ratio median ${med(rs.map((r) => r.p / r.d)).toFixed(2)}, zstd ${med(rs.map((r) => r.zp / r.zd)).toFixed(2)}; transform median ${med(rs.map((r) => r.ms)).toFixed(1)} ms`,
  );
}
