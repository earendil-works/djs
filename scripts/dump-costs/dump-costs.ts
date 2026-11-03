// dump-costs (0184): format 2 dumps next to session images, on djs-wasm. For every case
// (scripts/snapshot-research/cases, the data-heavy scripts/dump-costs/cases and the
// harvested corpus, tests/corpus/codemode-harvest) a Sandbox runs the script host-driven
// and at every idle point measures
//   - the dump: `Run.dump()` time, text size raw and zstd (levels 3 and 19),
//     `Sandbox.resume()` time (a new sandbox included; the bare `Sandbox.create` is given
//     separately);
//   - the image: `Sandbox.bytes()` (the container, zstd against the base) and its size
//     uncompressed, `Sandbox.fromBytes()` time;
// with packaging A (one module with the dumper) and B (the shipped module plus the dump
// module, D2635: dumps move the image to the dump module and back). The run goes on in the
// original sandbox. Writes work/scratch/dump-costs/report.md (or OUT/report.md).
//
//   scripts/build-wasm.sh && scripts/build-wasm.sh --dump
//   node scripts/dump-costs/dump-costs.ts [OUT] [--no-corpus]
//
// The tools are scripts/snapshot-research/wasm.ts's (real source files and prose).

import { readdirSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { constants as Z, zstdCompressSync } from "node:zlib";
import { loadModule, Sandbox, type Run, type SandboxOptions } from "../../js/src/sandbox.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const argv = process.argv.slice(2);
const noCorpus = argv.includes("--no-corpus");
const out = argv.filter((a) => !a.startsWith("--"))[0] ?? join(root, "work/scratch/dump-costs");
mkdirSync(out, { recursive: true });

// ---- Tools: real source files and real prose, so results compress like real data ------

function walk(dir: string): string[] {
  return readdirSync(dir)
    .sort()
    .flatMap((e) => {
      const p = join(dir, e);
      return statSync(p).isDirectory() ? walk(p) : p.endsWith(".rs") ? [p] : [];
    });
}
const files = walk(join(root, "crates/djs-core/src")).map((p) => ({ path: relative(root, p), text: readFileSync(p, "utf8") }));
/**
 * The search tool's step through `files`: coprime with their count, so the hits are
 * distinct files. A fixed 7 repeated every file three times in tool-heavy's 60 hits
 * whenever djs-core had 147 source files (0133 found it with `djs heap`'s duplicates).
 */
const gcd = (a: number, b: number): number => (b === 0 ? a : gcd(b, a % b));
let STRIDE = 7;
while (gcd(STRIDE, files.length) !== 1) STRIDE++;
const prose = ["docs/decisions.md", "ARCH.md", "TESTING.md", "docs/host.md"].flatMap((f) =>
  readFileSync(join(root, f), "utf8")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 30 && !l.startsWith("|") && !l.startsWith("#")),
);
const words = [...new Set(prose.flatMap((l) => l.split(/\s+/)).filter((w) => /^[a-zA-Z]{3,}$/.test(w)).map((w) => w.toLowerCase()))].sort();

class Rng {
  private s: number;
  constructor(seed: number) {
    this.s = seed;
  }
  next(): number {
    let x = this.s;
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    return (this.s = x >>> 0);
  }
  below(n: number): number {
    return this.next() % n;
  }
}
const sentence = (r: Rng, n: number) => Array.from({ length: n }, () => words[r.below(words.length)]).join(" ");
function text(r: Rng, n: number): string {
  let s = "";
  while (s.length < n) s += (s ? " " : "") + prose[r.below(prose.length)];
  return s.slice(0, n);
}

