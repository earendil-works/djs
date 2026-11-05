// Per-operation microbenchmarks: djs against the QuickJS builds of the engine comparison
// (js/bench/engines/engines.ts), on data shaped like the codemode workloads' (file texts,
// their lines, search-result prose, query rows: tests/bench/codemode/tools.ts). Each op is a
// loop in a codemode body; its time is the run's median over fresh VMs minus that of the
// setup alone (`base`), so it is the op's own cost. The last column is djs over the best
// QuickJS build; above 1 djs is slower. The engines take turns run by run.
//
//   scripts/bench-engines.sh --setup     (once: builds the QuickJS releases into vendor/engines)
//   node js/bench/engines/ops.ts [--reps N] [--engines djs,quickjs,quickjs-ng,quickjs-wasi] [FILTER...]
//
// FILTER: ops whose name contains one of them. DJS_WASM picks the djs build (default: the
// shipped module, scripts/build-wasm.sh). Times are wall clock: compare runs made at a
// similar load (printed), and prefer ratios to absolute numbers.

import { readFileSync } from "node:fs";
import { loadavg } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { record } from "../../../tests/bench/codemode/tools.ts";
import { djs, type Engine, quickjsBuilt, quickjsWasi } from "./engines.ts";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "../../..");
const argv = process.argv.slice(2);
const opt = (name: string) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv.splice(i, 2)[1] : undefined;
};
const REPS = Number(opt("--reps") ?? 9);
const want = (opt("--engines") ?? "djs,quickjs,quickjs-ng").split(",");
const filters = argv.filter((a) => !a.startsWith("--"));

// The data: the tool answers of three workloads, as the global `__DATA`.
const bodies = ["heavy.js", "loop.js", "json.js"].map((f) => readFileSync(join(root, "tests/bench/codemode", f), "utf8"));
const data = JSON.stringify(Object.assign({}, ...(await Promise.all(bodies.map((b) => record(b))))));

/**
 * The setup every op starts from: TX the file texts, L their lines, P search-result
 * prose, ROWS 600 query rows, R 600 small objects.
 */
const SETUP = `
const TX = []; for (const k in DATA) if (k.startsWith("read")) TX.push(JSON.parse(DATA[k]));
const L = TX.join("\\n").split("\\n");
const P = []; for (const k in DATA) if (k.startsWith("web_search")) for (const r of JSON.parse(DATA[k]).structuredContent.results) P.push(r.text);
const ROWS = JSON.parse(DATA[Object.keys(DATA).find((k) => k.startsWith("query_rows"))]).structuredContent.rows;
const R = Array.from({ length: 600 }, (_, i) => ({ d: ((i * 7919) % 5000) / 10, s: "svc" + (i % 8), u: "u" + (i % 400) }));
`;

