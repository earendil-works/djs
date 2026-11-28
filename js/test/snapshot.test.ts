// Snapshots of runs (D3700, D3720): `Run.snapshot` and `Sandbox.loadSnapshot`, with pi's
// code mode setup: namespaces with
// aliases and `missingMember` Proxies, `values`, the JSON boundary's converting functions,
// host modules, `RunExit` and the fuel budget across loads. The host's values are named in
// a snapshot, never written; the loading sandbox has its own.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";
import { docsModule } from "./docs-module.ts";
import { answer, piOptions, piRun } from "./pi-setup.ts";

test("a loaded run goes on with the call ids and random numbers of the uninterrupted one", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1); log("a", a, Math.random()); const b = await sleep(2); log("b", b, Math.random()); return a + b;`;
  const lines: string[] = [];
  const opts = { module, randomSeed: 7, functions: { log: (...a: unknown[]) => void lines.push(a.join(" ")) }, asyncFunctions: { sleep: () => 0 } };
  // The uninterrupted run's random numbers.
  const want: string[] = [];
  const plain = new Sandbox({ ...opts, functions: { log: (...a: unknown[]) => void want.push(a.join(" ")) }, asyncFunctions: { sleep: (n) => Number(n) * 100 } });
  await plain.runAsync(code);
  plain.dispose();

  const sb = new Sandbox(opts);
  const run = sb.begin(code);
  assert.deepEqual(run.calls.map((c) => [c.id, c.args]), [[1, [1]]]);
  const snapshot = run.snapshot({ states: { 1: { n: 1 } } });
  sb.dispose();
  const again = new Sandbox(opts);
  const loaded = again.loadSnapshot(snapshot);
  assert.deepEqual(loaded.calls, [{ id: 1, fn: "sleep", args: [1], state: { n: 1 } }]);
  loaded.settle(1, { ok: true, value: 100 });
  await loaded.advance();
  // The next call has the id the uninterrupted run gives it: `log("a")`, a sync call, takes 2.
  assert.deepEqual(loaded.calls.map((c) => [c.id, c.args]), [[3, [2]]]);
  loaded.settle(3, { ok: true, value: 200 });
  await loaded.advance();
  assert.equal(loaded.value, 300);
  // Math.random goes on with the same sequence.
  assert.deepEqual(lines, want);
  again.dispose();
});

test("a snapshot stores its script; describe tells what it holds", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1);\nreturn a + 1;`;
  const opts: SandboxOptions = { module, asyncFunctions: { sleep: () => 0 } };
  const sb = new Sandbox(opts);
  const snap = sb.begin(code, {}, { filename: "job-7.js" }).snapshot();
  sb.dispose();
  assert.ok(new TextDecoder("latin1").decode(snap).includes("return a + 1;"));
  const again = new Sandbox(opts);
  const run = again.loadSnapshot(snap);
  run.settle(run.calls[0].id, { ok: true, value: 41 });
  await run.advance();
  assert.equal(run.value, 42);
  if (await docsModule()) {
    const text = again.describeSnapshot(snap);
    assert.match(text, /^job-7\.js, waiting for the host to answer 1 call/);
    assert.match(text, /the body is waiting for sleep\(1\), at `await sleep\(1\)` \(line 1\)/);
    assert.throws(() => again.describeSnapshot(new Uint8Array([1, 2, 3])), /not a snapshot/);
    // The same as data: the view in JavaScript terms, and every record.
    const i = again.inspectSnapshot(snap);
    assert.equal(i.bytes, snap.length);
    assert.equal(i.scripts[0].name, "job-7.js");
    assert.equal(i.scripts[0].source, code);
    const call = i.view.calls[0];
    assert.equal(call.kind, "call");
    assert.equal(call.children[0].kind, "frame");
    assert.equal(call.children[0].source?.line, 1);
    assert.equal(i.run.calls[0].function, "sleep");
    assert.ok(i.records.every((r) => r.end > r.start && typeof r.label === "string" && typeof r.note === "string"));
    assert.equal(i.sizes.reduce((n, s) => n + s.bytes, 0), snap.length);
    assert.throws(() => again.inspectSnapshot(new Uint8Array([1, 2, 3])), /not a snapshot/);
  } else {
    // The shipped module has no describe (D3751).
    assert.throws(() => again.describeSnapshot(snap), (e: unknown) => e instanceof SandboxError && e.kind === "misuse");
    assert.throws(() => again.inspectSnapshot(snap), (e: unknown) => e instanceof SandboxError && e.kind === "misuse");
  }
  again.dispose();
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
