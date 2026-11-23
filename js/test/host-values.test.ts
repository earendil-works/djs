// What a host gets from a run, as pi's code mode needs it without a prelude (0295,
// D3600-D3603): thrown errors and console arguments as data, namespaces that are frozen,
// enumerable, with aliases and an answer for missing members, runs that end when their
// result settles, and the JSON boundary per function.

import { test } from "node:test";
import assert from "node:assert/strict";
import { type AsyncHostFunction, type ConsoleLike, loadModule, Opaque, type Run, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

const quiet = { log() {}, info() {}, warn() {}, error() {} };

/** Settles every open call with `answer` and advances until the run is not idle. */
async function finish(run: Run, answer: (fn: string, args: unknown[]) => unknown): Promise<void> {
  while (run.state === "idle" && run.calls.length > 0) {
    for (const c of run.calls) run.settle(c.id, { ok: true, value: answer(c.fn, c.args) });
    await run.advance();
  }
}

/** The error a sync `run` throws. */
function thrown(f: () => unknown): SandboxError {
  try {
    f();
  } catch (e) {
    assert.ok(e instanceof SandboxError);
    return e;
  }
  assert.fail("did not throw");
}

// ---- (a) a thrown Error as data ------------------------------------------------------

test("a thrown Error reaches the host with its name, message and the VM's stack", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    for (const frozenBuiltins of [false, true]) {
      const label = `jsonBoundary ${jsonBoundary}, frozenBuiltins ${frozenBuiltins}`;
      const sb = new Sandbox({ module, console: quiet, jsonBoundary, frozenBuiltins, asyncFunctions: { "tools.wait": () => 0 } });
      const code = "function f() { throw new TypeError(\"boom 1\"); }\nf();";
      const run = sb.begin(code, {}, { filename: "codemode.js" });
      assert.equal(run.state, "threw", label);
      assert.equal(run.error!.message, "TypeError: boom 1", label);
      const v = run.error!.value as Error;
      assert.ok(v instanceof Error, label);
      assert.equal(v.name, "TypeError", label);
      assert.equal(v.message, "boom 1", label);
      assert.equal(v.stack, "    at f (codemode.js:1:35)\n    at <eval> (codemode.js:2:2)\n", label);

      // The same through `run`, and from a rejection after an idle point.
      const e = thrown(() => sb.run(code));
      assert.equal((e.value as Error).name, "TypeError", label);
      const later = sb.begin(`
        class ToolError extends Error {
          constructor(m, code) { super(m, { cause: { why: "denied" } }); this.name = "ToolError"; this.code = code; }
        }
        await tools.wait();
        throw new ToolError("late", 7);
      `);
      await finish(later, () => 0);
      assert.equal(later.state, "threw", label);
      assert.equal(later.error!.message, "ToolError: late", label);
      const t = later.error!.value as Error & { code?: number };
      assert.deepEqual([t.name, t.message, t.code, t.cause], ["ToolError", "late", 7, { why: "denied" }], label);
      assert.match(t.stack!, /^ {4}at ToolError \(sandbox\.js:3:\d+\)\n/, label);
      sb.dispose();
    }
  }
});

test("a thrown non-Error stays a value: read as data, or through JSON with the boundary", async () => {
  const module = await loadModule();
  const plain = new Sandbox({ module, console: quiet });
  const boundary = new Sandbox({ module, console: quiet, jsonBoundary: true });
  const code = "throw { a: 1, when: new Date(0), f() {} }";
  const p = thrown(() => plain.run(code));
  assert.equal(p.message, "[object Object]");
  assert.deepEqual(Object.keys(p.value as object), ["a", "when", "f"]);
  assert.ok((p.value as { when: unknown }).when instanceof Date);
  assert.ok((p.value as { f: unknown }).f instanceof Opaque);
  const b = thrown(() => boundary.run(code));
  assert.equal(b.message, "[object Object]");
  assert.deepEqual(b.value, { a: 1, when: "1970-01-01T00:00:00.000Z" });
  assert.equal(thrown(() => boundary.run("throw 'text'")).value, "text");
  // JSON cannot carry it: undefined, and the message is still the value's display text.
  const c = thrown(() => boundary.run("const c = {}; c.self = c; throw c"));
  assert.deepEqual([c.message, c.value], ["[object Object]", undefined]);
  assert.equal(thrown(() => plain.run("const c = { n: 1 }; c.self = c; throw c")).value, undefined);
  assert.equal(thrown(() => boundary.run("throw () => 1")).value, undefined);
  // A promise the body rejects with: the same.
  assert.deepEqual(thrown(() => boundary.run("await null; throw { d: new Date(0) }")).value, { d: "1970-01-01T00:00:00.000Z" });
  plain.dispose();
  boundary.dispose();
});

