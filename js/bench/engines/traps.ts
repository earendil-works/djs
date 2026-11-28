// The runaway-script test of docs/guide/benchmarks.md: hostile or runaway scripts (endless
// loops, huge allocations, deep recursion, catastrophic regular expressions) on every
// engine of the page, each under its own documented limits (`Engine.limited`), the same
// budget for all where an engine has the mechanism: LIMITS below.
//
//   node js/bench/engines/traps.ts [--engines a,b] [--cases x,y] [--json FILE]
//
// Every engine x case runs in a child process of its own (`--child engine case`), with an
// outer kill after KILL_MS and a memory guard on the child's resident size, so a crash, an
// out-of-memory death or a hang of one engine cannot take the harness down. The child
// reports what happened to the run, its time, its memory high-water (the instance's linear
// memory; the process's peak RSS growth too, the only measure for psychoscript, which runs
// in the host's own heap), and whether a fresh VM in the same process still runs a script.
// Not timed like run.ts: a case is one run, and its time is how long the engine took to
// stop it.

import { spawn } from "node:child_process";
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { cpus, loadavg, platform, release } from "node:os";
import { fileURLToPath } from "node:url";
import { allJs, type Engine, type Limits, monty, psychoscript } from "./engines.ts";

export const LIMITS: Limits = { ms: 1000, memory: 64 << 20 };
/** The outer kill: a run that has not ended by then hangs its host. */
const KILL_MS = 10_000;
/** The memory guard: a child whose resident size passes it is killed. */
const GUARD_BYTES = 4 * 2 ** 30;

/** The cases: a JavaScript body and its Python translation (for monty). */
export const CASES: { name: string; what: string; js: string; py: string }[] = [
  { name: "loop", what: "`for (;;) {}`", js: "for (;;) {}", py: "while True:\n    pass" },
  {
    name: "microtasks",
    what: "a promise job that queues itself again forever, while the run awaits a promise that never settles",
    js: "const spin = () => {\n  Promise.resolve().then(spin);\n};\nspin();\nawait new Promise(() => {});",
    py: "import asyncio\nasync def spin():\n    while True:\n        await asyncio.sleep(0)\nawait spin()",
  },
  {
    name: "grow",
    what: "an array of small objects grown until memory runs out",
    js: 'const a = [];\nfor (let i = 0; ; i++) a.push({ i, s: "item " + i });',
    py: 'a = []\ni = 0\nwhile True:\n    a.append({"i": i, "s": "item " + str(i)})\n    i += 1',
  },
  { name: "huge", what: 'one huge allocation: `"x".repeat(2 ** 30)`, a GiB string', js: 'const s = "x".repeat(2 ** 30);', py: 's = "x" * 2 ** 30' },
  { name: "double", what: "a string that doubles until it fails", js: 'let s = "x";\nfor (;;) s += s;', py: 's = "x"\nwhile True:\n    s = s + s' },
  { name: "recursion", what: "unbounded recursion", js: "const f = (n) => f(n + 1) + 1;\nf(0);", py: "def f(n):\n    return f(n + 1) + 1\nf(0)" },
  {
    name: "json-parse",
    what: "`JSON.parse` of arrays nested a million deep",
    js: 'JSON.parse("[".repeat(1e6) + "]".repeat(1e6));',
    py: 'import json\njson.loads("[" * 1000000 + "]" * 1000000)',
  },
  {
    name: "json-stringify",
    what: "`JSON.stringify` of objects nested 100,000 deep",
    js: "let d = {};\nfor (let i = 0; i < 1e5; i++) d = { d };\nJSON.stringify(d);",
    py: 'import json\nd = {}\nfor i in range(100000):\n    d = {"d": d}\njson.dumps(d)',
  },
  {
    name: "regex",
    what: "catastrophic backtracking: `/(a+)+$/` on 40 `a`s and a `!`",
    js: '/(a+)+$/.test("a".repeat(40) + "!");',
    py: 'import re\nre.search(r"(a+)+$", "a" * 40 + "!")',
  },
];

/** A case body inside a `try`, so that a catchable error shows as `caught ...`. */
function wrap(e: Engine, body: string): string {
  if (e.language === "python") {
    const ind = body.replace(/^/gm, "    ");
    return `__r = "completed"\ntry:\n${ind}\nexcept Exception as e:\n    __r = "caught " + type(e).__name__ + ": " + str(e)[:200]\n__r`;
  }
  // Out of memory can leave no room for the message (Bellard's QuickJS then throws null and
  // the concatenation fails again): the literal needs none.
  return `try {\n${body}\nreturn "completed";\n} catch (e) {\ntry {\nreturn "caught " + String(e).slice(0, 200);\n} catch {\nreturn "caught (no memory left for its message)";\n}\n}`;
}

