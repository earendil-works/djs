// The engine comparison of docs/guide/benchmarks.md over the codemode workload set
// (tests/bench/codemode). scripts/bench-engines.sh fetches and builds the engines and runs
// this; scripts/status.py engines records its --json output.
//
//   node js/bench/engines/run.ts [--reps N] [--engines a,b] [--workloads x,y] [--json FILE] [--quick]
//
// Per engine: module bytes (raw, gzip -9, brotli 11; the JS glue apart), compile time,
// the linear memory of an idle VM, startup (a new instance and VM, ready to evaluate).
// Per workload and engine: run time (set the tool answers, evaluate the program, drain the
// jobs, read the result) and the linear memory after the run, medians over N fresh VMs
// after a warm-up (a host process that has run the engine before: V8 has tiered its code
// up). Every engine's result must be the string node computes.

import { readFileSync, writeFileSync } from "node:fs";
import { cpus, loadavg, platform, release } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { constants as Z, brotliCompressSync, gzipSync } from "node:zlib";
import { record, stripBody } from "../../../tests/bench/codemode/tools.ts";
import { allJs, bodyProgram, type Engine, monty } from "./engines.ts";

const here = fileURLToPath(new URL(".", import.meta.url));
const root = join(here, "../../..");
const WDIR = join(root, "tests/bench/codemode");
const argv = process.argv.slice(2);
const opt = (name: string) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
};
const quick = argv.includes("--quick");
const REPS = quick ? 1 : Number(opt("--reps") ?? 25);
const WARM = quick ? 0 : 3;
/** The workloads in the page's order: tests/bench/codemode/README.md. */
export const WORKLOADS = ["sequential", "fanout", "media", "inspect", "loop", "text", "json", "ts", "heavy"];
const workloads = opt("--workloads")?.split(",") ?? WORKLOADS;
let engines: Engine[] = [...allJs(), monty()];
if (opt("--engines")) {
  const want = opt("--engines")!.split(",");
  engines = engines.filter((e) => want.includes(e.name) || want.includes(slug(e.name)));
}

/** The engine's key in status.tsv: `quickjs-emscripten (ng)` -> `quickjs-emscripten-ng`. */
export function slug(name: string): string {
  return name.replace(/[^a-z0-9]+/gi, "-").replace(/-$/, "").toLowerCase();
}
const median = (xs: number[]) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length % 2 ? s[s.length >> 1] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};
const now = () => performance.now();
const gz = (b: Uint8Array) => gzipSync(b, { level: 9 }).length;
const br = (b: Uint8Array) => brotliCompressSync(b, { params: { [Z.BROTLI_PARAM_QUALITY]: 11 } }).length;
const size = (files: string[]) => {
  const bs = files.map((f) => readFileSync(f));
  // --quick (the gate's smoke run, on debug modules of 30 MB): no compression.
  if (quick) return { bytes: bs.reduce((s, b) => s + b.length, 0), gzip: 0, brotli: 0 };
  return { bytes: bs.reduce((s, b) => s + b.length, 0), gzip: bs.reduce((s, b) => s + gz(b), 0), brotli: bs.reduce((s, b) => s + br(b), 0) };
};

// ---- The workloads: bodies, the Python translations, the recorded tool answers ------------

interface Workload {
  name: string;
  ts: boolean;
  /** The body as written (TypeScript for ts) and as JavaScript. */
  source: string;
  js: string;
  py: string;
  data: string;
  /** The result string, as node computes it: every engine must return the same. */
  expected: string;
}
const W: Workload[] = [];
for (const name of workloads) {
  const ts = name === "ts";
  const source = readFileSync(join(WDIR, ts ? "ts.ts" : `${name}.js`), "utf8");
  const js = ts ? await stripBody(source) : source;
  const py = readFileSync(join(WDIR, `${name}.py`), "utf8");
  const data = JSON.stringify(await record(js));
  const AsyncFunction = (async () => {}).constructor as new (...a: string[]) => (...v: unknown[]) => Promise<unknown>;
  const expected = String(await new AsyncFunction("__DATA", bodyProgram(js))(data));
  W.push({ name, ts, source, js, py, data, expected });
}

