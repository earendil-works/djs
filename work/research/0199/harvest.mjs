// harvest.mjs: today's format 2 dumps at every idle point (at most MAX per program) of
// the tests/dump/v2 programs (scripts) and the harvested codemode corpus (bodies, with a
// one-line shim for pi's text/store/load). Writes progs/, today/<name>.<k>.dump.js,
// today/<name>.<k>.pre (output before the dump) and today/<name>.out (the whole run).
//   node harvest.mjs [filter]
import fs from "node:fs";
import path from "node:path";
import { root, djs, pool, here } from "./lib.mjs";

const MAX = 40;
const filter = process.argv[2] ?? "";
fs.mkdirSync(path.join(here, "progs"), { recursive: true });
fs.mkdirSync(path.join(here, "today"), { recursive: true });

// One line, so the case keeps its line numbers but one.
const SHIM =
  'const text = (v) => print(v === undefined || v === null || (typeof v !== "object" && typeof v !== "function") ? String(v) : (JSON.stringify(v) ?? String(v))); const $stored = new Map(); function store(k, v) { if (v === undefined) $stored.delete(k); else $stored.set(k, JSON.stringify(v)); } function load(k) { const j = $stored.get(k); return j === undefined ? undefined : JSON.parse(j); }\n';

const progs = [];
for (const dir of ["tests/dump/v2", "tests/dump/v2/golden"]) {
  for (const f of fs.readdirSync(path.join(root, dir)).sort()) {
    if (!f.endsWith(".src.js")) continue;
    const name = (dir.endsWith("golden") ? "g-" : "v2-") + f.replace(/\.src\.js$/, "");
    fs.copyFileSync(path.join(root, dir, f), path.join(here, "progs", name + ".js"));
    progs.push({ name, body: false });
  }
}
const hdir = path.join(root, "tests/corpus/codemode-harvest");
for (const f of fs.readdirSync(hdir).sort()) {
  if (!f.endsWith(".js")) continue;
  const name = f.replace(/\.js$/, "");
  fs.writeFileSync(path.join(here, "progs", name + ".js"), SHIM + fs.readFileSync(path.join(hdir, f), "utf8"));
  progs.push({ name, body: true });
}

const selected = progs.filter((p) => p.name.includes(filter));
fs.writeFileSync(path.join(here, "progs.json"), JSON.stringify(progs, null, 1));
let total = 0;
await pool(
  selected.map((p) => async () => {
    const flags = p.body ? ["--body"] : [];
    const src = path.join("progs", p.name + ".js");
    const full = await djs(["run", ...flags, src]);
    fs.writeFileSync(path.join(here, "today", p.name + ".out"), full.out);
    for (let k = 1; k <= MAX; k++) {
      const out = path.join("today", `${p.name}.${k}.dump.js`);
      const r = await djs(["run", ...flags, "--dump-after", String(k), out, src]);
      if (!fs.existsSync(path.join(here, out))) {
        if (!r.out.includes("the run ended")) fs.writeFileSync(path.join(here, "today", `${p.name}.${k}.err`), r.out);
        break;
      }
      fs.writeFileSync(path.join(here, "today", `${p.name}.${k}.pre`), r.out);
      total++;
    }
  }),
  4,
);
console.log(`${selected.length} programs, ${total} dumps`);
