// A host tool that sleeps on the host side, and a script that waits on it. While the sleep
// is open the host suspends the run into a dump and continues it from the dump in a new
// sandbox, here or in another process. See README.md.
//
//   node examples/sleep-dump/sleep-dump.ts [both|start|resume|node] [ms]
//
// `node` continues out/sleep.dump.js in node itself, not in djs: a plain dump is a Script
// that needs only the host's functions.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { Script } from "node:vm";
import { relative } from "node:path";
import { fileURLToPath } from "node:url";
import { type AsyncHostFunction, Sandbox, type SandboxOptions } from "../../js/src/sandbox.ts";

export const SCRIPT = `
async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
}
const t0 = Date.now();
const results = [];
for (const label of ["first", "second"]) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;
`;

/** Resolves at `due` (ms since the epoch), unless `signal` aborts first. */
function until(due: number, signal: AbortSignal): Promise<void> {
  return new Promise((done) => {
    const timer = setTimeout(done, due - Date.now());
    signal.addEventListener("abort", () => clearTimeout(timer));
  });
}

/** `tools.sleep(ms)`: the call's state is its deadline, so a resumed call sleeps only the rest. */
const sleep = {
  async call(ms) {
    const due = Date.now() + Number(ms);
    this.state = { due };
    await until(due, this.signal);
    return `slept ${ms} ms`;
  },
  async resume({ due }, call) {
    await until(due, call.signal);
    return `slept ${call.args[0]} ms`;
  },
} satisfies AsyncHostFunction<{ due: number }>;

export const options = (log = (...a: unknown[]) => console.log("  [vm]", ...a)): SandboxOptions => ({
  asyncFunctions: { "tools.sleep": sleep },
  // Plain dumps (D2830): the dump is a Script that runs given only the host's functions.
  plainDumps: true,
  console: { log, info: log, warn: log, error: log },
});

/** Starts the script and suspends it at its first idle point: the first sleep is open. */
export async function suspend(ms: number, opts = options()): Promise<string> {
  const sandbox = await Sandbox.create(opts);
  const dump = await sandbox.start(SCRIPT, { ms }, { filename: "sleep.js" }).suspend();
  sandbox.dispose();
  return dump;
}

/** Continues the run from its dump in a new sandbox, to its end. */
export async function resume(dump: string, opts = options()): Promise<unknown> {
  const task = await Sandbox.resume(dump, opts);
  return task.result.finally(() => task.sandbox.dispose());
}

/**
 * Continues the dump in node: the same host functions, as plain globals. `$djs.resume`
 * runs the open call's `resume` hook with its saved state; `$djs.seed` has nothing to set.
 */
export async function resumeInNode(dump: string): Promise<unknown> {
  const signal = new AbortController().signal;
  const g = globalThis as Record<string, unknown>;
  g.$djs = {
    seed() {},
    resume: (_id: number, _name: string, args: unknown[], state: { due: number }) => sleep.resume(state, { args, signal } as never),
  };
  g.tools = { sleep: (ms: number) => sleep.call.call({ signal } as never, ms) };
  return new Script(dump, { filename: "sleep.dump.js" }).runInThisContext();
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [mode = "both", ms = "5000"] = process.argv.slice(2);
  const file = fileURLToPath(new URL("out/sleep.dump.js", import.meta.url));
  if (mode !== "resume" && mode !== "node") {
    const dump = await suspend(Number(ms));
    mkdirSync(new URL("out", import.meta.url), { recursive: true });
    writeFileSync(file, dump);
    console.log(`host: suspended the run into ${relative(".", file)} (${dump.length} bytes):`);
    console.log(dump.trimEnd().replace(/^/gm, "  | "));
  }
  if (mode === "node") console.log("node: the run returned", await resumeInNode(readFileSync(file, "utf8")));
  else if (mode !== "start") console.log("host: the run returned", await resume(readFileSync(file, "utf8")));
}