let failedPage2 = false;
function tool(name: string, a: Record<string, unknown>): { ok: boolean; value: unknown } {
  const r = new Rng((0x9e3779b9 ^ (JSON.stringify(a ?? null).length * 0x1000001)) >>> 0 || 1);
  switch (name) {
    case "search":
      return { ok: true, value: Array.from({ length: Number(a.limit ?? 10) }, (_, i) => ({ path: files[(i * STRIDE + 3) % files.length].path, score: 1 / (i + 1) })) };
    case "read": {
      const f = files.find((f) => f.path === a.path);
      return { ok: true, value: { path: a.path, text: (f?.text ?? "").split("\n").slice(0, 150).join("\n") } };
    }
    case "ask":
      return { ok: true, value: { choice: (a.options as string[] | undefined)?.[0] ?? "yes" } };
    case "shell":
      return { ok: true, value: { code: 0, stdout: Array.from({ length: 40 }, (_, i) => `[${String(i).padStart(2, "0")}/40] Compiling ${files[r.below(files.length)].path} (${sentence(r, 3)})`).join("\n") } };
    case "logs": {
      const routes = ["users", "orders", "items", "carts", "sessions"];
      let s = "";
      for (let i = 0; i < Number(a.lines ?? 1000); i++) {
        const err = r.below(20) === 0;
        s += `2026-11-03T10:${String((i / 3600) % 60 | 0).padStart(2, "0")}:${String((i / 60) % 60 | 0).padStart(2, "0")}.${String(i % 1000).padStart(3, "0")}Z ${err ? "ERROR" : "INFO"} req=${r.next().toString(16).padStart(8, "0")} route=/v1/${routes[r.below(5)]}/${r.below(100000)} ms=${r.below(900)}`;
        if (err) s += ` code=E${100 + r.below(17)} user=${r.below(100000)} msg="${sentence(r, 6)}"`;
        s += "\n";
      }
      return { ok: true, value: s };
    }
    case "lookup":
      return { ok: true, value: { owner: `team-${r.below(9)}`, runbook: `https://wiki.example.com/runbooks/${r.below(1000)}` } };
    case "list_issues": {
      const labels = ["bug", "feature", "p1", "p2", "p3", "api", "ui", "infra", "docs", "flaky", "perf", "security"];
      return {
        ok: true,
        value: Array.from({ length: Number(a.limit ?? 100) }, (_, i) => ({
          number: 1000 + i,
          title: sentence(r, 7),
          state: "open",
          labels: Array.from({ length: 1 + r.below(3) }, () => labels[r.below(12)]),
          user: { login: `user${r.below(80)}`, id: r.below(1_000_000) },
          points: 1 + r.below(8),
          created_at: `2026-${String(1 + r.below(12)).padStart(2, "0")}-${String(1 + r.below(28)).padStart(2, "0")}T12:00:00Z`,
          body: text(r, 160 + r.below(300)),
        })),
      };
    }
    case "issue":
      return { ok: true, value: { number: a.number, comments: Array.from({ length: 2 + r.below(6) }, () => ({ author: `user${r.below(80)}`, body: text(r, 60 + r.below(400)) })) } };
    case "search_page": {
      const page = Number(a.cursor ?? 0);
      if (page === 2 && !failedPage2) {
        failedPage2 = true;
        return { ok: false, value: "rate limited" };
      }
      const owners = ["alice", "bob", "carol", "dave", "erin", "frank", "grace"];
      return {
        ok: true,
        value: { items: Array.from({ length: 20 }, (_, i) => ({ id: (page * 20 + i) % 90, owner: owners[((page * 20 + i) % 90) % 7], title: sentence(r, 6) })), next: page < 5 ? page + 1 : null },
      };
    }
    case "status":
      return { ok: true, value: { state: "open" } };
  }
  return { ok: false, value: `unknown tool ${name}` };
}
const TOOLS = ["search", "read", "ask", "shell", "logs", "lookup", "list_issues", "issue", "search_page", "status"];


// ---- Modules ---------------------------------------------------------------------------

