// The sleep-snapshot example (examples/sleep-snapshot) in one process with a short sleep:
// the run is suspended into a snapshot while the tool sleeps, the sandbox dropped, and the
// run continued from the snapshot in a new sandbox, where the tool's `resume` hook sleeps
// the rest of the call's deadline; and the command line's two halves in two processes.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { options, resume, suspend } from "../../examples/sleep-snapshot/sleep-snapshot.ts";

test("examples/sleep-snapshot: suspend while the tool sleeps, resume, continue", async () => {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const t0 = Date.now();
  const { snapshot, described } = await suspend(50, options(log));
  // The open call with its deadline, the two waiting functions by their bytecode, the
  // script by its hash, the host's values by name.
  const m = /host data: \{"states":\[\[2,\{"due":(\d+)\}\]\],"fuelUsed":\d+\}/.exec(described);
  assert.ok(m, described);
  assert.ok(Number(m[1]) >= t0 + 50 && Number(m[1]) <= Date.now() + 50, described);
  assert.equal(described.match(/^function .* \(bytecode\)$/gm)?.length, 2, described);
  assert.match(described, /^script 0: sleep\.js, parameters ms \(source by hash [0-9a-f]{16}\)$/m);
  assert.match(described, /^host values: console tools tools\.sleep\(\)/m);
  assert.deepEqual(lines, ["first sleeps 50 ms"]);
  const result = await resume(snapshot, options(log));
  assert.deepEqual(result, ["first: slept 50 ms", "second: slept 50 ms"]);
  assert.deepEqual(lines, ["first sleeps 50 ms", "second sleeps 50 ms", "done after at least 100 ms: first: slept 50 ms, second: slept 50 ms"]);
  assert.ok(Date.now() - t0 >= 100);
});

test("examples/sleep-snapshot: two processes, the second resuming the snapshot the first wrote", () => {
  // As `run.sh --restart`: process 2 has the script only through the `sources` option.
  const run = (...args: string[]) =>
    execFileSync(process.execPath, [fileURLToPath(new URL("../../examples/sleep-snapshot/sleep-snapshot.ts", import.meta.url)), ...args], {
      cwd: fileURLToPath(new URL("../..", import.meta.url)),
      encoding: "utf8",
      timeout: 30_000,
    })
      .split("\n")
      .filter((l) => l !== "" && !l.startsWith("  | "))
      .map((l) => l.replace(/\(\d+ bytes\)/, "(N bytes)"));
  assert.deepEqual(run("start", "30"), [
    "host: starting sleep.js with ms = 30",
    "  [vm] first sleeps 30 ms",
    "host: suspended the run into examples/sleep-snapshot/out/sleep.snapshot (N bytes) and dropped the sandbox; it holds:",
  ]);
  assert.deepEqual(run("resume"), [
    "host: resuming examples/sleep-snapshot/out/sleep.snapshot in a new sandbox",
    "  [vm] second sleeps 30 ms",
    "  [vm] done after at least 60 ms: first: slept 30 ms, second: slept 30 ms",
    "host: the run returned [ 'first: slept 30 ms', 'second: slept 30 ms' ]",
  ]);
});
