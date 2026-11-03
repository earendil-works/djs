// Host-driven runs (0179, the first piece of docs/continuation-dumps.md "Sandbox API"):
// `begin` runs to an idle point, the host settles the open async calls itself, `advance`
// continues; `Sandbox.resume` continues a run from a format 2 dump.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, Sandbox, SandboxError } from "../src/sandbox.ts";

const BODY = `
  console.log("before");
  const r = await sleep(ms);
  console.log("after", r);
  return r + 1;
`;

function capture(): { lines: string[]; console: Console } {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  return { lines, console: { log, info: log, warn: log, error: log } as unknown as Console };
}

test("begin stops at the idle point; settle and advance finish the run", async () => {
  const module = await loadModule();
  let called = 0;
  const out = capture();
  const sb = new Sandbox({ module, console: out.console, asyncFunctions: { sleep: () => void called++ } });
  const run = sb.begin(BODY, { ms: 5 });
  assert.equal(run.state, "idle");
  assert.deepEqual(run.calls.map((c) => [c.fn, c.args]), [["sleep", [5]]]);
  assert.equal(called, 0, "a host-driven run never calls async functions itself");
  assert.throws(() => sb.run("return 1"), (e: SandboxError) => e.kind === "misuse");
  await run.advance(); // nothing settled: still idle
  assert.equal(run.state, "idle");
  run.settle(run.calls[0].id, { ok: true, value: 41 });
  assert.equal(run.calls.length, 0);
  await run.advance();
  assert.equal(run.state, "done");
  assert.equal(run.value, 42);
  assert.deepEqual(out.lines, ["before", "after 41"]);
  assert.equal(sb.run("return 1"), 1, "the sandbox runs again after the run ended");
  await assert.rejects(run.advance(), /the run is done/);
});

test("a rejected call and a script exception end the run as threw", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: capture().console, asyncFunctions: { sleep: () => 0 } });
  const run = sb.begin(BODY, { ms: 1 });
  assert.throws(() => run.settle(999, { ok: true, value: 1 }), /no open call 999/);
  run.settle(run.calls[0].id, { ok: false, value: new RangeError("too long") });
  await run.advance();
  assert.equal(run.state, "threw");
  assert.equal(run.error?.message, "RangeError: too long");
  const sync = sb.begin("throw new TypeError('x')");
  assert.equal(sync.state, "threw");
  assert.equal(sb.begin("return 7").value, 7);
});

// Hand-written until the exporter writes it (0172): the body stopped at its await.
const DUMP = `// djs-dump 2
$dump.f1 = async function (ms) {
${BODY}
};
$dump.handle("run", $dump.resume($dump.f1, {
  at: "await#1",
  awaiting: $dump.pending(3, "sleep", [5]),
  this: undefined,
  args: [5],
  bindings: { ms: 5, r: $dump.TDZ },
}));
`;

test("resume continues a dumped run with the same open calls", async () => {
  const module = await loadModule();
  const out = capture();
  const { sandbox, run } = await Sandbox.resume(DUMP, { module, console: out.console, asyncFunctions: { sleep: () => 0 } });
  assert.equal(run.state, "idle");
  assert.deepEqual(run.calls, [{ id: 3, fn: "sleep", args: [5] }]);
  run.settle(3, { ok: true, value: 1 });
  await run.advance();
  assert.equal(run.value, 2);
  assert.deepEqual(out.lines, ["after 1"]);
  assert.equal(sandbox.run("return typeof $dump"), "undefined");
});

test("resume refuses a dump without a run, and unknown host functions", async () => {
  const module = await loadModule();
  await assert.rejects(Sandbox.resume("// djs-dump 2\nvar x = 1;", { module }), /holds no run/);
  await assert.rejects(Sandbox.resume(DUMP, { module }), /unknown host function sleep/);
  const sb = new Sandbox({ module, asyncFunctions: { sleep: () => 0 } });
  const { calls } = sb.load(DUMP);
  assert.deepEqual(calls, [{ id: 3, fn: "sleep", args: [5] }]);
});
