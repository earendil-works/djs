// The sleep-dump example (examples/sleep-dump, 0179, rewritten on tasks in 0189, on plain
// dumps in 0204) in one process with a short sleep: the run is suspended into a dump while
// the tool sleeps, the sandbox dropped, and the run continued from the dump in a new
// sandbox, where the tool's `resume` hook sleeps the rest of the call's deadline; and the
// same dump continued in node itself, a plain Script given the host's functions.

import { test } from "node:test";
import assert from "node:assert/strict";
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
  assert.equal(dump.match(/^\$dump\.K\w+ = async function/gm)?.length, 2, dump);
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
