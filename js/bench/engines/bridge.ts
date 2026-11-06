// The bridge benchmark of docs/guide/benchmarks.md: moving data between a host and the
// sandbox, in each engine's best way (`Engine.bridge` in engines.ts says how).
//
//   node js/bench/engines/bridge.ts [--reps N] [--engines a,b] [--payloads x,y] [--json FILE] [--quick]
//
// Per payload and engine, in fresh VMs:
// - in: a host value becomes a VM value (`BridgeVm.put`), as a tool result arrives;
// - out: a VM value becomes a host value (`BridgeVm.get`), as a host reads a run's result
//   or a tool call's arguments;
// each the median time of one operation (small payloads repeated within a sample), and
// the linear memory after it in a fresh VM. End to end, whole runs with a sync host function
// `tool`: a 1 MB result of which the script reads one field, a 1 MB file read, edited and
// written back, and a thousand small calls.
// Every engine must give back what it was given, and the end-to-end results must agree.
// An engine whose API offers several paths (`Engine.bridgePaths`) is measured on each, and
// each number is its best path's: a host that knows what it passes would pick it.

import { writeFileSync } from "node:fs";
import { cpus, loadavg, platform, release } from "node:os";
import { allJs, type BridgeVm, type Engine, type Kind, monty } from "./engines.ts";

const argv = process.argv.slice(2);
const opt = (name: string) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
};
// --quick: the gate's smoke run (one sample, no warm-up), which checks the paths, not times.
const quick = argv.includes("--quick");
const SAMPLES = quick ? 1 : Number(opt("--reps") ?? 9);
const WARM = quick ? 0 : 2;
const now = () => performance.now();
const median = (xs: number[]) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length % 2 ? s[s.length >> 1] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};

// ---- Payloads ------------------------------------------------------------------------------

let seed = 11;
const rnd = (n: number) => {
  seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
  return seed % n;
};
const WORDS = ["config", "server", "request", "handler", "error", "value", "user", "data", "index", "parse", "result", "render", "cache", "token", "session"];
const pick = <T>(a: T[]): T => a[rnd(a.length)];
const sentence = (n: number) => Array.from({ length: n }, () => pick(WORDS)).join(" ");
const utf8 = (s: string) => Buffer.byteLength(s, "utf8");

/** A search tool's result of about `bytes` bytes of JSON: what most tool results look like. */
function searchResult(bytes: number) {
  const results: unknown[] = [];
  const r = { query: "handler error", total: 0, truncated: false, results };
  let size = 60;
  while (size < bytes) {
    const hit = {
      path: `src/${pick(WORDS)}/${pick(WORDS)}_${results.length}.ts`,
      line: 1 + rnd(4000),
      score: rnd(1000) / 1000,
      text: sentence(6 + rnd(10)),
      tags: [pick(WORDS), pick(WORDS)],
      match: { start: rnd(80), end: 80 + rnd(40) },
    };
    size += JSON.stringify(hit).length + 1;
    results.push(hit);
  }
  r.total = results.length;
  return r;
}

/** Source-like ASCII text of `bytes` bytes. */
function sourceText(bytes: number) {
  const lines: string[] = [];
  let size = 0;
  while (size < bytes) {
    const l = `${" ".repeat(rnd(4) * 2)}const ${pick(WORDS)}${lines.length} = ${pick(WORDS)}(${rnd(100)}); // ${sentence(5)}`;
    lines.push(l);
    size += l.length + 1;
  }
  return lines.join("\n").slice(0, bytes);
}

/** Prose of about `bytes` UTF-8 bytes with accented letters, dashes and CJK: two-byte strings in every JS engine. */
function unicodeText(bytes: number) {
  const words = [...WORDS, "naïve", "größe", "café", "—", "日本語", "über", "→", "résumé"];
  const parts: string[] = [];
  let size = 0;
  while (size < bytes) {
    const w = pick(words);
    parts.push(w);
    size += utf8(w) + 1;
  }
  return parts.join(" ");
}

