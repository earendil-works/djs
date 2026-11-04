// check.mjs [filter] [--node]: run every plain dump (plain/) as an ordinary Script in djs
// (`djs run`, no loader) and, with --node, in node (harness.mjs); the output before the
// dump plus the continued output must equal the uninterrupted run's.
import fs from "node:fs";
import path from "node:path";
import { djs, run, pool, clean, here } from "./lib.mjs";

const args = process.argv.slice(2);
const useNode = args.includes("--node");
const filter = args.find((a) => !a.startsWith("--")) ?? "";
const dir = process.env.PLAIN ?? "plain";
const files = fs.readdirSync(path.join(here, dir)).filter((f) => f.endsWith(".dump.js") && f.includes(filter)).sort();
const res = { ok: 0, okEnv: 0, diff: 0, nodeOk: 0, nodeOkEnv: 0, nodeDiff: 0 };
// What differs between runs for reasons outside the dump: the wall clock (Date.now() before
// and after), and engine-specific error messages (djs vs V8).
const norm = (s) =>
  s
    .replace(/"elapsedMs":\d+/g, '"elapsedMs":N')
    .replace(/^(.*?\b(Type|Reference|Range|Syntax)Error): .*$/gm, "$1: <message>")
    .replace(/: Error: $/gm, ": Error");
const diffs = [];
await pool(
  files.map((f) => async () => {
    const m = /^(.*)\.(\d+)\.dump\.js$/.exec(f);
    const full = clean(fs.readFileSync(path.join(here, "today", m[1] + ".out"), "utf8"));
    const pre = fs.readFileSync(path.join(here, "today", `${m[1]}.${m[2]}.pre`), "utf8");
    const p = path.join(dir, f);
    const r = await djs(["run", p]);
    const got = clean(pre + r.out);
    if (got === full) res.ok++;
    else if (norm(got) === norm(full)) res.okEnv++;
    else {
      res.diff++;
      diffs.push(`DIFF djs ${f}\n${firstDiff(full, got)}`);
    }
    if (useNode) {
      const n = await run("node", [path.join(here, "harness.mjs"), p]);
      const gotN = clean(pre + n.out);
      if (gotN === full) res.nodeOk++;
      else if (norm(gotN) === norm(full)) res.nodeOkEnv++;
      else {
        res.nodeDiff++;
        diffs.push(`DIFF node ${f}\n${firstDiff(full, gotN)}`);
      }
    }
  }),
  4,
);
function firstDiff(a, b) {
  const x = a.split("\n");
  const y = b.split("\n");
  for (let i = 0; i < Math.max(x.length, y.length); i++)
    if (x[i] !== y[i]) return `  line ${i + 1}\n  want: ${(x[i] ?? "<end>").slice(0, 200)}\n  got:  ${(y[i] ?? "<end>").slice(0, 200)}`;
  return "";
}
fs.writeFileSync(path.join(here, "check.log"), diffs.join("\n") + "\n");
console.log(JSON.stringify(res), `(${files.length} dumps; diffs in check.log)`);