const engines = (): Engine[] => [...allJs(), monty(), psychoscript()];
function slug(name: string): string {
  return name.replace(/[^a-z0-9]+/gi, "-").replace(/-$/, "").toLowerCase();
}
const maxRss = () => process.resourceUsage().maxRSS * 1024;

// ---- The child: one engine, one case -------------------------------------------------------

async function child(engineName: string, caseName: string) {
  const e = engines().find((x) => x.name === engineName)!;
  const c = CASES.find((x) => x.name === caseName)!;
  const say = (v: unknown) => process.stdout.write(`${JSON.stringify(v)}\n`);
  await e.load();
  const program = e.trapProgram!(wrap(e, e.language === "python" ? c.py : c.js));
  const vm = await e.limited!(LIMITS);
  const rss0 = maxRss();
  say({ phase: "start" });
  const t = performance.now();
  const out = await vm.run(program);
  const ms = performance.now() - t;
  const memory = vm.memory();
  say({ phase: "end", out, ms, memory, rss: maxRss() - rss0 });
  vm.dispose();
  // The same process, a fresh VM: does the engine still work?
  let fresh: string;
  try {
    const vm2 = await e.limited!(LIMITS);
    const r = await vm2.run(e.trapProgram!(e.language === "python" ? "str(1 + 1)" : "return String(1 + 1);"));
    vm2.dispose();
    fresh = r.ok && r.value === "2" ? "ok" : `fails: ${r.ok ? r.value : r.error}`;
  } catch (err) {
    fresh = `fails: ${err}`;
  }
  say({ phase: "fresh", fresh });
  process.exit(0);
}

// ---- The parent ----------------------------------------------------------------------------

export interface TrapResult {
  /**
   * completed: the script ran to its end; caught: it caught an error; stopped: the engine
   * ended the run with an error the script could not catch; unsettled: the run ended
   * without a result (its promise never settled, as when out of memory ends a promise
   * chain); trap: a wasm trap ended the run; crash: the host process died; hang: the
   * outer kill; guard: the memory guard's kill.
   */
  outcome: string;
  /** The error's class (`RangeError`, `InternalError`, `RecursionError`; `null` for a thrown null), if it has one. */
  error: string;
  /** What ran out, from the message: time, memory, stack, length (a string's), depth (a nesting cap), or "". */
  cause: string;
  /** The engine's message, for the notes. */
  detail: string;
  /** Until the run ended (or the process died or was killed). */
  ms: number;
  /** Linear memory after the run (null: none, or the process died). */
  memory: number | null;
  /** Peak RSS growth of the process during the run. */
  rss: number | null;
  /** Whether a fresh VM in the same process ran a script afterwards. */
  fresh: boolean;
}

/** A message's error class and what ran out (`TrapResult.error`, `cause`). */
function classify(msg: string): { error: string; cause: string } {
  const m = msg.replace(/^(caught |ERROR |wasm trap: |aborted: |fuel: |memory: |error: |fatal-error: )+/, "");
  const causes: [RegExp, string][] = [
    [/interrupted|aborted|timed out|time limit|timeout/i, "time"],
    [/out of memory|MemoryError|memory limit/i, "memory"],
    [/maximum depth/i, "depth"],
    [/stack|recursion/i, "stack"],
    [/string length|too long/i, "length"],
  ];
  const error = m === "null" ? "null" : (/^([A-Za-z]*(Error|Exceeded))\b/.exec(m)?.[1] ?? "");
  return { error, cause: causes.find(([re]) => re.test(m))?.[1] ?? "" };
}