// ---- (b) console arguments as data ---------------------------------------------------

test("console gets Errors, Dates, Maps and Sets as host values; console.debug exists", async () => {
  const module = await loadModule();
  const calls: [string, unknown[]][] = [];
  const at = (m: string) => (...a: unknown[]) => void calls.push([m, a]);
  const cons: ConsoleLike = { log: at("log"), info: at("info"), warn: at("warn"), error: at("error"), debug: at("debug") };
  for (const jsonBoundary of [false, true]) {
    calls.length = 0;
    const sb = new Sandbox({ module, console: cons, jsonBoundary, frozenBuiltins: true });
    sb.run(`
      console.log(new Error("inner"), new Map([[1, { a: 2 }]]), new Set(["x"]), new Date(0), () => 1, NaN);
      console.debug("dbg");
    `);
    const [[m, [err, map, set, date, fn, nan]], debug] = calls;
    assert.equal(m, "log");
    assert.ok(err instanceof Error);
    assert.deepEqual([err.name, err.message], ["Error", "inner"]);
    assert.match(err.stack!, /^ {4}at <eval> \(sandbox\.js:2:\d+\)\n$/);
    assert.deepEqual(map, new Map([[1, { a: 2 }]]));
    assert.deepEqual(set, new Set(["x"]));
    assert.deepEqual(date, new Date(0));
    assert.equal(fn, "[Function]");
    assert.ok(Number.isNaN(nan));
    assert.deepEqual(debug, ["debug", ["dbg"]]);
    sb.dispose();
  }
  // Without `debug`, `log` receives console.debug.
  const lines: unknown[][] = [];
  const sb = new Sandbox({ module, console: { ...quiet, log: (...a) => void lines.push(a) } });
  sb.run("console.debug('d', 1)");
  assert.deepEqual(lines, [["d", 1]]);
  sb.dispose();
});

// ---- (c) missing namespace members ---------------------------------------------------

const missingMember = (ns: string, member: string) =>
  member === "quiet" ? undefined : `${ns}.${member} does not exist. Did you mean ${ns}.${member.toLowerCase()}?`;

test("missingMember: reading a member a namespace lacks throws the host's message", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const sb = new Sandbox({ module, console: quiet, jsonBoundary, frozenBuiltins: true, missingMember, functions: { "tools.echo": (x) => x } });
    const out = sb.run(`
      const out = [];
      try { tools.Echo(1); } catch (e) { out.push([e instanceof TypeError, e.message, e.stack.split("\\n")[0]]); }
      out.push("Echo" in tools, "echo" in tools, tools.quiet === undefined, tools.then === undefined, typeof tools.toString, tools[Symbol.iterator] === undefined);
      out.push(JSON.stringify(tools), (await tools) === tools, tools.echo(2));
      try { console.trace("x"); } catch (e) { out.push(e.message); }
      return out;
    `) as unknown[];
    assert.deepEqual(out, [
      [true, "tools.Echo does not exist. Did you mean tools.echo?", "    at <eval> (sandbox.js:3:18)"],
      false, true, true, true, "function", true,
      "{}", true, 2,
      "console.trace does not exist. Did you mean console.trace?",
    ]);
    sb.dispose();
  }
});

