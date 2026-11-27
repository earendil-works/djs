// Snapshots of runs (D3700, D3720): `Run.snapshot`, `Sandbox.loadSnapshot`, tasks'
// `suspend`/`checkpoint` and `Sandbox.resume`, with pi's code mode setup: namespaces with
// aliases and `missingMember` Proxies, `values`, the JSON boundary's converting functions,
// host modules, `RunExit` and the fuel budget across loads. The host's values are named in
// a snapshot, never written; the loading sandbox has its own.

import { test } from "node:test";
import assert from "node:assert/strict";
import { type AsyncHostFunction, type HostCall, loadModule, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";
import { answer, piOptions, piRun } from "./pi-setup.ts";

test("a task suspended into a snapshot resumes through the hooks, ids and all", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1); log("a", a, Math.random()); const b = await sleep(2); log("b", b, Math.random()); return a + b;`;
  const pending: { n: number; resolve(v: number): void }[] = [];
  const resumed: [unknown, HostCall][] = [];
  const calls: HostCall[] = [];
  const lines: string[] = [];
  const sleep: AsyncHostFunction<{ n: number }> = {
    call(n) {
      calls.push(this);
      this.state = { n: Number(n) };
      return new Promise((resolve) => pending.push({ n: Number(n), resolve }));
    },
    resume(state, call) {
      resumed.push([state, call]);
      return Promise.resolve(state.n * 100);
    },
  };
  const opts = { module, randomSeed: 7, functions: { log: (...a: unknown[]) => void lines.push(a.join(" ")) }, asyncFunctions: { sleep } };
  // The uninterrupted run's random numbers.
  const want: string[] = [];
  const plain = new Sandbox({ ...opts, functions: { log: (...a: unknown[]) => void want.push(a.join(" ")) }, asyncFunctions: { sleep: (n) => Number(n) * 100 } });
  await plain.runAsync(code);
  plain.dispose();

  const task = new Sandbox(opts).start(code);
  while (pending.length === 0) await new Promise((r) => setTimeout(r, 1));
  const snapshot = await task.suspend();
  await assert.rejects(task.result, (e: unknown) => e instanceof SandboxError && e.kind === "suspended");
  task.sandbox.dispose();
  const again = await Sandbox.resume(snapshot, opts);
  while (pending.length === 1) await new Promise((r) => setTimeout(r, 1));
  // The next call has the id the uninterrupted run gives it: `log("a")`, a sync call, takes 2.
  assert.deepEqual(calls.map((c) => [c.id, c.args]), [[1, [1]], [3, [2]]]);
  pending[1].resolve(200);
  assert.equal(await again.result, 300);
  assert.deepEqual(resumed.map(([s, c]) => [s, c.id]), [[{ n: 1 }, 1]]);
  // Math.random goes on with the same sequence.
  assert.deepEqual(lines, want);
  again.sandbox.dispose();
});

test("a snapshot without sources names its script by hash; describe tells what it holds", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1);\nreturn a + 1;`;
  const opts: SandboxOptions = { module, asyncFunctions: { sleep: { call: () => new Promise(() => {}), resume: () => 41 } } };
  const sb = new Sandbox(opts);
  const run = sb.begin(code, {}, { filename: "job-7.js" });
  const full = run.snapshot();
  const bare = run.snapshot({ sources: false });
  sb.dispose();
  assert.ok(bare.length < full.length, `${bare.length} < ${full.length}`);
  const text = new TextDecoder("latin1").decode(full);
  assert.ok(text.includes("return a + 1;"));
  assert.ok(!new TextDecoder("latin1").decode(bare).includes("return a + 1;"));
  // This process ran the script: it loads.
  const task = await Sandbox.resume(bare, opts);
  assert.equal(await task.result, 42);
  assert.match(task.sandbox.describeSnapshot(bare), /records/);
  assert.match(task.sandbox.describeSnapshot(bare), /job-7\.js/);
  assert.throws(() => task.sandbox.describeSnapshot(new Uint8Array([1, 2, 3])), /not a snapshot/);
  task.sandbox.dispose();
});