/** name -> loop body (after SETUP); each returns a number so nothing is optimized away. */
const OPS: Record<string, string> = {
  // JSON
  "JSON.parse texts x3": "let n = 0; for (let i = 0; i < 3; i++) for (const k in DATA) if (k.startsWith('read')) n += JSON.parse(DATA[k]).length; return n;",
  "JSON.parse rows x5": "let n = 0; const k = Object.keys(DATA).find((k) => k.startsWith('query_rows')); for (let i = 0; i < 5; i++) n += JSON.parse(DATA[k]).structuredContent.rows.length; return n;",
  "JSON.stringify rows x5": "let n = 0; for (let i = 0; i < 5; i++) n += JSON.stringify(ROWS).length; return n;",
  "JSON.stringify rows indent x5": "let n = 0; for (let i = 0; i < 5; i++) n += JSON.stringify(ROWS, null, 2).length; return n;",
  // strings
  "split lines x3": "let n = 0; for (let i = 0; i < 3; i++) for (const t of TX) n += t.split('\\n').length; return n;",
  "split words": "let n = 0; for (let i = 0; i < 5; i++) for (const p of P) n += p.split(' ').length; return n;",
  "join lines x3": "let n = 0; for (let i = 0; i < 3; i++) n += L.join('\\n').length; return n;",
  "join 100k x5": "const a = Array.from({ length: 100000 }, (_, i) => 'line ' + i); let n = 0; for (let i = 0; i < 5; i++) n += a.join('\\n').length; return n;",
  "includes x2 per line": "let n = 0; for (const l of L) if (l.includes('TODO') || l.includes('FIXME')) n++; return n;",
  "indexOf scan": "let n = 0; for (const t of TX) { let i = -1; while ((i = t.indexOf('return', i + 1)) >= 0) n++; } return n;",
  "replaceAll string": "let n = 0; for (const t of TX) n += t.replaceAll('Error(', 'SessionError(').length; return n;",
  "trim+slice per line": "let n = 0; for (const l of L) n += l.trim().slice(0, 80).length; return n;",
  "toLowerCase per line": "let n = 0; for (const l of L) n += l.toLowerCase().length; return n;",
  "charCodeAt loop": "let n = 0; for (const t of TX) for (let i = 0; i < t.length; i += 4) n += t.charCodeAt(i); return n;",
  "template concat": "let s = ''; for (let i = 0; i < L.length; i++) s += `${i}: ${L[i]}\\n`; return s.length;",
  "a + b 100k": "let n = 0; const a = 'abc', b = 'defgh'; for (let i = 0; i < 100000; i++) n += (a + b).length; return n;",
  "padStart 50k": "let n = 0; for (let i = 0; i < 50000; i++) n += String(i).padStart(6).length; return n;",
  "padEnd 50k": "let n = 0; for (let i = 0; i < 50000; i++) n += 'abc'.padEnd(28).length; return n;",
  "trim 100k": "let n = 0; const s = '  hello world  '; for (let i = 0; i < 100000; i++) n += s.trim().length; return n;",
  "slice 100k": "let n = 0; const s = 'hello world, this is a line of code'; for (let i = 0; i < 100000; i++) n += s.slice(0, 20).length; return n;",
  "substring 100k": "let n = 0; const s = 'hello world'; for (let i = 0; i < 100000; i++) n += s.substring(1, 4).length; return n;",
  "charAt 100k": "let n = 0; const s = 'hello'; for (let i = 0; i < 100000; i++) n += s.charAt(0).length; return n;",
  "at 100k": "let n = 0; const s = 'hello'; for (let i = 0; i < 100000; i++) n += s.at(0).length; return n;",
  "repeat 100k": "let n = 0; const s = 'ab'; for (let i = 0; i < 100000; i++) n += s.repeat(3).length; return n;",
  "toLowerCase 100k": "let n = 0; const s = 'Hello World This Is A Line'; for (let i = 0; i < 100000; i++) n += s.toLowerCase().length; return n;",
  // numbers
  "String(i) 100k": "let n = 0; for (let i = 0; i < 100000; i++) n += String(i).length; return n;",
  "'' + i 100k": "let n = 0; for (let i = 0; i < 100000; i++) n += ('' + i).length; return n;",
  "toFixed 50k": "let n = 0; for (let i = 0; i < 50000; i++) n += (i / 7).toFixed(1).length; return n;",
  "Math.max per line": "let m = 0; for (const l of L) m = Math.max(m, l.length); return m;",
  "Math.floor 100k": "let n = 0; for (let i = 0; i < 100000; i++) n += Math.floor(i / 3); return n;",
  "Number.isInteger 100k": "let n = 0; for (let i = 0; i < 100000; i++) n += Number.isInteger(i) ? 1 : 0; return n;",
  // regular expressions
  "regex match /function \\w+/g": "let n = 0; for (const t of TX) n += (t.match(/function \\w+/g) || []).length; return n;",
  "regex replace /\\s+/g x20": "let n = 0; for (let i = 0; i < 20; i++) for (const p of P) n += p.replace(/\\s+/g, ' ').length; return n;",
  "regex exec per line": "let n = 0; for (const l of L) { const m = /^([^:]+):(\\d+):(.*)$/.exec(l); if (m) n++; } return n;",
  "regex test per line": "let n = 0; for (const l of L) if (/\\b(TODO|FIXME)\\b/.test(l)) n++; return n;",
  "matchAll /gm": "let n = 0; for (const t of TX) n += [...t.matchAll(/^export (?:async )?function (\\w+)\\((\\w+)/gm)].length; return n;",
  "new RegExp 5k": "let n = 0; for (let i = 0; i < 5000; i++) n += new RegExp('report|metric', 'i').source.length; return n;",
  // arrays and collections
  "map x200": "let n = 0; for (let i = 0; i < 200; i++) n += R.map((r) => r.d).length; return n;",
  "filter x200": "const D = R.map((r) => r.d); let n = 0; for (let i = 0; i < 200; i++) n += D.filter((d) => d > 10).length; return n;",
  "reduce x200": "let n = 0; for (let i = 0; i < 200; i++) n += R.reduce((s, r) => s + r.d, 0); return n;",
  "sort comparator x50": "const D = R.map((r) => r.d); let n = 0; for (let i = 0; i < 50; i++) n += [...D].sort((a, b) => a - b).length; return n;",
  "sort default x50": "const D = R.map((r) => r.s + r.u); let n = 0; for (let i = 0; i < 50; i++) n += [...D].sort().length; return n;",
  "sort lines x5": "let n = 0; for (let i = 0; i < 5; i++) n += [...L].sort().length; return n;",
  "push 300k": "const a = []; for (let i = 0; i < 300000; i++) a.push(i); return a.length;",
  "Array.from length 20 x10k": "let n = 0; for (let i = 0; i < 10000; i++) n += Array.from({ length: 20 }, (_, j) => j).length; return n;",
  "Map string keys x200": "let n = 0; for (let i = 0; i < 200; i++) { const m = new Map(); for (const r of R) m.set(r.s + r.u, 1); n += m.size; } return n;",
  "Set string keys x200": "let n = 0; for (let i = 0; i < 200; i++) { const s = new Set(); for (const r of R) s.add(r.u); n += s.size; } return n;",
  "Map group rows x20": "let n = 0; for (let i = 0; i < 20; i++) { const m = new Map(); for (const r of ROWS) { const k = r.service + ' ' + r.route; const g = m.get(k) ?? { n: 0 }; g.n++; m.set(k, g); } n += m.size; } return n;",
  "object spread x20": "let n = 0; for (let i = 0; i < 20; i++) for (const r of ROWS) { const o = { ...r, x: 1 }; n += o.status; } return n;",
  "Object.entries/fromEntries x200": "let n = 0; for (let i = 0; i < 200; i++) n += Object.keys(Object.fromEntries(Object.entries(ROWS[i]))).length; return n;",
  // calls and async
  "closure calls 300k": "const f = (x) => x + 1; let n = 0; for (let i = 0; i < 300000; i++) n = f(n); return n;",
  "optional arg calls 100k": "function f(a, b) { return b === undefined ? a : b; } let n = 0; for (let i = 0; i < 100000; i++) n += f(i); return n;",
  "await value 50k": "let n = 0; for (let i = 0; i < 50000; i++) n += await i; return n;",
  "async call+await 20k": "let n = 0; const f = async (x) => x; for (let i = 0; i < 20000; i++) n += await f(1); return n;",
  "Promise.all 20 x500": "let n = 0; for (let i = 0; i < 500; i++) n += (await Promise.all(Array.from({ length: 20 }, (_, j) => Promise.resolve(j)))).length; return n;",
};

