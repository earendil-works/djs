// A host tool that sleeps on the host side, and a script that waits on it. While the sleep
// is open the host suspends the run into a dump and continues it from the dump in a new
// sandbox, here or in another process. See README.md.
//
//   node examples/sleep-dump/sleep-dump.ts [both|start|resume] [ms]

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [mode = "both", ms = "5000"] = process.argv.slice(2);
  const file = fileURLToPath(new URL("out/sleep.dump.js", import.meta.url));
  if (mode !== "resume") {
    const dump = await suspend(Number(ms));
    mkdirSync(new URL("out", import.meta.url), { recursive: true });
    writeFileSync(file, dump);
    console.log(`host: suspended the run into ${file} (${dump.length} bytes)`);
  }
  if (mode !== "start") console.log("host: the run returned", await resume(readFileSync(file, "utf8")));
}
