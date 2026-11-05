// The sleep-dump example (examples/sleep-dump, 0179, rewritten on tasks in 0189, on plain
// dumps in 0204) in one process with a short sleep: the run is suspended into a dump while
// the tool sleeps, the sandbox dropped, and the run continued from the dump in a new
// sandbox, where the tool's `resume` hook sleeps the rest of the call's deadline; and the
// same dump continued in node itself, a plain Script given the host's functions; and the
// command line's two halves in two processes.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { options, resume, resumeInNode, suspend } from "../../examples/sleep-dump/sleep-dump.ts";

test("examples/sleep-dump: suspend while the tool sleeps, resume, continue", async () => {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const t0 = Date.now();
  const dump = await suspend(50, options(log));
  // A plain dump: the open call with its deadline, both activations as continuations.
  assert.match(dump, /^"djs-dump";\n/);
  const m = /\$djs\.resume\(\d+, "tools\.sleep", \[50\], \{"due":(\d+)\}\);/.exec(dump);
  assert.ok(m, dump);
  assert.ok(Number(m[1]) >= t0 + 50 && Number(m[1]) <= Date.now() + 50, dump);
  assert.equal(dump.match(/\$djs\.k\(async function/g)?.length, 2, dump);
  assert.deepEqual(lines, ["first sleeps 50 ms"]);
  const result = await resume(dump, options(log));
  assert.deepEqual(result, ["first: slept 50 ms", "second: slept 50 ms"]);
  assert.deepEqual(lines, ["first sleeps 50 ms", "second sleeps 50 ms", "done after at least 100 ms: first: slept 50 ms, second: slept 50 ms"]);
  assert.ok(Date.now() - t0 >= 100);
});

test("examples/sleep-dump: the dump continues in node", async () => {
  const dump = await suspend(20, options(() => {}));
  const logged: unknown[][] = [];
  const log = console.log;
  console.log = (...a: unknown[]) => void logged.push(a);
  try {
    assert.deepEqual(await resumeInNode(dump), ["first: slept 20 ms", "second: slept 20 ms"]);
  } finally {
    console.log = log;
  }
  assert.deepEqual(logged.map((a) => a.join(" ")), ["second sleeps 20 ms", "done after at least 40 ms: first: slept 20 ms, second: slept 20 ms"]);
});

test("examples/sleep-dump: two processes, the second resuming the dump the first wrote", () => {
  // As `run.sh --restart`: process 2 has the script only through the `sources` option.
  const run = (...args: string[]) =>
    execFileSync(process.execPath, [fileURLToPath(new URL("../../examples/sleep-dump/sleep-dump.ts", import.meta.url)), ...args], {
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
    "host: suspended the run into examples/sleep-dump/out/sleep.dump.js (N bytes) and dropped the sandbox:",
  ]);
  assert.deepEqual(run("resume"), [
    "host: resuming examples/sleep-dump/out/sleep.dump.js in a new sandbox",
    "  [vm] second sleeps 30 ms",
    "  [vm] done after at least 60 ms: first: slept 30 ms, second: slept 30 ms",
    "host: the run returned [ 'first: slept 30 ms', 'second: slept 30 ms' ]",
  ]);
});
