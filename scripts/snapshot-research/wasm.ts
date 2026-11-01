// snapshot-research on wasm: the cases run through the pi adapter (js/pi/vm.ts) on djs-wasm,
// with a snapshot and a resume at every idle point, measuring the snapshot container
// (sizes, snapshot() and resumeVm() times) and, at the first, the largest and the last idle
// point of each case, what other encodings of the same image would cost (zstd levels and
// windows, with the prelude image, the base image or no dictionary).
//
//   node scripts/snapshot-research/wasm.ts [out-dir] [case.js ...]   ($DJS_WASM: the module)

import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { constants as Z, zstdCompressSync, zstdDecompressSync } from "node:zlib";
import { defaultWasmPath, loadModule, Sandbox } from "../../js/src/sandbox.ts";
import { Engine, instance, instanceBase } from "../../js/pi/engine.ts";
import { resumeVm, startVm, type VmSession, type WorkerToHostMessage } from "../../js/pi/vm.ts";
import { APP_PI_SESSION, encodeSnapshot, readSnapshotHeader } from "../../js/src/snapshot.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const args = process.argv.slice(2);
const out = args[0] ?? join(root, "work/scratch/snapshot-research");
const cases = args.length > 1 ? args.slice(1) : readdirSync(join(here, "cases")).sort().map((f) => join(here, "cases", f));
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
      return { ok: true, value: Array.from({ length: Number(a.limit ?? 10) }, (_, i) => ({ path: files[(i * 7 + 3) % files.length].path, score: 1 / (i + 1) })) };
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

// ---- Images seen: the last one snapshot() took, and the first (the prelude image) ------

let lastImage: Uint8Array | undefined;
let preludeImage: Uint8Array | undefined;
const image = Engine.prototype.image;
Engine.prototype.image = function (this: Engine) {
  const img = image.call(this);
  preludeImage ??= img;
  lastImage = img;
  return img;
};

const wasm = await loadModule();
const base = instanceBase(instance(wasm))!.bytes;

interface Point {
  idle: number;
  image: Uint8Array;
  container: number;
  snapshotMs: number;
  resumeMs: number;
  plainResumeMs: number;
}

/** Runs one case; at every idle point snapshot(), then continue on resumeVm(bytes). */
async function runCase(code: string): Promise<{ points: Point[]; end: WorkerToHostMessage }> {
  failedPage2 = false;
  const points: Point[] = [];
  const queue: { id: number; name: string; args?: string }[] = [];
  let session!: VmSession;
  let seq = 0;
  const hooks = {
    wasm,
    shouldInterrupt: () => false,
    post: (m: WorkerToHostMessage) => {
      if (m.type === "call") queue.push({ id: m.id, name: m.name, args: m.args });
      if (m.type === "done" || m.type === "crash") finish(m);
      if (m.type === "idle") setTimeout(step);
    },
  };
  let finish!: (m: WorkerToHostMessage) => void;
  const ended = new Promise<WorkerToHostMessage>((resolve) => (finish = resolve));
  const step = () => {
    if (queue.length === 0) return;
    let t = performance.now();
    const bytes = session.snapshot();
    const snapshotMs = performance.now() - t;
    const img = lastImage!;
    // The same image in an uncompressed container, for the restore baseline.
    const header = readSnapshotHeader(bytes)!;
    const plain = encodeSnapshot({ app: APP_PI_SESSION, build: header.build, meta: header.meta, image: img, codec: null });
    t = performance.now();
    resumeVm(plain, hooks).dispose();
    const plainResumeMs = performance.now() - t;
    t = performance.now();
    const resumed = resumeVm(bytes, hooks);
    const resumeMs = performance.now() - t;
    session.dispose();
    session = resumed;
    points.push({ idle: points.length, image: img, container: bytes.length, snapshotMs, resumeMs, plainResumeMs });
    const call = queue.shift()!;
    const r = tool(call.name, call.args === undefined ? {} : JSON.parse(call.args));
    seq++;
    session.deliver({
      type: "result",
      id: call.id,
      ok: r.ok,
      payload: r.ok ? JSON.stringify(r.value) : String(r.value),
      seq,
      now: 1_762_000_000_000 + 1500 * seq,
    });
  };
  session = await startVm({
    ...hooks,
    prelude: PRELUDE_SOURCE,
    data: {
      code,
      tools: TOOLS.map((name) => ({ name, jsName: name, description: `The ${name} tool.` })),
      globals: [],
      memoryLimitBytes: undefined,
      store: {},
      deterministic: { startedAt: 1_762_000_000_000, seed: 42 },
    },
  });
  const end = await ended;
  session.dispose();
  return { points, end };
}

// ---- Encodings of one image ----------------------------------------------------------

function windowLog(n: number): number {
  let log = 10;
  while (log < 31 && 2 ** log < n) log++;
  return log;
}