test("missingMember: the namespace is still named by its identity in a dump, and guards after a load", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = { module, console: quiet, frozenBuiltins: true, missingMember, asyncFunctions: { "tools.ask": () => 0 } };
  const sb = new Sandbox(opts);
  const run = sb.begin(`
    const t = tools;
    const a = await t.ask();
    const out = [a, t === tools, Object.keys(t), Object.isFrozen(t)];
    try { t.Ask(); } catch (e) { out.push(e.message); }
    return out;
  `);
  assert.equal(run.state, "idle");
  const dump = run.dump();
  assert.match(dump, /\$djs\.host\("tools"\)/);
  sb.dispose();
  const again = new Sandbox(opts);
  const resumed = again.loadRun(dump);
  await finish(resumed, () => "yes");
  assert.equal(resumed.state, "done", String(resumed.error));
  assert.deepEqual(resumed.value, ["yes", true, ["ask"], true, "tools.Ask does not exist. Did you mean tools.ask?"]);
  again.dispose();
});

// ---- (d) namespace objects -----------------------------------------------------------

test("namespaces are frozen and list their members in definition order", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const sb = new Sandbox({
      module,
      console: quiet,
      jsonBoundary,
      functions: { "tools.b": () => 1, "models.list": () => [] },
      asyncFunctions: { "tools.a": () => 2, "tools.c": () => 3 },
    });
    // Defined before the first script runs: still a member.
    sb.defineFunction("tools.late", () => 4);
    const out = sb.run(`
      // (A codemode body is strict code: the writes throw.)
      const writes = [];
      for (const w of [() => { models.extra = 1; }, () => { tools.b = 0; }, () => delete tools.b, () => { console.log = null; }]) {
        try { w(); writes.push("ok"); } catch (e) { writes.push(e instanceof TypeError); }
      }
      return [Object.keys(tools), Object.keys(console), Object.isFrozen(tools), Object.isFrozen(models),
        Object.isFrozen(console), "extra" in models, writes];
    `);
    assert.deepEqual(out, [["b", "a", "c", "late"], ["log", "info", "warn", "error", "debug"], true, true, true, false, [true, true, true, true]]);
    // A namespace is frozen once a script ran: no more members.
    assert.throws(() => sb.defineFunction("tools.later", () => 5), (e: unknown) => e instanceof SandboxError && e.kind === "misuse");
    // A new namespace is frozen before the next script.
    sb.defineFunction("more.x", () => 6);
    assert.deepEqual(sb.run("return [more.x(), Object.isFrozen(more)]"), [6, true]);
    sb.dispose();
  }
});

test("frozen namespaces after a dump and load; a module namespace is frozen too", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = {
    module,
    console: quiet,
    frozenBuiltins: true,
    asyncFunctions: { "tools.ask": () => 0, "tools.tell": () => 0 },
    modules: (s) => (s === "@mcp/x" ? { exports: { one: {}, two: "tools.tell" } } : undefined),
  };
  const sb = new Sandbox(opts);
  const run = sb.begin(`
    import * as x from "@mcp/x";
    const a = await tools.ask();
    return [a, Object.keys(tools), Object.isFrozen(tools), Object.keys(x), Object.isFrozen(x), x.two === tools.tell];
  `);
  const dump = run.dump();
  sb.dispose();
  const again = new Sandbox(opts);
  const resumed = again.loadRun(dump);
  await finish(resumed, () => 1);
  assert.deepEqual(resumed.value, [1, ["ask", "tell"], true, ["one", "two"], true, true]);
  again.dispose();
});

// ---- (e) aliases ---------------------------------------------------------------------

test("aliases: more member names for one function, which keeps its identity", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const opts: SandboxOptions = {
      module,
      console: quiet,
      jsonBoundary,
      frozenBuiltins: true,
      asyncFunctions: { "tools.my_tool": () => 0, "tools.other": () => 0 },
      aliases: { "tools.my_tool": ["my-tool", "My Tool"] },
    };
    const sb = new Sandbox(opts);
    const run = sb.begin(`
      const f = tools["my-tool"];
      const r = await f(1);
      return [r, f === tools.my_tool, tools["My Tool"] === tools.my_tool, Object.keys(tools)];
    `);
    assert.deepEqual(run.calls.map((c) => [c.fn, c.args]), [["tools.my_tool", [1]]]);
    const dump = run.dump();
    assert.doesNotMatch(dump, /my-tool/);
    sb.dispose();
    const again = new Sandbox(opts);
    const resumed = again.loadRun(dump);
    await finish(resumed, () => "r");
    assert.deepEqual(resumed.value, ["r", true, true, ["my_tool", "my-tool", "My Tool", "other"]]);
    again.dispose();
  }
  // An alias that is an identifier would be the name of an identity of its own.
  assert.throws(
    () => new Sandbox({ module, functions: { "tools.my_tool": () => 0 }, aliases: { "tools.my_tool": ["myTool"] } }),
    (e: unknown) => e instanceof SandboxError && e.kind === "misuse",
  );
  assert.throws(
    () => new Sandbox({ module, functions: { tool: () => 0 }, aliases: { tool: ["a-b"] } }),
    (e: unknown) => e instanceof SandboxError && e.kind === "misuse",
  );
});

