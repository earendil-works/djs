// Savings of the owner/host-decision items (0216) on top of given dumps (the minimal cases,
// or a suite rewritten by suite.mjs): each item alone, raw and gzip -9, summed over files.
//   node work/research/0216/owner.mjs FILE|DIR ...
import fs from "node:fs";
import path from "node:path";
import { gzipSync } from "node:zlib";
const gz = (t) => gzipSync(t, { level: 9 }).length;
const walk = (d) => (fs.statSync(d).isDirectory() ? fs.readdirSync(d).flatMap((e) => walk(path.join(d, e))) : [d]);
const items = {
  "seed left out (Math.random never used: state unobserved)": (t) => t.replace(/^\$djs\.seed\([^)]*\);\n/m, ""),
  '"use strict" left out (hosts compile dumps strict)': (t) => t.replace(/^"use strict";\n/m, ""),
  "source by hash alone, 8 hex digits": (t) => t.replace(/\$djs\.source\("[^"]*", ?"([0-9a-f]{8})[0-9a-f]{8}"(, ?"[^"]*")?(, ?1)?\)/g, '$djs.source("$1")'),
  "pi: call state left out where it is the call id": (t) => t.replace(/\$djs\.resume\((\d+),([^;]*?),\s?\{"id":(\d+)\}\)/g, (a, id, mid, s) => (id === s ? `$djs.resume(${id},${mid})` : a)),
  "no newlines between generated statements": (t) => t.replace(/;\n(?=\$djs\.|\$d\.|\$dump\.|\{const |\$g\.)/g, ";"),
};
const files = process.argv.slice(2).flatMap(walk).filter((f) => f.endsWith(".js"));
let base = [0, 0];
const tot = Object.fromEntries(Object.keys(items).map((k) => [k, [0, 0]]));
for (const f of files) {
  const t = fs.readFileSync(f, "utf8");
  if (!t.startsWith('"djs-dump')) continue;
  const b = [Buffer.byteLength(t), gz(t)];
  base = [base[0] + b[0], base[1] + b[1]];
  for (const [k, fn] of Object.entries(items)) {
    const u = fn(t);
    tot[k][0] += Buffer.byteLength(u) - b[0];
    tot[k][1] += gz(u) - b[1];
  }
}
console.log(`${files.length} files, ${base[0]} B raw, ${base[1]} B gzip\n\n| item | Δ raw | Δ gzip |\n|---|---:|---:|`);
for (const [k, [r, g]] of Object.entries(tot)) console.log(`| ${k} | ${r} (${((r / base[0]) * 100).toFixed(1)}%) | ${g} (${((g / base[1]) * 100).toFixed(1)}%) |`);