const startLoad = loadavg()[0];
const machine = `${cpus()[0].model}, ${cpus().length} cores, ${platform()} ${release()}, node ${process.version}`;
console.log(`${machine}; load ${loadavg().map((x) => x.toFixed(1)).join(" ")}; ${REPS} runs per cell after ${WARM} warm-up runs`);
if (startLoad > 4 && !quick) console.log("warning: load above 4: times are not comparable with recorded ones");

// ---- Per engine ----------------------------------------------------------------------------

interface EngineResult {
  name: string;
  version: string;
  language: string;
  wasm: { bytes: number; gzip: number; brotli: number };
  glue: { bytes: number; gzip: number; brotli: number };
  compileMs: number;
  idleMemory: number;
  startupMs: number;
  workloads: Record<string, { runMs: number; memory: number; ok: boolean } | undefined>;
}
const results: EngineResult[] = [];
const program = (e: Engine, w: Workload) => (e.language === "python" ? e.program(w.py) : e.program(e.typescript && w.ts ? w.source : w.js));

for (const e of engines) {
  let t = now();
  await e.load();
  const compileMs = now() - t;
  const idle = await e.create();
  const idleMemory = idle.memory();
  idle.dispose();
  const r: EngineResult = { name: e.name, version: e.version, language: e.language, wasm: size(e.files), glue: e.glue.length ? size(e.glue) : { bytes: 0, gzip: 0, brotli: 0 }, compileMs, idleMemory, startupMs: 0, workloads: {} };
  const starts: number[] = [];
  for (const w of W) {
    const ts = w.ts && e.typescript;
    const src = program(e, w);
    const runs: number[] = [];
    let memory = 0;
    let out = "";
    for (let i = 0; i < WARM + REPS; i++) {
      t = now();
      const vm = await e.create({ typescript: ts });
      const t1 = now();
      out = await vm.run(src, w.data);
      const t2 = now();
      memory = Math.max(memory, vm.memory());
      vm.dispose();
      if (i >= WARM) {
        starts.push(t1 - t);
        runs.push(t2 - t1);
      }
    }
    const ok = out === w.expected;
    if (!ok) console.log(`warning: ${e.name} ${w.name}: the result differs from node's (${out.slice(0, 200)})`);
    r.workloads[w.name] = { runMs: median(runs), memory, ok };
  }
  r.startupMs = median(starts);
  results.push(r);
  console.error(`${e.name}: done`);
}

// ---- Report --------------------------------------------------------------------------------

const kb = (n: number) => (n / 1024).toFixed(0);
const mib = (n: number) => (n / 1048576).toFixed(1);
const ms = (x: number) => (x < 1 ? x.toFixed(3) : x.toFixed(2));
let out = "\n| engine | wasm KiB | gzip | brotli | glue gzip | compile ms | idle MiB | startup ms |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n";
for (const r of results)
  out += `| ${r.name} | ${kb(r.wasm.bytes)} | ${kb(r.wasm.gzip)} | ${kb(r.wasm.brotli)} | ${kb(r.glue.gzip)} | ${ms(r.compileMs)} | ${mib(r.idleMemory)} | ${ms(r.startupMs)} |\n`;
out += `\nrun ms (median of ${REPS})\n\n| workload | ${results.map((r) => r.name).join(" | ")} |\n| --- |${results.map(() => " ---: |").join("")}\n`;
for (const w of W) out += `| ${w.name} | ${results.map((r) => (r.workloads[w.name] ? ms(r.workloads[w.name]!.runMs) + (r.workloads[w.name]!.ok ? "" : " (differs)") : "")).join(" | ")} |\n`;
out += `\nlinear memory after the run, MiB\n\n| workload | ${results.map((r) => r.name).join(" | ")} |\n| --- |${results.map(() => " ---: |").join("")}\n`;
for (const w of W) out += `| ${w.name} | ${results.map((r) => (r.workloads[w.name] ? mib(r.workloads[w.name]!.memory) : "")).join(" | ")} |\n`;
console.log(out);
console.log(`load at the end ${loadavg().map((x) => x.toFixed(1)).join(" ")}`);
const json = opt("--json");
if (json) writeFileSync(json, JSON.stringify({ machine, load: [startLoad, loadavg()[0]], reps: REPS, engines: results, workloads: W.map((w) => ({ name: w.name, bytes: w.source.length, data: w.data.length })) }, null, 1));
if (results.some((r) => Object.values(r.workloads).some((x) => x && !x.ok))) process.exitCode = 1;