// ---- (f) a run ends when its result settles ------------------------------------------

test("a host-driven run ends when its result settles; the calls still open are dropped", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const sb = new Sandbox({ module, console: quiet, frozenBuiltins: true, jsonBoundary, asyncFunctions: { "tools.echo": () => {} } });
    const run = sb.begin("tools.echo(1); return 'x'", {}, { start: false });
    await run.advance({});
    assert.equal(run.state, "done");
    assert.equal(run.value, "x");
    assert.deepEqual(run.calls, []);
    assert.deepEqual(run.dropped.map((c) => [c.fn, c.args]), [["tools.echo", [1]]]);
    // Settling a dropped call is a no-op; the sandbox runs the next script.
    run.settle(run.dropped[0].id, { ok: true, value: 1 });
    assert.equal(sb.run("return 2"), 2);

    // A body that awaits one call and leaves another open; and one that rejects.
    const two = sb.begin("const a = tools.echo('a'); tools.echo('b'); return await a;");
    assert.equal(two.calls.length, 2);
    two.settle(two.calls[0].id, { ok: true, value: "A" });
    await two.advance();
    assert.deepEqual([two.state, two.value, two.dropped.map((c) => c.args)], ["done", "A", [["b"]]]);
    const rejects = sb.begin("tools.echo(1); await null; throw new RangeError('no')");
    assert.deepEqual([rejects.state, rejects.error?.message, rejects.dropped.length], ["threw", "RangeError: no", 1]);
    sb.dispose();
  }
});

test("a loaded run ends when its result settles too", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = { module, console: quiet, frozenBuiltins: true, asyncFunctions: { "tools.echo": () => {} } };
  const sb = new Sandbox(opts);
  const run = sb.begin("const a = tools.echo('a'); tools.echo('b'); return await a;");
  const dump = run.dump();
  sb.dispose();
  const again = new Sandbox(opts);
  const loaded = again.loadRun(dump);
  assert.equal(loaded.calls.length, 2);
  loaded.settle(loaded.calls[0].id, { ok: true, value: "A" });
  await loaded.advance();
  assert.deepEqual([loaded.state, loaded.value, loaded.dropped.map((c) => c.args)], ["done", "A", [["b"]]]);
  again.dispose();
});

test("runAsync and tasks end when the result settles and abort the open calls' signals", async () => {
  const module = await loadModule();
  const signals: AbortSignal[] = [];
  let release!: () => void;
  const hold = new Promise<void>((r) => (release = r));
  const slow: AsyncHostFunction = {
    async call() {
      signals.push(this.signal);
      await hold;
      return "late";
    },
  };
  const sb = new Sandbox({ module, console: quiet, frozenBuiltins: true, asyncFunctions: { "tools.slow": slow } });
  assert.equal(await sb.runAsync("tools.slow(); return 'x'"), "x");
  assert.deepEqual(signals.map((s) => s.aborted), [true]);
  const task = sb.start("tools.slow(); await null; return 'y'");
  assert.equal(await task.result, "y");
  assert.deepEqual(signals.map((s) => s.aborted), [true, true]);
  release();
  await hold;
  // The late results change nothing.
  assert.equal(await sb.runAsync("return 3"), 3);
  sb.dispose();
});

// ---- (g) the JSON boundary per function ----------------------------------------------