// ---- pi's code mode setup -------------------------------------------------------------

const PI_CODE = `
import { search } from "@mcp/github";
import * as gh from "@mcp/github";
const first = ALL_TOOLS[0];
const alias = tools["my-tool"];
const r = await tools.read({ path: "a", when: new Date(0) });
text(r);
text([first === ALL_TOOLS[0], alias === tools.my_tool, Object.isFrozen(ALL_TOOLS), gh.search === search]);
try { tools.Read; } catch (e) { text(e.constructor.name + ": " + e.message); }
const s = await search({ q: 1 });
text(s);
const later2 = await later({ x: [1, 2] });
text(later2);
await Promise.all([tools.my_tool("x"), searchTools("y")]).then((v) => text(v));
return { done: true, n: ALL_TOOLS.length };
`;

test("pi's setup through snapshots: host values keep their identity, the boundary converts", async () => {
  const module = await loadModule();
  const straight = await piRun(module, PI_CODE, false);
  const moved = await piRun(module, PI_CODE, true);
  assert.deepEqual(moved.out, straight.out);
  assert.deepEqual(moved.outcome, straight.outcome);
  assert.deepEqual(straight.outcome, { value: { done: true, n: 2 }, exited: false });
  assert.deepEqual(straight.out.slice(0, 3), [
    `text {"fn":"tools.read","args":[{"path":"a","when":"1970-01-01T00:00:00.000Z"}]}`,
    "text [true,true,true,true]",
    "text TypeError: tools.Read is not a tool. ALL_TOOLS lists every tool.",
  ]);
  assert.equal(moved.sizes.length, 5);
  // ALL_TOOLS's descriptions are the host's: never in a snapshot.
  assert.equal(moved.hostText, false);
  assert.ok(Math.max(...moved.sizes) < 6000, String(moved.sizes));
});

test("RunExit and the fuel budget across loads", async () => {
  const module = await loadModule();
  const code = `const a = await tools.read(1); text("got"); if (a) exit(); text("not reached");`;
  const r = await piRun(module, code, true);
  assert.deepEqual(r.out, ["text got"]);
  assert.deepEqual(r.outcome, { value: "exited", exited: true });
  // A loop between idle points: the budget counts every step across the loads.
  const spin = `let n = 0; for (let k = 0; k < 4; k++) { await later(k); for (let i = 0; i < 20000; i++) n++; } return n;`;
  const free = await piRun(module, spin, true);
  assert.deepEqual(free.outcome, { value: 80000, exited: false });
  const used = await (async () => {
    const out: string[] = [];
    const sb = new Sandbox(piOptions(module, out));
    const run = sb.begin(spin, {}, { start: false });
    await run.advance();
    while (run.state === "idle") {
      answer(run);
      await run.advance();
    }
    const u = run.fuelUsed;
    sb.dispose();
    return u;
  })();
  const capped = await piRun(module, spin, true, Math.floor(used * 0.75));
  assert.equal(capped.out.at(-1), "step: fuel");
});

test("a snapshot refuses a sandbox without a host value it names", async () => {
  const module = await loadModule();
  const out: string[] = [];
  const sb = new Sandbox(piOptions(module, out));
  const run = sb.begin(`const t = ALL_TOOLS; await later(1); return t.length;`);
  const snap = run.snapshot();
  sb.dispose();
  const other = new Sandbox({ ...piOptions(module, out), values: {} });
  assert.throws(() => other.loadSnapshot(snap), /no host value or function ALL_TOOLS/);
  // The sandbox is still usable: nothing was loaded.
  assert.equal(other.run("return 1 + 1"), 2);
  other.dispose();
  // Nor can a run be loaded twice, or into a sandbox that runs one.
  const busy = new Sandbox(piOptions(module, out));
  busy.loadSnapshot(snap);
  assert.throws(() => busy.loadSnapshot(snap), /a run is in progress/);
  busy.dispose();
});
