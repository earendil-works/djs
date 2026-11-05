// Applies rules.mjs to whole suites of dumps (0216) and reports raw and gzip -9 bytes after
// each rule, cumulatively. With --out DIR, writes the rewritten dumps there (the same tree,
// other files copied) for check runs.
//   node work/research/0216/suite.mjs [--out DIR] [--no-env] SUITE=DIR ...
import fs from "node:fs";
import path from "node:path";
import { gzipSync } from "node:zlib";
import { RULES as ALL } from "./rules.mjs";

const args = process.argv.slice(2);
const oi = args.indexOf("--out");
const outDir = oi >= 0 ? args[oi + 1] : null;
const ri = args.indexOf("--rules");
// --rules N: only the first N rules (the score tool's env scanner reads main's names only)
const RULES = ri >= 0 ? ALL.slice(0, Number(args[ri + 1])) : ALL;
const suites = args.filter((a, i) => a.includes("=") && i !== oi + 1 && i !== ri + 1);
const gz = (t) => gzipSync(t, { level: 9 }).length;
const walk = (d) =>
  fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(d, e.name);
    return e.isDirectory() ? walk(p) : [p];
  });
for (const s of suites) {
  const [name, dir] = s.split("=");
  const files = walk(dir);
  const tot = RULES.map(() => [0, 0]);
  let base = [0, 0];
  let n = 0;
  const failures = [];
  for (const f of files) {
    const dest = outDir ? path.join(outDir, name, path.relative(dir, f)) : null;
    if (dest) fs.mkdirSync(path.dirname(dest), { recursive: true });
    if (!f.endsWith(".js")) {
      if (dest) fs.copyFileSync(f, dest);
      continue;
    }
    let t = fs.readFileSync(f, "utf8");
    if (!t.startsWith('"djs-dump')) {
      if (dest) fs.copyFileSync(f, dest);
      continue;
    }
    n++;
    base[0] += Buffer.byteLength(t);
    base[1] += gz(t);
    RULES.forEach(([rule, fn], i) => {
      try {
        t = fn(t);
      } catch (e) {
        failures.push(`${f}: ${rule}: ${e.message}`);
      }
      tot[i][0] += Buffer.byteLength(t);
      tot[i][1] += gz(t);
    });
    if (dest) fs.writeFileSync(dest, t);
  }
  console.log(`\n### ${name}: ${n} dumps, ${base[0]} B raw, ${base[1]} B gzip\n`);
  console.log("| rule (cumulative) | raw | Δ raw | gzip | Δ gzip |\n|---|---:|---:|---:|---:|");
  let prev = base;
  RULES.forEach(([rule], i) => {
    const [r, g] = tot[i];
    console.log(`| ${rule} | ${r} | ${r - prev[0]} (${(((r - prev[0]) / base[0]) * 100).toFixed(1)}%) | ${g} | ${g - prev[1]} (${(((g - prev[1]) / base[1]) * 100).toFixed(1)}%) |`);
    prev = tot[i];
  });
  const [r, g] = tot[tot.length - 1];
  console.log(`| **total** | ${r} | ${r - base[0]} (${(((r - base[0]) / base[0]) * 100).toFixed(1)}%) | ${g} | ${g - base[1]} (${(((g - base[1]) / base[1]) * 100).toFixed(1)}%) |`);
  for (const x of failures.slice(0, 10)) console.log("FAIL", x);
  if (failures.length) console.log(`${failures.length} failures`);
}
