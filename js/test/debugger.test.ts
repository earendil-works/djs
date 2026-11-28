// `debugger;` as a pause (D3050): with the sandbox option `debugger`, the statement in an
// async function awaits a call of the host function `debugger`, so a host-driven run is
// idle there and can be snapshotted, and a sandbox with the option that loads the
// snapshot pauses again at the next `debugger;`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, type Run, Sandbox, type SandboxOptions } from "../src/sandbox.ts";

function host(module: WebAssembly.Module, extra: Partial<SandboxOptions> = {}) {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const opts: SandboxOptions = { module, frozenBuiltins: true, console: { log, info: log, warn: log, error: log }, debugger: true, ...extra };
  return { opts, lines };
}

const LOOP = `
let sum = 0;
for (let i = 0; i < 3; i++) {
  sum += i;
  console.log("i", i, sum);
  debugger;
}
async function twice(x) {
  debugger;
  return x * 2;
}
return [sum, await twice(sum)];
`;

/** Settles the pause the run waits on and runs to the next one. */
async function goOn(run: Run): Promise<void> {
  assert.deepEqual(run.calls.map((c) => [c.fn, c.args]), [["debugger", []]]);
  run.settle(run.calls[0].id, { ok: true, value: undefined });
  await run.advance();
}

test("a debugger statement pauses a host-driven run; settling the pause goes on", async () => {
  const h = host(await loadModule());
  const sb = new Sandbox(h.opts);
  const run = sb.begin(LOOP);
  for (let n = 0; n < 4; n++) await goOn(run);
  assert.equal(run.state, "done");
  assert.deepEqual(run.value, [3, 6]);
  assert.deepEqual(h.lines, ["i 0 0", "i 1 1", "i 2 3"]);
  sb.dispose();
});

test("a pause is snapshotted as an open call; each loaded snapshot pauses at the next debugger", async () => {
  const h = host(await loadModule());
  let sb = new Sandbox(h.opts);
  let run = sb.begin(LOOP);
  const snapshots: Uint8Array[] = [];
  while (snapshots.length < 4) {
    const snapshot = run.snapshot();
    snapshots.push(snapshot);
    sb.dispose();
    sb = new Sandbox(h.opts);
    run = sb.loadSnapshot(snapshot);
    await goOn(run);
  }
  assert.deepEqual(run.value, [3, 6]);
  assert.deepEqual(h.lines, ["i 0 0", "i 1 1", "i 2 3"]);
  // The loop's pauses hold the same state but for the numbers.
  const sizes = snapshots.slice(0, 3).map((d) => d.length);
  assert.ok(Math.max(...sizes) - Math.min(...sizes) < 8, String(sizes));
  sb.dispose();
});

test("run and runAsync call the debugger function; without the option, or outside async functions, nothing pauses", async () => {
  const module = await loadModule();
  let pauses = 0;
  const counting = host(module, { debugger: () => void pauses++ });
  const sb0 = new Sandbox(counting.opts);
  assert.deepEqual(await sb0.runAsync(LOOP), [3, 6]);
  assert.deepEqual(sb0.run(LOOP), [3, 6]);
  assert.equal(pauses, 8);
  sb0.dispose();
  const sb = new Sandbox({ module });
  assert.equal(await sb.runAsync("debugger; return 1"), 1);
  sb.dispose();
  const h = host(module);
  const sb2 = new Sandbox(h.opts);
  const run = sb2.begin("function f() { debugger; return 2; } return f();");
  assert.equal(run.state, "done");
  assert.equal(run.value, 2);
  sb2.dispose();
});

test("callSite: where a pause waits, in the body, also after a load (D3054)", async () => {
  const h = host(await loadModule());
  const sb = new Sandbox(h.opts);
  const run = sb.begin(LOOP, {}, { filename: "loop.js" });
  const [call] = run.calls;
  assert.equal(call.fn, "debugger");
  assert.deepEqual(sb.callSite(call.id), { file: "loop.js", line: 6, column: 3 });
  assert.equal(sb.callSite(call.id + 100), undefined);
  // Continued from a snapshot, the run is in the same script: the next pause's position.
  const snapshot = run.snapshot();
  sb.dispose();
  const sb2 = new Sandbox(h.opts);
  const loaded = sb2.loadSnapshot(snapshot);
  await goOn(loaded);
  assert.deepEqual(sb2.callSite(loaded.calls[0].id), { file: "loop.js", line: 6, column: 3 });
  sb2.dispose();
});