function nested(depth: number) {
  let v: Record<string, unknown> = { leaf: true };
  for (let i = depth - 1; i >= 0; i--) v = { level: i, name: `node ${i}`, child: v };
  return v;
}

interface Payload {
  name: string;
  what: string;
  kind: Kind;
  value: unknown;
  /** Its size: the JSON text's UTF-8 bytes, or the bytes. */
  bytes: number;
}
const MB = 1_000_000;
const make: [string, string, Kind, () => unknown][] = [
  ["args", "a tool call's arguments", "json", () => ({ path: "src/server/handler.ts", offset: 120, limit: 200, flags: ["--literal", "-n"], recursive: false })],
  ["json-1k", "a search result, 1 KB of JSON", "json", () => searchResult(1000)],
  ["json-100k", "the same, 100 KB", "json", () => searchResult(100_000)],
  ["json-1m", "the same, 1 MB", "json", () => searchResult(MB)],
  ["json-10m", "the same, 10 MB", "json", () => searchResult(10 * MB)],
  ["text-1m", "a 1 MB ASCII string (source text)", "string", () => sourceText(MB)],
  ["text-10m", "a 10 MB ASCII string", "string", () => sourceText(10 * MB)],
  ["unicode-1m", "a 1 MB string with non-ASCII characters", "string", () => unicodeText(MB)],
  ["numbers", "100,000 floating-point numbers", "json", () => Array.from({ length: 100_000 }, () => rnd(1e9) / 1000)],
  ["bytes-1m", "a 1 MiB Uint8Array", "bytes", () => Uint8Array.from({ length: 1 << 20 }, () => rnd(256))],
  ["nested", "objects nested 100 deep", "json", () => nested(100)],
];
const wantPayloads = opt("--payloads")?.split(",");
const PAYLOADS: Payload[] = make
  .filter(([n]) => !wantPayloads || wantPayloads.includes(n))
  .map(([name, what, kind, f]) => {
    const value = f();
    const bytes = kind === "bytes" ? (value as Uint8Array).length : utf8(kind === "string" ? (value as string) : JSON.stringify(value));
    return { name, what, kind, value, bytes };
  });

// ---- End-to-end cases ------------------------------------------------------------------------

const FILE = sourceText(MB);
const SEARCH = searchResult(MB);
/** The host's one tool: `op` says what it does. */
function tool(a: unknown): unknown {
  // monty hands dicts over as Maps.
  const arg = (k: string) => (a instanceof Map ? a.get(k) : (a as Record<string, unknown>)[k]);
  switch (arg("op")) {
    case "search":
      return SEARCH;
    case "read":
      return FILE;
    case "write":
      return { ok: true, bytes: (arg("content") as string).length };
    default:
      return { ok: true, size: Number(arg("offset")) * 2, path: arg("path") };
  }
}
interface Case {
  name: string;
  what: string;
  js: string;
  py: string;
}
const CASES: Case[] = [
  {
    name: "field-1m",
    what: "a tool returns a 1 MB search result; the script reads one field",
    js: 'const r = tool({ op: "search", q: "handler" });\nreturn r.results[0].path;',
    py: 'r = tool({"op": "search", "q": "handler"})\nr["results"][0]["path"]',
  },
  {
    name: "edit-1m",
    what: "a tool returns a 1 MB file; the script edits it and passes it to a second tool",
    js: 'const t = tool({ op: "read", path: "a.ts" });\nconst u = t.replace("handler", "Handler");\ntool({ op: "write", path: "a.ts", content: u });\nreturn u.length;',
    py: 't = tool({"op": "read", "path": "a.ts"})\nu = t.replace("handler", "Handler", 1)\ntool({"op": "write", "path": "a.ts", "content": u})\nlen(u)',
  },
  {
    name: "calls-1000",
    what: "a thousand calls with small arguments and results",
    js: 'let n = 0;\nfor (let i = 0; i < 1000; i++) n += tool({ op: "stat", path: "src/f" + i + ".ts", offset: i, limit: 100 }).size;\nreturn n;',
    py: 'n = 0\nfor i in range(1000):\n    n += tool({"op": "stat", "path": "src/f" + str(i) + ".ts", "offset": i, "limit": 100})["size"]\nn',
  },
];
const EXPECTED: Record<string, string> = {
  "field-1m": SEARCH.results[0] && (SEARCH.results[0] as { path: string }).path,
  "edit-1m": String(FILE.length),
  "calls-1000": String(999 * 1000),
};