test("jsonBoundary.except: those functions get plain wire values", async () => {
  const module = await loadModule();
  const got: [string, unknown[]][] = [];
  const at = (name: string) => (...a: unknown[]) => void got.push([name, a]);
  const opts: SandboxOptions = {
    module,
    console: quiet,
    frozenBuiltins: true,
    jsonBoundary: { except: ["text", "store"] },
    functions: { text: at("text"), store: at("store"), "tools.sync": at("tools.sync") },
    asyncFunctions: { "tools.read": () => 0 },
  };
  const sb = new Sandbox(opts);
  const run = sb.begin(`
    text(0 / 0); text(() => 1); store("k", new Date(0)); tools.sync(0 / 0, new Date(0));
    const r = await tools.read(new Date(0));
    return { r, when: new Date(0) };
  `);
  assert.deepEqual(run.calls.map((c) => c.args), [["1970-01-01T00:00:00.000Z"]]);
  const dump = run.dump();
  sb.dispose();
  assert.equal(got.length, 4);
  assert.ok(Number.isNaN(got[0][1][0]));
  assert.ok(got[1][1][0] instanceof Opaque);
  assert.deepEqual(got[2], ["store", ["k", new Date(0)]]);
  assert.deepEqual(got[3], ["tools.sync", [null, "1970-01-01T00:00:00.000Z"]]);
  // The result is still converted; after a load as before.
  const again = new Sandbox(opts);
  const loaded = again.loadRun(dump);
  await finish(loaded, () => "read");
  assert.deepEqual(loaded.value, { r: "read", when: "1970-01-01T00:00:00.000Z" });
  again.dispose();
});

// ---- (h) maxFuel across the steps of a host-driven run -------------------------------

test("maxFuel bounds a host-driven run across its steps, and across a dump and load with fuelUsed", async () => {
  const module = await loadModule();
  const maxFuel = 10_000_000;
  const opts: SandboxOptions = { module, console: quiet, frozenBuiltins: true, jsonBoundary: true, maxFuel, asyncFunctions: { "tools.tick": () => {} } };
  const code = "for (;;) { for (let i = 0; i < 1e5; i++); await tools.tick(); }";
  /** Advances `run`, settling every call, until it fails; the number of idle steps. */
  const exhaust = async (run: Run): Promise<number> => {
    let steps = 0;
    for (;;) {
      try {
        await run.advance({ yieldMs: 10 });
      } catch (e) {
        assert.ok(e instanceof SandboxError && e.kind === "fuel", String(e));
        return steps;
      }
      steps++;
      assert.ok(steps < 1000, "never out of fuel");
      for (const c of run.calls) run.settle(c.id, { ok: true, value: 1 });
    }
  };
  const sb = new Sandbox(opts);
  const run = sb.begin(code, {}, { start: false });
  const steps = await exhaust(run);
  assert.ok(steps > 1, `${steps} steps: each is far below the budget`);
  assert.deepEqual([run.state, run.error?.message, run.fuelUsed], ["threw", `out of fuel after ${maxFuel} units`, maxFuel]);

  // Dumped half way: loaded with its fuelUsed, the run has the rest of the budget; without, a fresh one.
  const half = sb.begin(code, {}, { start: false });
  while (half.fuelUsed < maxFuel / 2) {
    await half.advance();
    for (const c of half.calls) half.settle(c.id, { ok: true, value: 1 });
    await half.advance();
  }
  const used = half.fuelUsed;
  const dump = half.dump();
  sb.dispose();
  for (const [fuelUsed, expected] of [[used, steps - Math.floor(used / (maxFuel / steps))], [undefined, steps]] as const) {
    const again = new Sandbox(opts);
    const loaded = again.loadRun(dump, { fuelUsed });
    assert.equal(loaded.fuelUsed, fuelUsed ?? 0);
    for (const c of loaded.calls) loaded.settle(c.id, { ok: true, value: 1 });
    const n = await exhaust(loaded);
    assert.ok(Math.abs(n - expected) <= 2, `fuelUsed ${fuelUsed}: ${n} steps, about ${expected} expected`);
    assert.equal(loaded.fuelUsed, maxFuel);
    again.dispose();
  }
});

test("maxFuel is exact in run and runAsync", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, maxFuel: 2_500_000 });
  assert.throws(() => sb.run("for (;;);"), { message: "out of fuel after 2500000 units" });
  await assert.rejects(sb.runAsync("for (;;) await null;"), { message: "out of fuel after 2500000 units" });
  sb.dispose();
});
