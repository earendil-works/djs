// Host-driven runs for durable code mode (0291, D3592-D3595): host errors carry their data
// properties, `values` are host data a dump names, the JSON boundary, `RunExit`, and
// bounded steps.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, type Run, RunExit, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

const quiet = { log() {}, info() {}, warn() {}, error() {} };

/** Settles every open call with `answer` and advances until the run is not idle. */
async function finish(run: Run, answer: (fn: string, args: unknown[]) => unknown): Promise<void> {
  while (run.state === "idle" && run.calls.length > 0) {
    for (const c of run.calls) run.settle(c.id, { ok: true, value: answer(c.fn, c.args) });
    await run.advance();
  }
}

// ---- host errors -------------------------------------------------------------------

test("a host error's own data properties reach the script's catch", async () => {
  const sb = new Sandbox({
    module: await loadModule(),
    console: quiet,
    functions: {
      fail: () => {
        throw Object.assign(new RangeError("no"), { code: "E_NO", structuredOutput: { rows: [1, 2] }, hook() {}, name: "Custom" });
      },
    },
    asyncFunctions: { "tools.ask": () => 0 },
  });
  const code = `
    const seen = [];
    for (const f of [() => fail(), () => tools.ask()]) {
      try { await f(); } catch (e) {
        seen.push([e instanceof RangeError, e.name, e.message, e.code, e.structuredOutput, "hook" in e, Object.keys(e)]);
      }
    }
    return seen;
  `;
  const run = sb.begin(code);
  assert.equal(run.state, "idle");
  run.settle(run.calls[0].id, { ok: false, value: Object.assign(new Error("denied"), { reason: "person", undef: undefined }) });
  await run.advance();
  assert.equal(run.state, "done", String(run.error));
  assert.deepEqual(run.value, [
    // name, message and stack are the VM's own; a function cannot cross and is left out.
    [true, "RangeError", "no", "E_NO", { rows: [1, 2] }, false, ["code", "structuredOutput"]],
    [false, "Error", "denied", undefined, undefined, false, ["reason"]],
  ]);
  sb.dispose();
});

// ---- values ------------------------------------------------------------------------

const TOOLS = [{ name: "read", description: "reads" }, { name: "ask", description: "asks a person" }];

test("values are deep-frozen globals that dumps name and loads must be given again", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = { module, frozenBuiltins: true, console: quiet, asyncFunctions: { "tools.ask": () => 0 }, values: { ALL_TOOLS: TOOLS, LIMIT: 3 } };
  const sb = new Sandbox(opts);
  const code = `
    const first = ALL_TOOLS[0], all = ALL_TOOLS;
    const frozen = [Object.isFrozen(ALL_TOOLS), Object.isFrozen(first), LIMIT];
    try { ALL_TOOLS[0].name = "x"; } catch (e) { frozen.push(e instanceof TypeError); }
    const answer = await tools.ask(ALL_TOOLS.map((t) => t.name));
    return [frozen, answer, all === ALL_TOOLS, first === ALL_TOOLS[0], first.name];
  `;
  const run = sb.begin(code, {}, { filename: "values.js" });
  assert.deepEqual(run.calls[0].args, [["read", "ask"]]);
  const dump = run.dump();
  assert.match(dump, /\$djs\.host\("ALL_TOOLS"\)/);
  assert.doesNotMatch(dump, /asks a person/, "the data is not written");
  sb.dispose();

  const again = new Sandbox(opts);
  const resumed = again.loadRun(dump);
  await finish(resumed, () => "yes");
  assert.deepEqual(resumed.value, [[true, true, 3, true], "yes", true, true, "read"]);
  again.dispose();

  // A sandbox without the value cannot load the dump.
  const without = new Sandbox({ ...opts, values: { LIMIT: 3 } });
  assert.throws(() => without.loadRun(dump), /unknown host value ALL_TOOLS/);
  without.dispose();
});

test("a value named like a host function, namespace or global is a misuse", async () => {
  const module = await loadModule();
  const misuse = (e: unknown) => e instanceof SandboxError && e.kind === "misuse";
  assert.throws(() => new Sandbox({ module, functions: { text: () => 0 }, values: { text: 1 } }), misuse);
  assert.throws(() => new Sandbox({ module, functions: { "tools.read": () => 0 }, values: { tools: {} } }), misuse);
  assert.throws(() => new Sandbox({ module, values: { console: [] } }), misuse);
  assert.throws(() => new Sandbox({ module, functions: { f: () => 0 }, globals: { g: "f" }, values: { g: 1 } }), misuse);
  assert.throws(() => new Sandbox({ module, values: { f: () => 0 } }), /value f: /);
  // A value is a host value like a namespace: bindings can name it.
  const sb = new Sandbox({ module, values: { DATA: { a: 1 } }, bindings: { data: "DATA" } });
  assert.deepEqual(sb.run("return [data === DATA, data.a]"), [true, 1]);
  sb.dispose();
});

// ---- the JSON boundary -------------------------------------------------------------