// ---- Checks --------------------------------------------------------------------------------

/** Host values as JSON-like data: Maps (monty's dicts) as objects, byte arrays as arrays. */
function plain(v: unknown): unknown {
  if (v instanceof Map) return Object.fromEntries([...v].map(([k, x]) => [k, plain(x)]));
  if (v instanceof Uint8Array) return Buffer.from(v).toString("base64");
  if (Array.isArray(v)) return v.map(plain);
  if (v && typeof v === "object") return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, plain(x)]));
  return v;
}
const same = (a: unknown, b: unknown) => JSON.stringify(plain(a)) === JSON.stringify(plain(b));

// ---- Running -------------------------------------------------------------------------------

let engines: Engine[] = [...allJs(), monty()];
if (opt("--engines")) {
  const want = opt("--engines")!.split(",");
  engines = engines.filter((e) => want.includes(e.name) || want.includes(slug(e.name)));
}
function slug(name: string): string {
  return name.replace(/[^a-z0-9]+/gi, "-").replace(/-$/, "").toLowerCase();
}

/** Operations per sample: enough that a sample of a small payload takes a while. */
const opsPer = (bytes: number) => Math.max(1, Math.min(200, Math.ceil(200_000 / bytes)));

interface Cell {
  ms: number;
  /** Linear memory after the operation, in a fresh VM (after `in`, then after `out`). */
  memory: number;
  path: string;
}
interface PathResult {
  payloads: Record<string, { in: Cell; out: Cell } | null>;
  e2e: Record<string, Cell>;
}
interface EngineResult {
  name: string;
  version: string;
  idleMemory: number;
  /** Per payload and direction, and per case, the best path's numbers. */
  payloads: Record<string, { in: Cell; out: Cell } | null>;
  e2e: Record<string, Cell>;
  /** Every path's. */
  paths: Record<string, PathResult>;
}

const startLoad = loadavg()[0];
console.log(`${cpus()[0].model}, ${cpus().length} cores, ${platform()} ${release()}, node ${process.version}; load ${loadavg().map((x) => x.toFixed(1)).join(" ")}; ${SAMPLES} samples after ${WARM} warm-up`);
if (startLoad > 4 && !quick) console.log("warning: load above 4: times are not comparable with recorded ones");
for (const p of PAYLOADS) console.log(`  ${p.name.padEnd(11)} ${(p.bytes / 1000).toFixed(1).padStart(9)} KB  ${p.what}`);

