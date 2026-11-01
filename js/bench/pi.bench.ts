// pi codemode on djs vs quickjs-wasi, through the same VM contract (`startVm`), on the
// calling thread: djs through js/pi/vm.ts, quickjs-wasi through pi's own vm.ts, imported
// read-only from pi-private ($PI_PRIVATE, default ~/Development/pi-private) when it is there.
// Both run pi's prelude from that checkout (djs falls back to the tests/pi copy).
//
//   node js/bench/pi.bench.ts           (DJS_WASM overrides the djs-wasm build)

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { defaultWasmPath } from "../src/sandbox.ts";
import { startVm as djsStartVm } from "../pi/vm.ts";
import { PRELUDE_SOURCE as FIXTURE_PRELUDE } from "../../tests/pi/prelude-source.ts";

type Message = { type: string; id?: number; args?: string; ok?: boolean; error?: string; message?: string };
type Session = { deliver(m: unknown): void; dispose(): void };
type Start = (options: {
  data: unknown;
  wasm: object;
  prelude: string;
  reuseImages?: boolean;
  post(m: Message): void;
  shouldInterrupt(): boolean;
}) => Promise<Session>;

const pi = process.env.PI_PRIVATE ?? join(homedir(), "Development/pi-private");
const piSrc = join(pi, "packages/codemode/src");
const hasPi = existsSync(join(pi, "node_modules/quickjs-wasi/package.json")) && existsSync(join(piSrc, "runtime/vm.ts"));

const engines: { name: string; start: Start; wasm: object; prelude: string; reuseImages?: boolean }[] = [];
const djsPath = defaultWasmPath();
const djsWasm = await WebAssembly.compile(readFileSync(typeof djsPath === "string" ? djsPath : fileURLToPath(djsPath)));
let prelude = FIXTURE_PRELUDE;
if (hasPi) {
  const vm = (await import(join(piSrc, "runtime/vm.ts"))) as { startVm: Start };
  const wasm = (await import(join(piSrc, "wasm.ts"))) as { loadQuickJSWasm(): Promise<object> };
  prelude = ((await import(join(piSrc, "runtime/prelude-source.ts"))) as { PRELUDE_SOURCE: string }).PRELUDE_SOURCE;
  engines.push({ name: "quickjs-wasi", start: vm.startVm, wasm: await wasm.loadQuickJSWasm(), prelude });
} else {
  console.log(`(no pi-private at ${pi}: djs only)`);
}
engines.push({ name: "djs, prelude image reused", start: djsStartVm as unknown as Start, wasm: djsWasm, prelude });
engines.push({ name: "djs, prelude run each time", start: djsStartVm as unknown as Start, wasm: djsWasm, prelude, reuseImages: false });

/** Runs `code` to its end, answering tool calls in a microtask like pi's in-thread launcher. */
function execute(engine: (typeof engines)[number], code: string, tool: (args: unknown) => unknown): Promise<void> {
  return new Promise((resolve, reject) => {
    let session: Session | undefined;
    const queue: Message[] = [];
    const answer = (m: Message) => {
      const value = tool(m.args === undefined ? undefined : JSON.parse(m.args));
      session!.deliver({ type: "result", id: m.id, ok: true, payload: JSON.stringify(value) });
    };
    engine
      .start({
        wasm: engine.wasm,
        prelude: engine.prelude,
        reuseImages: engine.reuseImages,
        data: {
          code,
          tools: [{ name: "search", jsName: "search", description: "" }, { name: "read", jsName: "read", description: "" }],
          globals: [],
          memoryLimitBytes: undefined,
          store: {},
        },
        post: (m) => {
          if (m.type === "call") {
            if (session) queueMicrotask(() => answer(m));
            else queue.push(m);
          } else if (m.type === "done" || m.type === "crash") {
            queueMicrotask(() => {
              session?.dispose();
              if (m.type === "crash" || m.ok === false) reject(new Error(m.error ?? m.message));
              else resolve();
            });
          }
        },
        shouldInterrupt: () => false,
      })
      .then((s) => {
        session = s;
        for (const m of queue.splice(0)) queueMicrotask(() => answer(m));
      }, reject);
  });
}

async function median(f: () => Promise<void>, n: number): Promise<number> {
  for (let i = 0; i < Math.max(3, n >> 3); i++) await f();
  const runs: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = performance.now();
    await f();
    runs.push(performance.now() - t);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1];
}

// A typical tool-heavy script: search, then read the hits in parallel batches, filter and
// summarize the results, print a table.
const TOOL_HEAVY = `
const hits = await tools.search({ query: "error handling", limit: 60 });
const files = [];
for (let i = 0; i < hits.length; i += 10) {
  const batch = await Promise.all(hits.slice(i, i + 10).map((hit) => tools.read({ path: hit.path })));
  files.push(...batch);
}
const matches = files
  .map((file) => ({ path: file.path, lines: file.text.split("\\n").filter((line) => line.includes("throw")) }))
  .filter((file) => file.lines.length > 0)
  .sort((a, b) => b.lines.length - a.lines.length);
for (const file of matches.slice(0, 10)) console.log(file.path.padEnd(40), String(file.lines.length).padStart(4));
store("seen", matches.map((file) => file.path).slice(0, 20));
return { files: files.length, matching: matches.length, top: matches[0]?.path };
`;
const LINES = Array.from({ length: 120 }, (_, i) => (i % 7 === 0 ? `  throw new Error("case ${i}");` : `  const value${i} = compute(${i}, "text");`));
const tool = (args: unknown): unknown => {
  const a = args as { query?: string; path?: string };
  if (a.query !== undefined) return Array.from({ length: 60 }, (_, i) => ({ path: `src/module${i}/file${i}.ts`, score: 1 / (i + 1) }));
  const n = Number(/\d+/.exec(a.path ?? "")?.[0] ?? 0);
  return { path: a.path, text: LINES.slice(0, 20 + n).join("\n") };
};

const cases: [string, string, number][] = [
  ["script start (`return 1`)", "return 1", 100],
  ["tool-heavy script (61 calls, 120 KB of results)", TOOL_HEAVY, 40],
];
console.log(`| case | ${engines.map((e) => e.name).join(" | ")} |`);
console.log(`| --- | ${engines.map(() => "---:").join(" | ")} |`);
for (const [name, code, n] of cases) {
  const cells: string[] = [];
  for (const engine of engines) cells.push(`${(await median(() => execute(engine, code, tool), n)).toFixed(2)} ms`);
  console.log(`| ${name} | ${cells.join(" | ")} |`);
}
