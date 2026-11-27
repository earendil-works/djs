// A host tool that sleeps on the host side, and a script that waits on it. While the sleep
// is open the host suspends the run into a snapshot and continues it from the snapshot in
// a new sandbox, here or in another process. See README.md.
//
//   node examples/sleep-snapshot/sleep-snapshot.ts [both|start|resume] [ms]

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
    const timer = setTimeout(done, Math.max(0, due - Date.now())); // a passed deadline: at once
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
  // The snapshot names the script it continues by its name and hash (`sources: false`
  // below); `sources` gives it back to a process that did not run it (--restart's second).
  sources: (name) => (name === "sleep.js" ? SCRIPT : undefined),
  console: { log, info: log, warn: log, error: log },
});

/** Starts the script and suspends it at its first idle point: the first sleep is open. */
export async function suspend(ms: number, opts = options()): Promise<{ snapshot: Uint8Array; described: string }> {
  const sandbox = await Sandbox.create(opts);
  const snapshot = await sandbox.start(SCRIPT, { ms }, { filename: "sleep.js" }).suspend({ sources: false });
  const described = sandbox.describeSnapshot(snapshot);
  sandbox.dispose();
  return { snapshot, described };
}

/** Continues the run from its snapshot in a new sandbox, to its end. */
export async function resume(snapshot: Uint8Array, opts = options()): Promise<unknown> {
  const task = await Sandbox.resume(snapshot, opts);
  return task.result.finally(() => task.sandbox.dispose());
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [mode = "both", ms = "1000"] = process.argv.slice(2);
  const file = fileURLToPath(new URL("out/sleep.snapshot", import.meta.url));
  if (mode !== "resume") {
    console.log(`host: starting sleep.js with ms = ${ms}`);
    const { snapshot, described } = await suspend(Number(ms));
    mkdirSync(new URL("out", import.meta.url), { recursive: true });
    writeFileSync(file, snapshot);
    console.log(`host: suspended the run into ${relative(".", file)} (${snapshot.length} bytes) and dropped the sandbox; it holds:`);
    // The summary lines; the record types' fields follow them.
    console.log(described.split("\n").filter((l) => l !== "" && !l.startsWith("type ")).join("\n").replace(/^/gm, "  | "));
  }
  if (mode !== "start") {
    console.log(`host: resuming ${relative(".", file)} in a new sandbox`);
    console.log("host: the run returned", await resume(new Uint8Array(readFileSync(file))));
  }
}