const mainPath = process.env.DJS_WASM ?? join(root, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm");
const dumpPath = process.env.DJS_DUMP_WASM ?? join(root, "target/djs-dump.wasm");
const main = await loadModule(mainPath);
const dumper = await loadModule(dumpPath);
const PACKAGINGS: [string, SandboxOptions][] = [
  ["A (dump module alone)", { module: dumper }],
  ["B (djs.wasm + djs-dump.wasm)", { module: main, dumpModule: dumper }],
];
const DET = { deterministic: { startedAt: 1_762_000_000_000, seed: 42 } };

interface Point {
  dumpMs: number;
  raw: number;
  z3: number;
  z19: number;
  resumeMs: number;
  container: number;
  image: number;
  bytesMs: number;
  fromBytesMs: number;
}

const zsize = (s: Uint8Array, level: number) =>
  zstdCompressSync(s, { params: { [Z.ZSTD_c_compressionLevel]: level } }).length;
const time = <T>(f: () => T): [T, number] => {
  const t = performance.now();
  const v = f();
  return [v, performance.now() - t];
};
async function timeAsync<T>(f: () => Promise<T>): Promise<[T, number]> {
  const t = performance.now();
  const v = await f();
  return [v, performance.now() - t];
}

/** Measures one idle point of `run` in `sb`. */
async function measure(sb: Sandbox, run: Run, opts: SandboxOptions, z19: boolean): Promise<Point | string> {
  let dump: string;
  let dumpMs: number;
  try {
    [dump, dumpMs] = time(() => run.dump());
  } catch (e) {
    // A refusal: its first reason, without the path.
    return String((e as Error).message).split("\n")[1]?.replace(/^.*?: /, "") ?? String(e);
  }
  const text = new TextEncoder().encode(dump);
  const [r, resumeMs] = await timeAsync(() => Sandbox.resume(dump, opts));
  r.sandbox.dispose();
  const [bytes, bytesMs] = time(() => sb.bytes());
  const image = sb.bytes({ compression: "none" }).length;
  const [s2, fromBytesMs] = time(() => new Sandbox(opts, bytes));
  s2.dispose();
  return {
    dumpMs,
    raw: text.length,
    z3: zsize(text, 3),
    z19: z19 ? zsize(text, 19) : 0,
    resumeMs,
    container: bytes.length,
    image,
    bytesMs,
    fromBytesMs,
  };
}

/** Runs `code` as a body in a sandbox made from `opts`, measuring every idle point. */
async function runCase(
  code: string,
  opts: SandboxOptions,
  answer: (fn: string, args: unknown[]) => { ok: boolean; value: unknown },
  z19: boolean,
): Promise<{ points: Point[]; refused: string[]; state: string }> {
  const sb = new Sandbox(opts);
  const points: Point[] = [];
  const refused: string[] = [];
  const run = sb.begin(code);
  for (let guard = 0; run.state === "idle" && guard < 10000; guard++) {
    const m = await measure(sb, run, opts, z19);
    if (typeof m === "string") refused.push(m);
    else points.push(m);
    const c = run.calls[0];
    run.settle(c.id, answer(c.fn, c.args));
    await run.advance();
  }
  sb.dispose();
  lastError = run.error;
  return { points, refused, state: run.state };
}

const toolOpts = (base: SandboxOptions): SandboxOptions => ({
  ...base,
  ...DET,
  console: { log() {}, info() {}, warn() {}, error() {} },
  // pi's prelude globals the cases use.
  functions: { text: () => undefined, store: () => undefined },
  asyncFunctions: Object.fromEntries(TOOLS.map((t) => [`tools.${t}`, () => 0])),
});
const toolAnswer = (fn: string, args: unknown[]) => {
  const name = fn.replace(/^tools\./, "");
  if (name === "shell" && (args[0] as { cmd?: string })?.cmd === "blob") {
    let s = "";
    for (let i = 0; s.length < 1 << 16; i++) s += files[i % files.length].text;
    return { ok: true, value: { code: 0, stdout: Buffer.from(s.slice(0, 1 << 16)).toString("base64") } };
  }
  return tool(name, (args[0] ?? {}) as Record<string, unknown>);
};

// The harvested corpus: `later(v, rejected)` answers at the next idle point, `text`
// prints (the difftest's codemode model, D2590).
const corpusOpts = (base: SandboxOptions): SandboxOptions => ({
  ...base,
  ...DET,
  console: { log() {}, info() {}, warn() {}, error() {} },
  functions: { text: () => undefined },
  asyncFunctions: { later: () => 0 },
});
const corpusAnswer = (_fn: string, args: unknown[]) => ({ ok: args[1] !== true, value: args[0] });

// ---- Report ----------------------------------------------------------------------------

const kb = (n: number) => (n / 1024).toFixed(1);
const ms = (x: number) => x.toFixed(2);
const pct = (xs: number[], p: number) => {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))];
};
const col = (ps: Point[], k: keyof Point) => ps.map((p) => p[k]);

let report = `djs-wasm: A ${relative(root, dumpPath)}, B ${relative(root, mainPath)} + ${relative(root, dumpPath)}; node ${process.version}\n`;
{
  const t = performance.now();
  for (let i = 0; i < 20; i++) new Sandbox(toolOpts({ module: main })).dispose();
  report += `\nA new Sandbox (B's main module, 20 times): ${ms((performance.now() - t) / 20)} ms; Sandbox.resume includes one.\n`;
}