const all: Engine[] = [djs(), quickjsBuilt("bellard"), quickjsBuilt("ng"), quickjsWasi()];
const engines = all.filter((e) => want.includes(e.name));
if (engines[0]?.name !== "djs" || engines.length < 2) {
  console.error("ops.ts: --engines needs djs first and at least one QuickJS build (quickjs, quickjs-ng, quickjs-wasi)");
  process.exit(2);
}
const ops = Object.entries(OPS).filter(([k]) => filters.length === 0 || filters.some((f) => k.includes(f)));

const median = (xs: number[]) => [...xs].sort((a, b) => a - b)[xs.length >> 1];
console.log(`load ${loadavg().map((x) => x.toFixed(1)).join(" ")}; median of ${REPS} fresh VMs; ms, the setup's time subtracted`);
for (const e of engines) await e.load();
/** Median ms of `body` per engine; the engines take turns run by run, so a change in load hits all alike. */
async function time(body: string): Promise<number[]> {
  const srcs = engines.map((e) => e.program(SETUP + body));
  const ts: number[][] = engines.map(() => []);
  for (let i = 0; i < REPS + 1; i++) {
    for (const [j, e] of engines.entries()) {
      const vm = await e.create();
      const t = performance.now();
      const r = await vm.run(srcs[j], data);
      const dt = performance.now() - t;
      vm.dispose();
      if (String(r).startsWith("ERROR")) throw new Error(`${e.name}: ${r}`);
      if (i > 0) ts[j].push(dt);
    }
  }
  return ts.map(median);
}
const base = await time("return 0;");
const times = new Map<string, number[]>();
for (const [name, body] of ops) times.set(name, (await time(body)).map((x, j) => Math.max(0, x - base[j])));
const w = Math.max(...ops.map(([k]) => k.length));
console.log(`${"op".padEnd(w)} ${engines.map((e) => e.name.padStart(12)).join(" ")}   djs/best`);
for (const [name] of ops) {
  const row = times.get(name)!;
  const best = Math.min(...row.slice(1));
  // Below 0.05 ms a difference is noise: no ratio.
  const ratio = best < 0.05 || row[0] < 0.05 ? "-" : (row[0] / best).toFixed(2) + (row[0] / best > 1.5 ? " !" : "");
  console.log(`${name.padEnd(w)} ${row.map((x) => x.toFixed(2).padStart(12)).join(" ")}   ${ratio}`);
}