function zstd(img: Uint8Array, dict: Uint8Array | undefined, level: number, window: boolean) {
  const params: Record<number, number> = { [Z.ZSTD_c_compressionLevel]: level, [Z.ZSTD_c_checksumFlag]: 1 };
  if (window) params[Z.ZSTD_c_windowLog] = windowLog((dict?.length ?? 0) + img.length);
  const opts = dict === undefined ? { params } : { params, dictionary: dict };
  const t0 = performance.now();
  let c!: Uint8Array;
  const reps = level >= 15 ? 1 : 5;
  for (let i = 0; i < reps; i++) c = zstdCompressSync(img, opts);
  const compressMs = (performance.now() - t0) / reps;
  const t1 = performance.now();
  for (let i = 0; i < 5; i++) zstdDecompressSync(c, dict === undefined ? {} : { dictionary: dict });
  return { size: c.length, compressMs, decompressMs: (performance.now() - t1) / 5 };
}

const kb = (n: number) => (n / 1024).toFixed(1);
const med = (xs: number[]) => [...xs].sort((a, b) => a - b)[xs.length >> 1];
const ms = (x: number) => x.toFixed(2);

let report = `djs-wasm ${String(defaultWasmPath())}; node ${process.version}; base image ${kb(base.length)} KB\n\n`;
report += "| case | idle points | image (collected) median / max | container median / max | snapshot() ms median / max | resumeVm ms median / max | uncompressed resumeVm ms median |\n";
report += "| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n";
let sweeps = "";
for (const path of cases) {
  const name = basename(path, ".js");
  const { points, end } = await runCase(readFileSync(path, "utf8"));
  if (end.type !== "done" || !end.ok) throw new Error(`${name}: ${JSON.stringify(end)}`);
  const f = (k: keyof Point) => points.map((p) => p[k] as number);
  const imgs = points.map((p) => p.image.length);
  report += `| ${name} | ${points.length} | ${kb(med(imgs))} / ${kb(Math.max(...imgs))} | ${kb(med(f("container")))} / ${kb(Math.max(...f("container")))} | ${ms(med(f("snapshotMs")))} / ${ms(Math.max(...f("snapshotMs")))} | ${ms(med(f("resumeMs")))} / ${ms(Math.max(...f("resumeMs")))} | ${ms(med(f("plainResumeMs")))} |\n`;
  writeFileSync(join(out, `${name}.points.json`), JSON.stringify(points.map(({ image, ...p }) => ({ ...p, image: image.length }))));
  writeFileSync(join(out, `${name}-last.img`), points[points.length - 1].image);
  const maxAt = imgs.indexOf(Math.max(...imgs));
  for (const i of [...new Set([0, maxAt, points.length - 1])]) {
    const img = points[i].image;
    sweeps += `\n${name} idle ${i}: image ${kb(img.length)} KB, container ${kb(points[i].container)} KB\n`;
    sweeps += "| dictionary | level | window | size KB | compress ms | decompress ms |\n| --- | ---: | --- | ---: | ---: | ---: |\n";
    for (const [dname, dict] of [["prelude image", preludeImage], ["base image", base], ["none", undefined]] as const) {
      for (const level of dict === undefined ? [3, 19] : [1, 3, 6, 9, 19]) {
        for (const window of dict === undefined ? [false] : level === 3 ? [false, true] : [true]) {
          const z = zstd(img, dict, level, window);
          sweeps += `| ${dname} | ${level} | ${window ? "covers both" : "default"} | ${kb(z.size)} | ${ms(z.compressMs)} | ${ms(z.decompressMs)} |\n`;
        }
      }
    }
  }
}
// The Sandbox: compressed against the base image.
report += "\n| sandbox | image KB | bytes() KB | bytes() ms | fromBytes ms | uncompressed fromBytes ms |\n| --- | ---: | ---: | ---: | ---: | ---: |\n";
for (const [label, setup] of [
  ["new", ""],
  ["1000 records", "globalThis.rows = Array.from({ length: 1000 }, (_, i) => ({ id: i, name: 'row ' + i, tags: ['a', 'b'] })); return 1"],
] as const) {
  const sb = await Sandbox.create({ module: wasm });
  if (setup) sb.run(setup);
  const times = (f: () => unknown) => {
    const t = performance.now();
    for (let i = 0; i < 20; i++) f();
    return (performance.now() - t) / 20;
  };
  const z = sb.bytes();
  const plain = sb.bytes({ compression: "none" });
  const bytesMs = times(() => sb.bytes());
  const restoreMs = times(() => new Sandbox({ module: wasm }, z).dispose());
  const plainMs = times(() => new Sandbox({ module: wasm }, plain).dispose());
  report += `| ${label} | ${kb(readSnapshotHeader(plain)!.imageLength)} | ${kb(z.length)} | ${ms(bytesMs)} | ${ms(restoreMs)} | ${ms(plainMs)} |\n`;
  sb.dispose();
}
report += `\nprelude image ${kb(preludeImage!.length)} KB\n${sweeps}`;
writeFileSync(join(out, "prelude.img"), preludeImage!);
writeFileSync(join(out, "wasm-report.md"), report);
console.log(report);