const caseFiles = [
  ...readdirSync(join(root, "scripts/snapshot-research/cases")).sort().map((f) => join(root, "scripts/snapshot-research/cases", f)),
  ...readdirSync(join(here, "cases")).sort().map((f) => join(here, "cases", f)),
];
const head = "| case | idle (refused) | dump raw KB med / max | zstd-3 KB med / max | zstd-19 KB max | container KB med / max | image KB med / max | dump ms med / max | resume ms med / max | bytes() ms med / max | fromBytes ms med / max |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n";
const row = (name: string, ps: Point[], refused: number) => {
  if (ps.length === 0) return `| ${name} | 0 (${refused}) |\n`;
  const m = (k: keyof Point, f: (x: number) => string) => `${f(pct(col(ps, k), 50))} / ${f(Math.max(...col(ps, k)))}`;
  return `| ${name} | ${ps.length + refused}${refused ? ` (${refused})` : ""} | ${m("raw", kb)} | ${m("z3", kb)} | ${kb(Math.max(...col(ps, "z19")))} | ${m("container", kb)} | ${m("image", kb)} | ${m("dumpMs", ms)} | ${m("resumeMs", ms)} | ${m("bytesMs", ms)} | ${m("fromBytesMs", ms)} |\n`;
};
const all: Record<string, unknown> = {};
const reasons = new Map<string, number>();
let lastError: unknown;
for (const [pk, base] of PACKAGINGS) {
  report += `\n### Cases, packaging ${pk}\n\n${head}`;
  for (const path of caseFiles) {
    failedPage2 = false;
    const name = basename(path, ".js");
    const { points, refused, state } = await runCase(readFileSync(path, "utf8"), toolOpts(base), toolAnswer, true);
    if (state !== "done") throw new Error(`${name}: ${state} ${String(lastError)}`);
    report += row(name, points, refused.length);
    for (const r of new Set(refused)) reasons.set(r, (reasons.get(r) ?? 0) + refused.filter((x) => x === r).length);
    all[`${pk}/${name}`] = points;
    console.error(`${pk} ${name}: ${points.length} idle points`);
  }
}

if (!noCorpus) {
  const dir = join(root, "tests/corpus/codemode-harvest");
  const names = readdirSync(dir).filter((f) => f.endsWith(".js")).sort();
  const [pk, base] = PACKAGINGS[1];
  const points: Point[] = [];
  let ran = 0;
  let failed = 0;
  let refusedPoints = 0;
  const corpusReasons = new Map<string, number>();
  for (const f of names) {
    try {
      const { points: ps, refused } = await runCase(readFileSync(join(dir, f), "utf8"), corpusOpts(base), corpusAnswer, false);
      ran++;
      points.push(...ps);
      refusedPoints += refused.length;
      for (const r of refused) corpusReasons.set(r, (corpusReasons.get(r) ?? 0) + 1);
    } catch {
      failed++;
    }
  }
  all.corpus = points;
  const dist = (k: keyof Point, f: (x: number) => string) =>
    [50, 90, 99, 100].map((p) => f(pct(col(points, k), p))).join(" / ");
  report += `\n### The harvested corpus, packaging ${pk}\n\n${ran} cases ran (${failed} failed to start), ${points.length + refusedPoints} idle points, ${refusedPoints} refused${[...corpusReasons].map(([r, n]) => ` (${n}: ${r})`).join("")}.\n\n`;
  report += "| at every idle point | p50 / p90 / p99 / max |\n| --- | --- |\n";
  report += `| dump raw KB | ${dist("raw", kb)} |\n| dump zstd-3 KB | ${dist("z3", kb)} |\n| container KB | ${dist("container", kb)} |\n| image (uncompressed) KB | ${dist("image", kb)} |\n`;
  report += `| dump ms | ${dist("dumpMs", ms)} |\n| resume ms | ${dist("resumeMs", ms)} |\n| bytes() ms | ${dist("bytesMs", ms)} |\n| fromBytes ms | ${dist("fromBytesMs", ms)} |\n`;
  const ratio = points.map((p) => p.z3 / p.container);
  report += `| dump zstd-3 / container | ${[50, 90, 99, 100].map((p) => pct(ratio, p).toFixed(2)).join(" / ")} |\n`;
}
if (reasons.size) report += `\nRefused in the cases (packagings together): ${[...reasons].map(([r, n]) => `${n}: ${r}`).join("; ")}\n`;
writeFileSync(join(out, "report.md"), report);
writeFileSync(join(out, "points.json"), JSON.stringify(all));
console.log(report);