function runChild(e: Engine, c: string): Promise<TrapResult> {
  return new Promise((resolve) => {
    const p = spawn(process.execPath, [fileURLToPath(import.meta.url), "--child", e.name, c], { stdio: ["ignore", "pipe", "pipe"] });
    let buf = "";
    let err = "";
    let started = 0;
    let end: { out: { ok: boolean; value?: string; error?: string }; ms: number; memory: number | null; rss: number } | undefined;
    let fresh: string | undefined;
    let killed: "hang" | "guard" | undefined;
    let peak = 0;
    const killer = setTimeout(() => {
      killed = "hang";
      p.kill("SIGKILL");
    }, KILL_MS + 30_000);
    const guard = setInterval(() => {
      try {
        const rss = Number(execFileSync("ps", ["-o", "rss=", "-p", String(p.pid)], { encoding: "utf8" }).trim()) * 1024;
        peak = Math.max(peak, rss);
        if (rss > GUARD_BYTES) {
          killed = "guard";
          p.kill("SIGKILL");
        }
      } catch {}
      if (started && !end && performance.now() - started > KILL_MS) {
        killed = "hang";
        p.kill("SIGKILL");
      }
    }, 100);
    p.stdout.on("data", (d) => {
      buf += d;
      for (let i; (i = buf.indexOf("\n")) >= 0; ) {
        const m = JSON.parse(buf.slice(0, i));
        buf = buf.slice(i + 1);
        if (m.phase === "start") started = performance.now();
        if (m.phase === "end") end = m;
        if (m.phase === "fresh") fresh = m.fresh;
      }
    });
    p.stderr.on("data", (d) => {
      err = (err + d).slice(-4000);
    });
    p.on("exit", (code, signal) => {
      clearTimeout(killer);
      clearInterval(guard);
      const elapsed = started ? performance.now() - started : 0;
      if (end) {
        const o = end.out;
        const msg = o.ok ? o.value! : o.error!;
        // A rejection that reached the host's handler (`ERROR ...`) escaped the script's catch.
        const outcome = o.ok
          ? msg.startsWith("caught ")
            ? "caught"
            : msg === "completed"
              ? "completed"
              : msg === "undefined"
                ? "unsettled"
                : "stopped"
          : /wasm trap|RuntimeError|unreachable|out of bounds/i.test(msg)
            ? "trap"
            : "stopped";
        resolve({ outcome, ...(outcome === "completed" || outcome === "unsettled" ? { error: "", cause: "" } : classify(msg)), detail: msg.slice(0, 300), ms: end.ms, memory: end.memory, rss: end.rss, fresh: fresh === "ok" });
        return;
      }
      const tail = err.trim().split("\n").filter((l) => !/ExperimentalWarning|--trace-warnings/.test(l)).slice(-3).join(" | ");
      if (killed) {
        resolve({ outcome: killed, error: "", cause: "", detail: killed === "hang" ? `killed after ${KILL_MS / 1000} s` : `killed at ${(peak / 2 ** 30).toFixed(1)} GiB resident`, ms: elapsed, memory: null, rss: peak, fresh: false });
        return;
      }
      resolve({ outcome: "crash", error: "", cause: signal ?? `exit ${code}`, detail: tail.slice(0, 300), ms: elapsed, memory: null, rss: peak, fresh: false });
    });
  });
}

async function main() {
  const argv = process.argv.slice(2);
  const opt = (name: string) => {
    const i = argv.indexOf(name);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  if (argv[0] === "--child") return child(argv[1], argv[2]);
  let es = engines();
  if (opt("--engines")) {
    const want = opt("--engines")!.split(",");
    es = es.filter((e) => want.includes(e.name) || want.includes(slug(e.name)));
  }
  const cases = opt("--cases")?.split(",") ?? CASES.map((c) => c.name);
  const startLoad = loadavg()[0];
  console.log(`${cpus()[0].model}, ${cpus().length} cores, ${platform()} ${release()}, node ${process.version}; load ${loadavg().map((x) => x.toFixed(1)).join(" ")}`);
  console.log(`limits: ${LIMITS.ms} ms, ${LIMITS.memory >> 20} MiB; outer kill ${KILL_MS / 1000} s, memory guard ${GUARD_BYTES / 2 ** 30} GiB resident`);
  const results: { name: string; limits: string; cases: Record<string, TrapResult> }[] = [];
  for (const e of es) {
    const r = { name: e.name, limits: e.limitsUsed ?? "none", cases: {} as Record<string, TrapResult> };
    for (const c of cases) {
      const x = await runChild(e, c);
      r.cases[c] = x;
      const mem = x.memory === null ? "" : ` ${(x.memory / 2 ** 20).toFixed(1)} MiB`;
      console.log(`${e.name.padEnd(24)} ${c.padEnd(15)} ${x.outcome.padEnd(9)} ${(x.ms / 1000).toFixed(2).padStart(6)} s${mem}, rss +${((x.rss ?? 0) / 2 ** 20).toFixed(0)} MiB${x.fresh ? "" : ", no fresh VM"}  ${x.error} ${x.cause}  ${x.detail.slice(0, 120).replace(/\n/g, " ")}`);
    }
    results.push(r);
  }
  const json = opt("--json");
  if (json) writeFileSync(json, JSON.stringify({ load: startLoad, limits: LIMITS, killMs: KILL_MS, guard: GUARD_BYTES, cases: CASES.map(({ name, what }) => ({ name, what })), engines: results }, null, 1));
}

await main();
