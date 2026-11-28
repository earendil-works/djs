// A host tool that sleeps on the host side, and a script that waits on it. While the sleep
// is open the host snapshots the run and continues it from the snapshot in a new sandbox,
// here or in another process. See README.md.
//
//   node examples/sleep-snapshot/sleep-snapshot.ts [both|start|resume] [ms]

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { relative } from "node:path";
import { fileURLToPath } from "node:url";
import { Sandbox, type SandboxOptions } from "../../js/src/sandbox.ts";

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

/** Resolves at `due` (ms since the epoch); a passed deadline at once. */
function until(due: number): Promise<void> {
  return new Promise((done) => setTimeout(done, Math.max(0, due - Date.now())));
}

/** The sandbox: `tools.sleep` is an async host function the host settles itself. */
export const options = (log = (...a: unknown[]) => console.log("  [vm]", ...a)): SandboxOptions => ({
  asyncFunctions: { "tools.sleep": () => {} },
  console: { log, info: log, warn: log, error: log },
});

/** A sleep's deadline, the host's state of the call, which a snapshot keeps. */
type Sleep = { due: number };

/** Starts the script and snapshots it at its first idle point: the first sleep is open. */
export async function suspend(ms: number, opts = options()): Promise<{ snapshot: Uint8Array; described: string }> {
  const sandbox = await Sandbox.create(opts);
  const run = sandbox.begin(SCRIPT, { ms }, { filename: "sleep.js" });
  const states: Record<number, Sleep> = {};
  for (const call of run.calls) states[call.id] = { due: Date.now() + Number(call.args[0]) };
  const snapshot = run.snapshot({ states });
  const described = sandbox.describeSnapshot(snapshot);
  sandbox.dispose();
  return { snapshot, described };
}

/**
 * Continues the run from its snapshot in a new sandbox, to its end: settles each sleep at
 * its deadline (a loaded call's from its state, a new call's from its argument).
 */
export async function resume(snapshot: Uint8Array, opts = options()): Promise<unknown> {
  const sandbox = await Sandbox.create(opts);
  try {
    const run = sandbox.loadSnapshot(snapshot);
    while (run.state === "idle" && run.calls.length > 0) {
      const [call] = run.calls;
      await until((call.state as Sleep | undefined)?.due ?? Date.now() + Number(call.args[0]));
      run.settle(call.id, { ok: true, value: `slept ${call.args[0]} ms` });
      await run.advance();
    }
    if (run.state === "threw") throw run.error;
    return run.value;
  } finally {
    sandbox.dispose();
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [mode = "both", ms = "1000"] = process.argv.slice(2);
  const file = fileURLToPath(new URL("out/sleep.snapshot", import.meta.url));
  if (mode !== "resume") {
    console.log(`host: starting sleep.js with ms = ${ms}`);
    const { snapshot, described } = await suspend(Number(ms));
    mkdirSync(new URL("out", import.meta.url), { recursive: true });
    writeFileSync(file, snapshot);
    console.log(`host: snapshotted the run into ${relative(".", file)} (${snapshot.length} bytes) and dropped the sandbox; it holds:`);
    console.log(described.trimEnd().replace(/^/gm, "  | "));
  }
  if (mode !== "start") {
    console.log(`host: resuming ${relative(".", file)} in a new sandbox`);
    console.log("host: the run returned", await resume(new Uint8Array(readFileSync(file))));
  }
}
