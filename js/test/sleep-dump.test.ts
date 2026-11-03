// The sleep-dump example (examples/sleep-dump, 0179) in one process with a short sleep:
// a run waiting on a host tool is dumped, the sandbox dropped, and the run continued
// from the dump in a new sandbox, which re-arms the host's timer.

import { test } from "node:test";
import assert from "node:assert/strict";
import { inProcess } from "../../examples/sleep-dump/sleep-dump.ts";

test("examples/sleep-dump: dump while the tool sleeps, resume, continue", async () => {
  const lines: string[] = [];
  const { dump, result } = await inProcess({ ms: 50, log: (l) => void lines.push(l) });
  assert.equal(result, "slept 50 ms");
  assert.match(dump, /^\/\/ djs-dump 2/);
  assert.match(dump, /\$dump\.pending\(\d+, "sleep", \[50\]\)/);
  assert.match(dump, /\$dump\.handle\("run"/);
  const vm = lines.filter((l) => l.startsWith("  [vm]"));
  assert.deepEqual(vm, ["  [vm] before", "  [vm] after slept 50 ms (the script waited at least 50 ms)"]);
  // The "after" line comes from the second sandbox.
  const loaded = lines.findIndex((l) => l.startsWith("host: loaded the dump"));
  assert.ok(loaded > 0 && lines.indexOf(vm[1]) > loaded, lines.join("\n"));
});
