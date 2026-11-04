// The sleep-dump example (examples/sleep-dump, 0179, rewritten on tasks in 0189) in one
// process with a short sleep: the run is suspended into a dump while the tool sleeps, the
// sandbox dropped, and the run continued from the dump in a new sandbox, where the
// tool's `resume` hook sleeps the rest of the call's deadline.

import { test } from "node:test";
import assert from "node:assert/strict";
import { options, resume, suspend } from "../../examples/sleep-dump/sleep-dump.ts";

test("examples/sleep-dump: suspend while the tool sleeps, resume, continue", async () => {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const t0 = Date.now();
  const dump = await suspend(50, options(log));
  // The open call with its deadline, both activations, the run's result promise.
  const m = /\$dump\.pending\(\d+, "tools\.sleep", \[50\], \{"due":(\d+)\}\);/.exec(dump);
  assert.ok(m, dump);
  assert.ok(Number(m[1]) >= t0 + 50 && Number(m[1]) <= Date.now() + 50, dump);
  assert.equal(dump.match(/\$dump\.resume\(/g)?.length, 2, dump);
  assert.match(dump, /\$dump\.handle\("run"/);
  assert.deepEqual(lines, ["first sleeps 50 ms"]);
  const result = await resume(dump, options(log));
  assert.deepEqual(result, ["first: slept 50 ms", "second: slept 50 ms"]);
  assert.deepEqual(lines, ["first sleeps 50 ms", "second sleeps 50 ms", "done after at least 100 ms: first: slept 50 ms, second: slept 50 ms"]);
  assert.ok(Date.now() - t0 >= 100);
});
