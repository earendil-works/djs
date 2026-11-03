// The sleep-dump example (examples/sleep-dump, 0179) in one process with a short sleep:
// a run waiting on a host tool is dumped, the sandbox dropped, and the run continued
// from the dump in a new sandbox, which re-arms the host's timer.

import { test } from "node:test";
import assert from "node:assert/strict";
import { inProcess } from "../../examples/sleep-dump/sleep-dump.ts";

test("examples/sleep-dump: dump while the tool sleeps, resume, continue", async () => {
  const lines: string[] = [];
  const { dump, result } = await inProcess({ ms: 50, log: (l) => void lines.push(l) });
  assert.deepEqual(result, ["first: slept 50 ms", "second: slept 50 ms"]);
  assert.match(dump, /^\/\/ djs-dump 2/);
  // What was suspended: the open call, both activations, the half-done push, the loop.
  assert.match(dump, /\$dump\.pending\(\d+, "sleep", \[50\]\)/);
  assert.equal(dump.match(/\$dump\.resume\(/g)?.length, 2, dump);
  assert.match(dump, /"call\.this"/);
  assert.match(dump, /for-of/);
  assert.match(dump, /\$dump\.handle\("run"/);
  const vm = lines.filter((l) => l.startsWith("  [vm]"));
  assert.deepEqual(vm, [
    "  [vm] first sleeps 50 ms",
    "  [vm] second sleeps 50 ms",
    "  [vm] done after at least 100 ms: first: slept 50 ms, second: slept 50 ms",
  ]);
  // The first sandbox printed only the first line; the rest came from the second.
  const loaded = lines.findIndex((l) => l.startsWith("host: loaded the dump"));
  assert.ok(loaded > lines.indexOf(vm[0]) && lines.indexOf(vm[1]) > loaded, lines.join("\n"));
});