const results: EngineResult[] = [];
for (const e of engines) {
  await e.load();
  const idle = await e.bridge!();
  const r: EngineResult = { name: e.name, version: e.version, idleMemory: idle.memory(), payloads: {}, e2e: {}, paths: {} };
  idle.dispose();
  const paths = e.bridgePaths ?? ["default"];
  for (const path of paths) {
    const vmFor = () => e.bridge!(path === "default" ? undefined : path) as Promise<BridgeVm>;
    const pr: PathResult = { payloads: {}, e2e: {} };
    for (const p of PAYLOADS) {
      // Correctness, and the memory of one operation each way.
      const vm = await vmFor();
      let m1 = 0;
      let m2 = 0;
      try {
        vm.put(p.value, p.kind);
        m1 = vm.memory();
        const back = vm.get(p.kind);
        m2 = vm.memory();
        if (!same(back, p.value)) throw new Error("the value read back differs");
      } catch (err) {
        // An engine that cannot take the payload (a nesting limit) has no numbers for it.
        console.log(`${e.name} (${path}) ${p.name}: ${String((err as Error).message).split("\n").slice(-1)[0]}`);
        pr.payloads[p.name] = null;
        continue;
      } finally {
        vm.dispose();
      }
      const n = opsPer(p.bytes);
      const ins: number[] = [];
      const outs: number[] = [];
      for (let i = 0; i < WARM + SAMPLES; i++) {
        let vm = await vmFor();
        let t = now();
        for (let k = 0; k < n; k++) vm.put(p.value, p.kind);
        const tIn = (now() - t) / n;
        vm.dispose();
        vm = await vmFor();
        vm.put(p.value, p.kind);
        t = now();
        for (let k = 0; k < n; k++) vm.get(p.kind);
        const tOut = (now() - t) / n;
        vm.dispose();
        if (i >= WARM) {
          ins.push(tIn);
          outs.push(tOut);
        }
      }
      pr.payloads[p.name] = { in: { ms: median(ins), memory: m1, path }, out: { ms: median(outs), memory: m2, path } };
      console.error(`${e.name} ${path} ${p.name}: in ${median(ins).toFixed(3)} ms, out ${median(outs).toFixed(3)} ms`);
    }
    for (const c of CASES) {
      const program = e.bridgeProgram!(e.language === "python" ? c.py : c.js);
      const runs: number[] = [];
      for (let i = 0; i < WARM + SAMPLES; i++) {
        const vm = await vmFor();
        const t = now();
        const out = vm.call(program, tool);
        const dt = now() - t;
        vm.dispose();
        if (String(out) !== EXPECTED[c.name]) throw new Error(`${e.name} (${path}) ${c.name}: ${String(out).slice(0, 100)}, expected ${EXPECTED[c.name]}`);
        if (i >= WARM) runs.push(dt);
      }
      pr.e2e[c.name] = { ms: median(runs), memory: 0, path };
      console.error(`${e.name} ${path} ${c.name}: ${median(runs).toFixed(3)} ms`);
    }
    r.paths[path] = pr;
  }
  const best = (a: Cell | undefined, b: Cell) => (a && a.ms <= b.ms ? a : b);
  for (const pr of Object.values(r.paths)) {
    for (const [k, v] of Object.entries(pr.payloads)) {
      if (!v) {
        if (!(k in r.payloads)) r.payloads[k] = null;
        continue;
      }
      const cur = r.payloads[k];
      r.payloads[k] = { in: best(cur?.in, v.in), out: best(cur?.out, v.out) };
    }
    for (const [k, v] of Object.entries(pr.e2e)) r.e2e[k] = best(r.e2e[k], v);
  }
  results.push(r);
}

// ---- Report --------------------------------------------------------------------------------

const fmt = (v: number) => (v >= 100 ? v.toFixed(0) : v >= 10 ? v.toFixed(1) : v >= 1 ? v.toFixed(2) : v.toFixed(3));
const w = (r: EngineResult) => Math.max(10, r.name.length);
/** A cell: the time, and the path when the engine has several. */
const cell = (r: EngineResult, c: Cell | undefined) => (c ? fmt(c.ms) + (Object.keys(r.paths).length > 1 ? ` ${c.path[0]}` : "") : "-").padStart(w(r));
const head = (first: string) => [first.padEnd(11), ...results.map((r) => r.name.padStart(w(r)))].join(" ");
for (const dir of ["in", "out"] as const) {
  console.log(`\n${dir}: ms per operation (median); the letter names the best path`);
  console.log(head("payload"));
  for (const p of PAYLOADS) console.log([p.name.padEnd(11), ...results.map((r) => cell(r, r.payloads[p.name]?.[dir]))].join(" "));
}
console.log("\nend to end: ms per run (median)");
console.log(head("case"));
for (const c of CASES) console.log([c.name.padEnd(11), ...results.map((r) => cell(r, r.e2e[c.name]))].join(" "));

const json = opt("--json");
if (json) {
  const payloads = PAYLOADS.map(({ name, what, kind, bytes }) => ({ name, what, kind, bytes }));
  writeFileSync(json, JSON.stringify({ load: startLoad, payloads, cases: CASES.map(({ name, what }) => ({ name, what })), engines: results }, null, 1));
}
