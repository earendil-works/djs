// The per-byte accounting of one hand-minimized dump (0216). A case is a directory
// cases/NAME with main.js (main's dump, copied unchanged) and steps.mjs, which exports
// `steps`: [category, edit] pairs applied in order, an edit being a rule of rules.mjs (a
// function) or a list of [from, to] text replacements, each `from` occurring exactly once.
// The result is written to minimal.js; the report gives raw and gzip -9 bytes per step.
//   node work/research/0216/account.mjs [NAME ...]     (default: every case)
//   --md   one markdown table per case and a summary (what README.md includes)
import fs from "node:fs";
import path from "node:path";
import { gzipSync } from "node:zlib";

const here = path.dirname(new URL(import.meta.url).pathname);
const gz = (t) => gzipSync(t, { level: 9 }).length;
const raw = (t) => Buffer.byteLength(t);
const argv = process.argv.slice(2).filter((a) => !a.startsWith("--"));
const names = argv.length ? argv : fs.readdirSync(path.join(here, "cases")).sort();
const summary = [];
const bycat = new Map();
for (const name of names) {
  const dir = path.join(here, "cases", name);
  let t = fs.readFileSync(path.join(dir, "main.js"), "utf8");
  const { steps, note } = await import(path.join(dir, "steps.mjs"));
  const base = [raw(t), gz(t)];
  const rows = [];
  for (const [cat, edit] of steps) {
    const before = [raw(t), gz(t)];
    if (typeof edit === "function") t = edit(t);
    else
      for (const [from, to] of edit) {
        const n = t.split(from).length - 1;
        if (n !== 1) throw new Error(t.slice(-300) + "\n" + `${name}: ${cat}: ${JSON.stringify(from.slice(0, 60))} occurs ${n} times`);
        t = t.replace(from, () => to);
      }
    const d = [raw(t) - before[0], gz(t) - before[1]];
    rows.push([cat, ...d]);
    const c = bycat.get(cat) ?? [0, 0];
    bycat.set(cat, [c[0] + d[0], c[1] + d[1]]);
  }
  fs.writeFileSync(path.join(dir, "minimal.js"), t);
  const fin = [raw(t), gz(t)];
  summary.push([name, ...base, ...fin]);
  console.log(`\n#### ${name}${note ? ": " + note : ""}\n`);
  console.log(`main ${base[0]} B raw, ${base[1]} B gzip → minimal ${fin[0]} B raw (${pct(fin[0], base[0])}), ${fin[1]} B gzip (${pct(fin[1], base[1])})\n`);
  console.log("| step (in order) | Δ raw | Δ gzip |\n|---|---:|---:|");
  for (const [c, r, g] of rows) console.log(`| ${c} | ${r} | ${g} |`);
}
function pct(a, b) {
  return `${(((a - b) / b) * 100).toFixed(0)}%`;
}
console.log("\n#### Summary\n\n| case | main raw | main gzip | minimal raw | minimal gzip | raw | gzip |\n|---|---:|---:|---:|---:|---:|---:|");
let T = [0, 0, 0, 0];
for (const [n, a, b, c, d] of summary) {
  console.log(`| ${n} | ${a} | ${b} | ${c} | ${d} | ${pct(c, a)} | ${pct(d, b)} |`);
  T = [T[0] + a, T[1] + b, T[2] + c, T[3] + d];
}
console.log(`| **total** | ${T[0]} | ${T[1]} | ${T[2]} | ${T[3]} | ${pct(T[2], T[0])} | ${pct(T[3], T[1])} |`);
console.log("\n#### By category, over all cases\n\n| category | Δ raw | Δ gzip |\n|---|---:|---:|");
for (const [c, [r, g]] of [...bycat].sort((a, b) => a[1][0] - b[1][0])) console.log(`| ${c} | ${r} | ${g} |`);