test("the JSON boundary converts arguments with JSON.stringify's rules", async () => {
  const got: unknown[][] = [];
  const sb = new Sandbox({
    module: await loadModule(),
    console: quiet,
    jsonBoundary: true,
    functions: { record: (...args) => void got.push(args) },
  });
  sb.run(`
    record(new Date(0), new Map([[1, 2]]), { a: undefined, f() {}, n: NaN, d: [undefined, () => 1] }, undefined, { toJSON: () => "j" }, () => 1, 7n === 7n);
    JSON.stringify = () => "patched";
    record({ still: "json" });
  `);
  assert.deepEqual(got, [
    ["1970-01-01T00:00:00.000Z", {}, { n: null, d: [null, null] }, undefined, "j", undefined, true],
    [{ still: "json" }],
  ]);
  sb.dispose();
});

test("a value JSON cannot carry fails at the call: a sync call throws, an async call rejects", async () => {
  let calls = 0;
  const sb = new Sandbox({
    module: await loadModule(),
    console: quiet,
    jsonBoundary: true,
    functions: { store: () => void calls++ },
    asyncFunctions: { "tools.read": () => calls++ },
  });
  const code = `
    const cyclic = {}; cyclic.self = cyclic;
    const out = [];
    try { store("k", cyclic); } catch (e) { out.push(["sync", e instanceof TypeError]); }
    try { store("k", 1n); } catch (e) { out.push(["bigint", e instanceof TypeError]); }
    const p = tools.read({ big: 2n });
    out.push(p instanceof Promise);
    try { await p; } catch (e) { out.push(["async", e instanceof TypeError]); }
    return out;
  `;
  const run = sb.begin(code);
  assert.equal(run.state, "done", String(run.error));
  assert.deepEqual(run.value, [["sync", true], ["bigint", true], true, ["async", true]]);
  assert.equal(calls, 0, "the host function is never called");
  sb.dispose();
});

test("the JSON boundary converts the run's result; a result JSON cannot carry fails the run", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, jsonBoundary: true, functions: { now: () => 1 } });
  assert.deepEqual(sb.run("return { when: new Date(0), m: new Map(), u: undefined }"), { when: "1970-01-01T00:00:00.000Z", m: {} });
  assert.equal(sb.run("return undefined"), undefined);
  assert.equal(await sb.runAsync("await 0; return new Date(0)"), "1970-01-01T00:00:00.000Z");
  assert.throws(() => sb.run("const a = []; a.push(a); return a"), (e: unknown) => e instanceof SandboxError && e.kind === "threw" && /TypeError/.test(e.message));
  const run = sb.begin("return [1n]");
  assert.equal(run.state, "threw");
  // A toJSON of the result may call host functions.
  assert.equal(sb.run("return { toJSON: () => now() + 1 }"), 2);
  sb.dispose();
});

test("converting functions are host values: one per identity, named in dumps, === across a load", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = {
    module,
    frozenBuiltins: true,
    console: quiet,
    jsonBoundary: true,
    asyncFunctions: { "tools.read": () => 0, "@mcp/github#search_issues": () => 0 },
    bindings: { search_issues: "@mcp/github#search_issues" },
  };
  const sb = new Sandbox(opts);
  const code = `
    const read = tools.read, search = search_issues;
    const a = await read({ at: new Date(0) });
    const b = await search({ q: "dumps", seen: new Set([1]) });
    return [a, b, read === tools.read, search === search_issues, read.name, typeof read];
  `;
  const run = sb.begin(code, {}, { filename: "json.js" });
  assert.deepEqual(run.calls.map((c) => [c.fn, c.args]), [["tools.read", [{ at: "1970-01-01T00:00:00.000Z" }]]]);
  const dump = run.dump();
  assert.match(dump, /\$djs\.host\("tools\.read"\)|\$djs\.host\("tools"\)/);
  assert.match(dump, /\$djs\.host\("@mcp\/github#search_issues"\)/);
  assert.doesNotMatch(dump, /stringify/, "the boundary's code is not in the dump");
  sb.dispose();
  const again = new Sandbox(opts);
  const resumed = again.loadRun(dump);
  assert.deepEqual(resumed.calls.map((c) => [c.fn, c.args]), [["tools.read", [{ at: "1970-01-01T00:00:00.000Z" }]]]);
  await finish(resumed, (fn, args) => `${fn} ${JSON.stringify(args)}`);
  assert.equal(resumed.state, "done", String(resumed.error));
  assert.deepEqual(resumed.value, [
    'tools.read [{"at":"1970-01-01T00:00:00.000Z"}]',
    '@mcp/github#search_issues [{"q":"dumps","seen":{}}]',
    true,
    true,
    "read",
    "function",
  ]);
  again.dispose();
});

test("console is not converted by the JSON boundary", async () => {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const sb = new Sandbox({ module: await loadModule(), console: { log, info: log, warn: log, error: log }, jsonBoundary: true });
  const cyclic: Record<string, unknown> = {};
  cyclic.self = cyclic;
  sb.run("const c = {}; c.self = c; console.log('x', new Date(0), 1n)");
  assert.deepEqual(lines, ["x [Date] 1"]);
  sb.dispose();
});

// ---- RunExit -----------------------------------------------------------------------

