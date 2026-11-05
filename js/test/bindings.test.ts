// Host bindings (0213, D2970-D2972): a host function is identified by a name its host
// chooses; scripts find it where the sandbox binds it (globals, namespaces, body bindings,
// the stand-in for imports), and dumps name it by identity (`$djs.host(name)`), so a run
// that holds a bare `myTool` dumps and resumes like one that uses `tools.x`, also in node.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Script } from "node:vm";
import { type AsyncHostFunction, loadModule, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";
import { helpers } from "../src/dump-runtime.js";

const MY_TOOL = "@sdk/our-stuff#myTool";

/** `myTool(x)` and `sdk.fetch(x)` answer at once; their state is their argument. */
function echo(prefix: string): AsyncHostFunction<{ arg: unknown }> {
  return {
    call(arg) {
      this.state = { arg };
      return Promise.resolve(`${prefix} ${String(arg)}`);
    },
    resume: ({ arg }) => Promise.resolve(`${prefix} ${String(arg)} (resumed)`),
  };
}

async function options(extra: Partial<SandboxOptions> = {}): Promise<SandboxOptions> {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  return {
    module: await loadModule(),
    frozenBuiltins: true,
    console: { log, info: log, warn: log, error: log },
    asyncFunctions: { [MY_TOOL]: echo("my"), "sdk.fetch": echo("fetched") },
    globals: { console: "console" },
    bindings: { myTool: MY_TOOL, sdk: "sdk" },
    ...extra,
  };
}

const CODE = `
  const held = myTool, ns = sdk;
  const a = await myTool("a");
  const b = await ns.fetch("b");
  const c = await held("c");
  return [a, b, c, held === myTool, ns === sdk, typeof tools, typeof globalThis.sdk];
`;

test("explicit globals and body bindings: only what the host binds is visible", async () => {
  const sb = new Sandbox(await options());
  assert.deepEqual(await sb.runAsync(CODE), ["my a", "fetched b", "my c", true, true, "undefined", "undefined"]);
  // A run parameter and a binding of the same name is a misuse.
  assert.throws(() => sb.run("return 1", { myTool: 1 }), (e: unknown) => e instanceof SandboxError && e.kind === "misuse");
  sb.dispose();
  // Bindings to names the sandbox does not have fail at the run; globals at creation.
  const bad = new Sandbox(await options({ bindings: { x: "nope" } }));
  assert.throws(() => bad.run("return 1"), /binding x: no host function or namespace nope/);
  bad.dispose();
  const module = await loadModule();
  assert.throws(() => new Sandbox({ module, globals: { x: "nope" } }), /global x: no host function or namespace nope/);
});

test("the written rule: dotted names make namespace globals, other identities none", async () => {
  const sb = new Sandbox({ module: await loadModule(), functions: { "tools.read": (p) => `read ${String(p)}`, plain: () => 1, [MY_TOOL]: () => 2 } });
  assert.deepEqual(sb.run("return [tools.read('x'), plain(), typeof console.log, Object.keys(globalThis).includes('@sdk/our-stuff#myTool')]"), ["read x", 1, "function", false]);
  sb.dispose();
});

test("a run with body bindings dumps at every idle point and continues identically", async () => {
  const want = ["my a", "fetched b", "my c", true, true, "undefined", "undefined"];
  for (let k = 0; k < 3; k++) {
    const sb = new Sandbox(await options());
    const run = sb.begin(CODE, {}, { filename: "bound.js" });
    for (let i = 0; i < k; i++) {
      const c = run.calls[0];
      run.settle(c.id, { ok: true, value: c.fn === MY_TOOL ? `my ${String(c.args[0])}` : `fetched ${String(c.args[0])}` });
      await run.advance();
    }
    const text = run.dump();
    assert.match(text, /\$djs\.host\("@sdk\/our-stuff#myTool"\)/);
    assert.match(text, /\$djs\.host\("sdk"\)/);
    sb.dispose();
    const again = new Sandbox(await options());
    const resumed = again.loadRun(text);
    while (resumed.state === "idle") {
      const c = resumed.calls[0];
      resumed.settle(c.id, { ok: true, value: c.fn === MY_TOOL ? `my ${String(c.args[0])}` : `fetched ${String(c.args[0])}` });
      await resumed.advance();
    }
    assert.equal(resumed.state, "done", String(resumed.error));
    assert.deepEqual(resumed.value, want, `dumped after ${k} results`);
    again.dispose();
  }
});

test("a suspended task with body bindings resumes in a sandbox and in node", async () => {
  const code = `const held = myTool; const a = await held("a"); return [a, held === myTool];`;
  const opts = await options();
  // A call that never returns, so the task waits at its first idle point.
  const stuck: AsyncHostFunction<{ arg: unknown }> = {
    call(arg) {
      this.state = { arg };
      return new Promise(() => {});
    },
    resume: ({ arg }) => Promise.resolve(`my ${String(arg)} (resumed)`),
  };
  const withStuck = { ...opts, asyncFunctions: { ...opts.asyncFunctions, [MY_TOOL]: stuck } };
  const task = new Sandbox(withStuck).start(code, {}, { filename: "task.js" });
  const dump = await task.suspend();
  await assert.rejects(task.result, (e: unknown) => e instanceof SandboxError && e.kind === "suspended");
  const again = await Sandbox.resume(dump, opts);
  assert.deepEqual(await again.result, ["my a (resumed)", true]);
  again.sandbox.dispose();

  // node: the host functions are bound nowhere but given to `$djs.host` by identity.
  const myTool = (arg: unknown) => Promise.resolve(`node ${String(arg)}`);
  const values: Record<string, unknown> = { [MY_TOOL]: myTool, sdk: { fetch: myTool }, console };
  (globalThis as Record<string, unknown>).$djs = {
    ...helpers,
    seed() {},
    resume: (_id: number, name: string, args: unknown[]) => (values[name] as (a: unknown) => unknown)(args[0]),
    source: () => function body() {},
    host: (name: string) => {
      if (!Object.hasOwn(values, name)) throw new Error(`host: unknown host value ${name}`);
      return values[name];
    },
  };
  const result = await new Script(dump, { filename: "task.dump.js" }).runInThisContext();
  // The open call is answered by calling the function again; `held` and the parameter
  // are the one object `$djs.host` gave for the identity.
  assert.deepEqual(result, ["node a", true]);
  assert.equal("$djs" in globalThis, false);
});
