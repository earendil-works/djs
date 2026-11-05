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
// up). Every engine's result must be the string node computes. For djs also: the
// continuation dumps at every idle point of each workload, run through the Sandbox with
// async host tools (the shipped module plus the dump module): dump time and size,
// load time (a new sandbox with the run loaded).

import { readFileSync, writeFileSync } from "node:fs";
import { cpus, loadavg, platform, release } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { constants as Z, brotliCompressSync, gzipSync } from "node:zlib";
import { Sandbox, type SandboxOptions } from "../../src/sandbox.ts";
import { answer, catalog, record, stripBody } from "../../../tests/bench/codemode/tools.ts";
import { allJs, bodyProgram, djs, type Engine, monty } from "./engines.ts";

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

// ---- djs dumps -----------------------------------------------------------------------------

interface DumpResult {
  points: number;
  rawP50: number;
  rawMax: number;
  gzipP50: number;
  gzipMax: number;
  dumpMsP50: number;
  resumeMsP50: number;
}
const dumps: Record<string, DumpResult> = {};
if (engines.some((e) => e.name === "djs")) {
  const mainPath = djs().files[0];
  const main = await WebAssembly.compile(readFileSync(mainPath));
  const dumper = await WebAssembly.compile(readFileSync(join(root, "target/djs-dump.wasm")));
  const tools = ["read", "bash", "edit", "write", "web_search", "query_rows", "render_view", "read_image"];
  const time = (f: () => void) => {
    const t0 = now();
    f();
    return now() - t0;
  };
  for (const w of W) {
    const opts: SandboxOptions = {
      module: main,
      dumpModule: dumper,
      typescript: w.ts,
      deterministic: { startedAt: 1_762_000_000_000, seed: 42 },
      // Dumps need frozen builtins.
      frozenBuiltins: true,
      console: { log() {}, info() {}, warn() {}, error() {} },
      functions: { text: () => undefined, image: () => undefined },
      asyncFunctions: Object.fromEntries(tools.map((t) => [`tools.${t}`, () => 0])),
    };
    const sb = new Sandbox(opts);
    const run = sb.begin(w.source, { ALL_TOOLS: catalog() });
    const raw: number[] = [];
    const gzs: number[] = [];
    const dumpMs: number[] = [];
    const resumeMs: number[] = [];
    while (run.state === "idle") {
      let text = "";
      let best = Infinity;
      for (let i = 0; i < (quick ? 1 : 5); i++)
        best = Math.min(best, time(() => {
          text = run.dump();
        }));
      const bytes = new TextEncoder().encode(text);
      raw.push(bytes.length);
      gzs.push(gz(bytes));
      dumpMs.push(best);
      let bestResume = Infinity;
      for (let i = 0; i < (quick ? 1 : 5); i++) {
        // A new sandbox with the dumped run loaded, idle on its open calls: what a host
        // does before it settles them.
        const t0 = now();
        const fresh = await Sandbox.create(opts);
        fresh.loadRun(text);
        bestResume = Math.min(bestResume, now() - t0);
        fresh.dispose();
      }
      resumeMs.push(bestResume);
      // Settle one call at a time, oldest first: every open call is an idle point of its own.
      const c = run.calls[0];
      const name = c.fn.replace(/^tools\./, "");
      try {
        run.settle(c.id, { ok: true, value: answer(name, (c.args[0] ?? {}) as Record<string, unknown>) });
      } catch (err) {
        run.settle(c.id, { ok: false, value: String((err as Error).message) });
      }
      await run.advance();
    }
    if (run.state !== "done") throw new Error(`${w.name}: the dump run ended ${run.state}: ${String(run.error)}`);
    sb.dispose();
    if (raw.length)
      dumps[w.name] = { points: raw.length, rawP50: median(raw), rawMax: Math.max(...raw), gzipP50: median(gzs), gzipMax: Math.max(...gzs), dumpMsP50: median(dumpMs), resumeMsP50: median(resumeMs) };
  }
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
if (Object.keys(dumps).length) {
  out += "\ndjs dumps at every idle point\n\n| workload | points | raw KiB p50 / max | gzip KiB p50 / max | dump ms p50 | resume ms p50 |\n| --- | ---: | ---: | ---: | ---: | ---: |\n";
  for (const [k, d] of Object.entries(dumps))
    out += `| ${k} | ${d.points} | ${(d.rawP50 / 1024).toFixed(1)} / ${(d.rawMax / 1024).toFixed(1)} | ${(d.gzipP50 / 1024).toFixed(1)} / ${(d.gzipMax / 1024).toFixed(1)} | ${ms(d.dumpMsP50)} | ${ms(d.resumeMsP50)} |\n`;
}
console.log(out);
console.log(`load at the end ${loadavg().map((x) => x.toFixed(1)).join(" ")}`);
const json = opt("--json");
if (json) writeFileSync(json, JSON.stringify({ machine, load: [startLoad, loadavg()[0]], reps: REPS, engines: results, dumps, workloads: W.map((w) => ({ name: w.name, bytes: w.source.length, data: w.data.length })) }, null, 1));
if (results.some((r) => Object.values(r.workloads).some((x) => x && !x.ok))) process.exitCode = 1;