test("a host function's RunExit ends the run: no catch, no finally, no more script code", async () => {
  const out: string[] = [];
  const opts = async (): Promise<SandboxOptions> => ({
    module: await loadModule(),
    console: quiet,
    functions: {
      text: (s) => void out.push(String(s)),
      exit: (v) => {
        throw new RunExit(v);
      },
    },
    asyncFunctions: { "tools.wait": () => new Promise(() => {}) },
  });
  const code = `
    const pending = tools.wait();
    pending.then(() => text("then"), () => text("caught later"));
    try {
      text("before");
      try { exit(n); } catch (e) { text("caught"); } finally { text("finally"); }
      text("after");
    } finally { text("outer finally"); }
    return "returned";
  `;
  // Host-driven: done, exited, with the exit's value; the open call is dropped.
  const sb = new Sandbox(await opts());
  const run = sb.begin(code, { n: 7 });
  assert.deepEqual([run.state, run.exited, run.value, run.calls], ["done", true, 7, []]);
  assert.deepEqual(out, ["before"]);
  assert.equal(sb.run("return 1"), 1, "the sandbox runs again");
  out.length = 0;
  // `run`, `runAsync` and a task return the value.
  assert.equal(sb.run("text('a'); exit('v'); text('b')"), "v");
  assert.deepEqual(await sb.runAsync("await 0; text('c'); exit(['w']); text('d')"), ["w"]);
  assert.deepEqual(out, ["a", "c"]);
  sb.dispose();
});

test("RunExit through an async call: rejected by the function in a task, settled by the host", async () => {
  const sb = new Sandbox({
    module: await loadModule(),
    console: quiet,
    asyncFunctions: { "tools.quit": () => Promise.reject(new RunExit("bye")), "tools.ask": () => 0 },
  });
  const task = sb.start("let n = 0; try { await tools.quit(); } catch { n = 1; } return n;");
  assert.equal(await task.result, "bye");
  const run = sb.begin("try { await tools.ask(); } catch { return 'caught'; } return 'answered';");
  run.settle(run.calls[0].id, { ok: false, value: new RunExit() });
  await run.advance();
  assert.deepEqual([run.state, run.exited, run.value], ["done", true, undefined]);
  sb.dispose();
});

test("a recorded run that exits replays", async () => {
  const opts = async (): Promise<SandboxOptions> => ({
    module: await loadModule(),
    console: quiet,
    deterministic: { startedAt: 0, seed: 1 },
    functions: { exit: () => { throw new RunExit(42); } },
    asyncFunctions: { sleep: () => Promise.resolve(1) },
  });
  const journal: import("../src/sandbox.ts").JournalEntry[] = [];
  const code = "const a = await sleep(); exit(); return a;";
  assert.equal(await new Sandbox(await opts()).runAsync(code, {}, { journal }), 42);
  assert.equal(await new Sandbox(await opts()).replay(journal), 42);
});

// ---- bounded steps -----------------------------------------------------------------

test("a step yields to the event loop and stops when its signal is aborted", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, fuelSlice: 20_000 });
  const run = sb.begin("let i = 0; while (true) i++;", {}, { start: false });
  assert.equal(run.state, "ready");
  const ac = new AbortController();
  let ticks = 0;
  const timer = setInterval(() => {
    if (++ticks === 3) ac.abort(new Error("stop"));
  }, 5);
  const step = run.advance({ signal: ac.signal, yieldMs: 2 });
  await assert.rejects(step, (e: unknown) => e instanceof SandboxError && e.kind === "aborted" && /stop/.test(e.message));
  clearInterval(timer);
  assert.ok(ticks >= 3, "timers ran while the script computed");
  assert.equal(run.state, "threw");
  assert.equal(run.error?.kind, "aborted");
  assert.equal(sb.run("return 2"), 2, "the sandbox runs again");
  // An aborted signal stops the step before it runs.
  const again = sb.begin("return 1", {}, { start: false });
  await assert.rejects(again.advance({ signal: AbortSignal.abort() }), /aborted/);
  sb.dispose();
});

test("a step's fuel budget bounds it; begin's first step too", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, asyncFunctions: { tick: () => 0 } });
  const fuelOut = (e: unknown) => e instanceof SandboxError && e.kind === "fuel";
  assert.throws(() => sb.begin("let i = 0; while (true) i++;", {}, { fuel: 50_000 }), fuelOut);
  // Each step has its own budget: a run of short steps goes on.
  const code = "let s = 0; for (let k = 0; k < 3; k++) { for (let i = 0; i < 2000; i++) s += i; await tick(); } return s;";
  const run = sb.begin(code, {}, { fuel: 200_000 });
  while (run.state === "idle") {
    run.settle(run.calls[0].id, { ok: true, value: 0 });
    await run.advance({ fuel: 200_000 });
  }
  assert.equal(run.value, 3 * 1999 * 1000);
  const slow = sb.begin("await tick(); let i = 0; while (true) i++;");
  slow.settle(slow.calls[0].id, { ok: true, value: 0 });
  await assert.rejects(slow.advance({ fuel: 100_000 }), fuelOut);
  assert.equal(slow.state, "threw");
  sb.dispose();
});
