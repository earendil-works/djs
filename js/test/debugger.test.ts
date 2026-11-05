// `debugger;` as a pause (D3050): with the sandbox option `debugger`, the statement in an
// async function awaits a call of that host function, so a task can be suspended there
// into a dump, and a sandbox with the option that resumes the dump pauses again at the
// next `debugger;`.

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
    // A pause continued from a dump goes on at once.
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

test("a pause is dumped as an open call; each resumed dump pauses at the next debugger", async () => {
  const h = host(await loadModule());
  let task = new Sandbox(h.opts).start(LOOP);
  const dumps: string[] = [];
  for (;;) {
    const dump = await task.suspend();
    assert.match(dump, /\$djs\.resume\(\d+, "debugger", \[\]\)/);
    dumps.push(dump);
    task.sandbox.dispose();
    task = await Sandbox.resume(dump, h.opts);
    if (dumps.length === 4) break;
  }
  assert.deepEqual(await task.result, [3, 6]);
  assert.deepEqual(h.lines, ["i 0 0", "i 1 1", "i 2 3"]);
  // Re-dumps keep their size: the continuation is written from the original each time.
  const sizes = dumps.slice(0, 3).map((d) => d.length);
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

test("the option needs a module with the dumper", async () => {
  const nodump = process.env.DJS_WASM_NODUMP;
  if (!nodump) return;
  const module = await loadModule(nodump);
  assert.throws(
    () => new Sandbox({ module, debugger: () => {} }),
    (e: unknown) => e instanceof SandboxError && e.kind === "misuse" && /dumper/.test(e.message),
  );
});

test("callSite: where a pause waits, in the body, then in the dump the run continued from (D3054)", async () => {
  const h = host(await loadModule());
  let task = new Sandbox(h.opts).start(LOOP, {}, { filename: "loop.js" });
  while (h.paused() < 1) await new Promise((r) => setTimeout(r, 1));
  const [call] = task.calls;
  assert.equal(call.fn, "debugger");
  assert.deepEqual(task.sandbox.callSite(call.id), { file: "loop.js", line: 6, column: 3 });
  assert.equal(task.sandbox.callSite(call.id + 100), undefined);
  // Continued from a dump, the run executes the dump's text: positions are in it.
  const dump = await task.suspend();
  task.sandbox.dispose();
  task = await Sandbox.resume(dump, h.opts);
  while (h.paused() < 2) await new Promise((r) => setTimeout(r, 1));
  const at = task.sandbox.callSite(task.calls[0].id)!;
  assert.equal(at.file, "dump.js");
  assert.equal(dump.split("\n")[at.line - 1].slice(at.column - 1), "debugger;");
  task.result.catch(() => {});
  task.sandbox.dispose();
});

test("callSite is absent without the dumper", async () => {
  const nodump = process.env.DJS_WASM_NODUMP;
  if (!nodump) return;
  const sb = new Sandbox({ module: await loadModule(nodump) });
  assert.equal(sb.callSite(1), undefined);
  sb.dispose();
});
