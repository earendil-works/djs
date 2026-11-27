// `debugger;` as a pause (D3050): with the sandbox option `debugger`, the statement in an
// async function awaits a call of that host function, so a task can be suspended there
// into a snapshot, and a sandbox with the option that resumes the snapshot pauses again at
// the next `debugger;`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { type AsyncHostFunction, loadModule, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

/** Options whose `debugger` counts the pauses and resolves each when told to. */
function host(module: WebAssembly.Module) {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  let go: (() => void) | undefined;
  let paused = 0;
  const pause: AsyncHostFunction = {
    call() {
      paused++;
      return new Promise<void>((r) => (go = r));
    },
    // A pause continued from a snapshot goes on at once.
    resume() {},
  };
  const opts: SandboxOptions = { module, frozenBuiltins: true, console: { log, info: log, warn: log, error: log }, debugger: pause };
  return { opts, lines, paused: () => paused, go: () => go?.() };
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

test("a debugger statement pauses the run; the task runs on when the pause resolves", async () => {
  const h = host(await loadModule());
  const sb = new Sandbox(h.opts);
  const task = sb.start(LOOP);
  for (let n = 1; n <= 4; n++) {
    while (h.paused() < n) await new Promise((r) => setTimeout(r, 1));
    h.go();
  }
  assert.deepEqual(await task.result, [3, 6]);
  assert.deepEqual(h.lines, ["i 0 0", "i 1 1", "i 2 3"]);
  sb.dispose();
});

test("a pause is snapshotted as an open call; each resumed snapshot pauses at the next debugger", async () => {
  const h = host(await loadModule());
  let task = new Sandbox(h.opts).start(LOOP);
  const snapshots: Uint8Array[] = [];
  for (;;) {
    while (task.calls.length === 0) await new Promise((r) => setTimeout(r, 1));
    assert.deepEqual(task.calls.map((c) => [c.fn, c.args]), [["debugger", []]]);
    const snapshot = await task.suspend({ sources: false });
    snapshots.push(snapshot);
    task.sandbox.dispose();
    task = await Sandbox.resume(snapshot, h.opts);
    if (snapshots.length === 4) break;
  }
  assert.deepEqual(await task.result, [3, 6]);
  assert.deepEqual(h.lines, ["i 0 0", "i 1 1", "i 2 3"]);
  // The loop's pauses hold the same state but for the numbers.
  const sizes = snapshots.slice(0, 3).map((d) => d.length);
  assert.ok(Math.max(...sizes) - Math.min(...sizes) < 8, String(sizes));
  task.sandbox.dispose();
});

test("without the option, or outside async functions, debugger does nothing", async () => {
  const module = await loadModule();
  const sb = new Sandbox({ module });
  assert.equal(await sb.runAsync("debugger; return 1"), 1);
  sb.dispose();
  const h = host(module);
  const sb2 = new Sandbox(h.opts);
  assert.equal(await sb2.runAsync("function f() { debugger; return 2; } return f();"), 2);
  assert.equal(h.paused(), 0);
  sb2.dispose();
});

test("callSite: where a pause waits, in the body, also after a resume (D3054)", async () => {
  const h = host(await loadModule());
  let task = new Sandbox(h.opts).start(LOOP, {}, { filename: "loop.js" });
  while (h.paused() < 1) await new Promise((r) => setTimeout(r, 1));
  const [call] = task.calls;
  assert.equal(call.fn, "debugger");
  assert.deepEqual(task.sandbox.callSite(call.id), { file: "loop.js", line: 6, column: 3 });
  assert.equal(task.sandbox.callSite(call.id + 100), undefined);
  // Continued from a snapshot, the run is in the same script: the next pause's position.
  const snapshot = await task.suspend();
  task.sandbox.dispose();
  task = await Sandbox.resume(snapshot, h.opts);
  while (h.paused() < 2) await new Promise((r) => setTimeout(r, 1));
  assert.deepEqual(task.sandbox.callSite(task.calls[0].id), { file: "loop.js", line: 6, column: 3 });
  task.result.catch(() => {});
  task.sandbox.dispose();
});
