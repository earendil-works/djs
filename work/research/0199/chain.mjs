// chain.mjs [filter] [--node]: re-dump chains. For every program: today's dump at idle
// point 1 -> plain dump P1; then run Pk as a plain Script in djs to idle point 2, dump it
// (today's exporter, on a VM running continuations), map it back to the original
// functions (normalize) and write P(k+1) from the original text again; at most MAX links.
// The output before the first dump plus every link's output must equal the uninterrupted
// run's; sizes per link show whether re-dumps stay bounded. With --node every link's plain
// dump is also run to the end in node and compared.
import fs from "node:fs";
import path from "node:path";
import { djs, run, pool, clean, here } from "./lib.mjs";
import { toPlain } from "./plain.mjs";
import { normalize } from "./normalize.mjs";

const MAX = 50;
const args = process.argv.slice(2);
const useNode = args.includes("--node");
const filter = args.find((a) => !a.startsWith("--")) ?? "";
const progs = JSON.parse(fs.readFileSync(path.join(here, "progs.json"), "utf8")).filter(
  (p) => p.name.includes(filter) && fs.existsSync(path.join(here, "today", `${p.name}.1.dump.js`)),
);
fs.mkdirSync(path.join(here, "chain"), { recursive: true });
const norm = (s) => s.replace(/"elapsedMs":\d+/g, '"elapsedMs":N').replace(/^(.*?\b(Type|Reference|Range|Syntax)Error): .*$/gm, "$1: <message>");
const rows = [];
await pool(
  progs.map((p) => async () => {
    const w = path.join("chain", p.name);
    fs.rmSync(path.join(here, w), { recursive: true, force: true });
    fs.mkdirSync(path.join(here, w));
    const full = clean(fs.readFileSync(path.join(here, "today", p.name + ".out"), "utf8"));
    let out = fs.readFileSync(path.join(here, "today", `${p.name}.1.pre`), "utf8");
    let d = fs.readFileSync(path.join(here, "today", `${p.name}.1.dump.js`), "utf8");
    const sizes = [];
    const dsizes = [];
    let status = "ok";
    let nodeBad = 0;
    let k = 1;
    for (; k <= MAX; k++) {
      let plain;
      try {
        plain = toPlain(k === 1 ? d : normalize(d).text).text;
      } catch (e) {
        status = `refused at link ${k}: ${e.message}`;
        break;
      }
      const pk = path.join(w, `p${k}.js`);
      fs.writeFileSync(path.join(here, pk), plain);
      sizes.push(plain.length);
      dsizes.push(d.length);
      if (useNode) {
        const n = await run("node", [path.join(here, "harness.mjs"), pk]);
        if (norm(clean(out + n.out)) !== norm(full)) nodeBad++;
      }
      const next = path.join(w, `d${k + 1}.js`);
      const r = await djs(["run", "--dump-after", "2", next, pk]);
      out += r.out;
      if (!fs.existsSync(path.join(here, next))) {
        if (/cannot dump/.test(r.out)) status = `re-dump refused at link ${k}: ${r.out.split("\n").find((l) => /cannot dump|:/.test(l))}`;
        break;
      }
      d = fs.readFileSync(path.join(here, next), "utf8");
    }
    const got = clean(out);
    const same = got === full ? "same" : norm(got) === norm(full) ? "same-env" : "DIFF";
    rows.push({ name: p.name, links: Math.min(k, MAX), status, same, sizes, dsizes, nodeBad });
  }),
  4,
);
rows.sort((a, b) => a.name.localeCompare(b.name));
fs.writeFileSync(path.join(here, "chain.json"), JSON.stringify(rows, null, 1));
const sum = { programs: rows.length, same: 0, sameEnv: 0, diff: 0, refused: 0, links: 0, nodeBad: 0 };
let worst = 0;
for (const r of rows) {
  if (r.status !== "ok") sum.refused++;
  sum[r.same === "same" ? "same" : r.same === "same-env" ? "sameEnv" : "diff"]++;
  sum.links += r.links;
  sum.nodeBad += r.nodeBad;
  if (r.sizes.length > 1) worst = Math.max(worst, Math.max(...r.sizes) / r.sizes[0]);
}
console.log(JSON.stringify(sum), "largest link / first link:", worst.toFixed(2));
for (const r of rows) if (r.status !== "ok" || r.same === "DIFF") console.log(r.name, r.status, r.same);
