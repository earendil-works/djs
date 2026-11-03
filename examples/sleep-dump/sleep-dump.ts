// A host tool that sleeps on the host side, and a script that waits on it. While the call
// is open the host dumps the VM, throws it away, and continues the run from the dump in a
// new sandbox (or a new process). See README.md.
//
//   node examples/sleep-dump/sleep-dump.ts [--ms 5000]              in one process
//   node examples/sleep-dump/sleep-dump.ts --restart [--ms 5000]    dump, exit, resume in a new process
//   node examples/sleep-dump/sleep-dump.ts start  [--ms N] [--dir D]   (the two halves of --restart)
//   node examples/sleep-dump/sleep-dump.ts resume [--dir D]

import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type ConsoleLike, type OpenCall, type Run, Sandbox } from "../../js/src/sandbox.ts";

/** The script: a codemode body, `ms` its parameter. */
export const SCRIPT = `
console.log("before");
const t0 = Date.now();
const r = await sleep(ms);
console.log("after", r, "(the script waited", Date.now() - t0 >= ms ? "at least" : "less than", ms, "ms)");
return r;
`;

/**
 * The host's own state: when each open sleep is due. The VM knows the call (its id,
 * `sleep`, its arguments) and the dump keeps it; the timer is the host's, so the host
 * keeps its deadline and re-arms the timer from it after a restart.
 */
export type HostState = Record<number, number>;

/** The tool. The sandbox only needs its name: a host-driven run never calls it itself. */
const sleep = (ms: unknown) => new Promise<string>((done) => setTimeout(() => done(`slept ${String(ms)} ms`), Number(ms)));

export interface Options {
  ms: number;
  /** The sandbox's console and the host's own messages; default: the process's console. */
  log?: (line: string) => void;
}

function sandboxOptions(log: (line: string) => void) {
  const say = (...a: unknown[]) => log(`  [vm] ${a.map(String).join(" ")}`);
  const console: ConsoleLike = { log: say, info: say, warn: say, error: say };
  return { asyncFunctions: { sleep }, console };
}

/** Starts the host's timer for every open call it has not started yet. */
function arm(run: Run, due: HostState, timers: Map<number, NodeJS.Timeout>, wake: { now(): void }, log: (line: string) => void): void {
  for (const call of run.calls) {
    if (timers.has(call.id)) continue;
    const ms = Number(call.args[0]);
    // A new call is due `ms` from now; one that survived a restart when it was due then.
    due[call.id] ??= Date.now() + ms;
    const left = Math.max(0, due[call.id] - Date.now());
    log(`host: call #${call.id} ${describe(call)}: timer armed, ${left} ms left`);
    timers.set(
      call.id,
      setTimeout(() => {
        delete due[call.id];
        run.settle(call.id, { ok: true, value: `slept ${ms} ms` });
        wake.now();
      }, left),
    );
  }
}

/** Runs `run` to its end: arms timers for its calls and advances when results came in. */
async function finish(run: Run, due: HostState, log: (line: string) => void): Promise<unknown> {
  const timers = new Map<number, NodeJS.Timeout>();
  let settled = 0;
  let woken = () => {};
  const wake = { now: () => (settled++, woken()) };
  while (run.state === "idle") {
    arm(run, due, timers, wake, log);
    if (settled === 0) await new Promise<void>((w) => (woken = w));
    settled = 0;
    await run.advance();
  }
  if (run.state === "threw") throw run.error;
  return run.value;
}

function describe(c: OpenCall): string {
  return `${c.fn}(${c.args.map((a) => JSON.stringify(a)).join(", ")})`;
}

/** The first half: begin the run, arm the timer, dump while the call is open. */
async function start(opts: Options): Promise<{ dump: string; due: HostState }> {
  const log = opts.log ?? console.log;
  const sb = await Sandbox.create(sandboxOptions(log));
  const run = sb.begin(SCRIPT, { ms: opts.ms });
  const due: HostState = {};
  const timers = new Map<number, NodeJS.Timeout>();
  arm(run, due, timers, { now: () => {} }, log);
  log(`host: the VM is idle, waiting on ${run.calls.map(describe).join(", ")}; dumping it`);
  const dump = run.dump();
  // The restart: the timers and the sandbox go away. Only the dump and `due` survive.
  for (const t of timers.values()) clearTimeout(t);
  sb.dispose();
  return { dump, due };
}

/** The second half: a new sandbox loads the dump, re-arms the timers, finishes. */
async function resume(dump: string, due: HostState, opts: Omit<Options, "ms">): Promise<unknown> {
  const log = opts.log ?? console.log;
  const { sandbox, run } = await Sandbox.resume(dump, sandboxOptions(log));
  log(`host: loaded the dump into a new sandbox; open calls: ${run.calls.map((c) => `#${c.id} ${describe(c)}`).join(", ")}`);
  try {
    const r = await finish(run, due, log);
    log(`host: the run returned ${JSON.stringify(r)}`);
    return r;
  } finally {
    sandbox.dispose();
  }
}

/** In one process: start, dump, drop everything, resume from the dump text. */
export async function inProcess(opts: Options): Promise<{ dump: string; result: unknown }> {
  const log = opts.log ?? console.log;
  const { dump, due } = await start(opts);
  log(`host: the dump (${dump.length} bytes):\n${dump.replace(/^/gm, "  | ")}`);
  const result = await resume(dump, due, opts);
  return { dump, result };
}

function arg(name: string, fallback: string): string {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : fallback;
}

async function main(): Promise<void> {
  const ms = Number(arg("--ms", "5000"));
  const dir = arg("--dir", join(fileURLToPath(new URL(".", import.meta.url)), "out"));
  const mode = process.argv[2];
  if (mode === "start") {
    const { dump, due } = await start({ ms });
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "sleep.dump.js"), dump);
    writeFileSync(join(dir, "host.json"), JSON.stringify(due));
    console.log(`host: wrote ${join(dir, "sleep.dump.js")} (${dump.length} bytes) and host.json; exiting`);
  } else if (mode === "resume") {
    const dump = readFileSync(join(dir, "sleep.dump.js"), "utf8");
    const due = JSON.parse(readFileSync(join(dir, "host.json"), "utf8")) as HostState;
    await resume(dump, due, {});
  } else if (process.argv.includes("--restart")) {
    const self = fileURLToPath(import.meta.url);
    for (const half of ["start", "resume"]) {
      console.log(`--- node sleep-dump.ts ${half} (process ${half === "start" ? 1 : 2})`);
      const r = spawnSync(process.execPath, [self, half, "--ms", String(ms), "--dir", dir], { stdio: "inherit" });
      if (r.status !== 0) process.exit(r.status ?? 1);
    }
  } else {
    await inProcess({ ms });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
