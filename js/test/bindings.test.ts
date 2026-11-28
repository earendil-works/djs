// Host bindings (0213, D2970, D2971): a host function is identified by a name its host
// chooses; scripts find it where the sandbox binds it (globals, namespaces, imports), and
// snapshots name it by identity, so a run that holds a bare `myTool` is snapshotted and
// loaded like one that uses `tools.x`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, Sandbox, type SandboxOptions } from "../src/sandbox.ts";

const MY_TOOL = "@sdk/our-stuff#myTool";

/** `myTool(x)` and `sdk.fetch(x)` answer at once. */
const echo = (prefix: string) => (arg: unknown) => Promise.resolve(`${prefix} ${String(arg)}`);

async function options(extra: Partial<SandboxOptions> = {}): Promise<SandboxOptions> {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  return {
    module: await loadModule(),
    frozenBuiltins: true,
    console: { log, info: log, warn: log, error: log },
    asyncFunctions: { [MY_TOOL]: echo("my"), "sdk.fetch": echo("fetched") },
    globals: { console: "console" },
    modules: (spec) => (spec === "@sdk/our-stuff" ? { exports: { myTool: MY_TOOL, sdk: "sdk" } } : undefined),
    ...extra,
  };
}

const CODE = `
  import { myTool, sdk } from "@sdk/our-stuff";
  const held = myTool, ns = sdk;
  const a = await myTool("a");
  const b = await ns.fetch("b");
  const c = await held("c");
  return [a, b, c, held === myTool, ns === sdk, typeof tools, typeof globalThis.sdk];
`;

test("explicit globals and imports: only what the host binds is visible", async () => {
  const sb = new Sandbox(await options());
  assert.deepEqual(await sb.runAsync(CODE), ["my a", "fetched b", "my c", true, true, "undefined", "undefined"]);
  sb.dispose();
  // Globals of names the sandbox does not have fail at creation.
  const module = await loadModule();
  assert.throws(() => new Sandbox({ module, globals: { x: "nope" } }), /global x: no host function or namespace nope/);
});

test("the written rule: dotted names make namespace globals, other identities none", async () => {
  const sb = new Sandbox({ module: await loadModule(), functions: { "tools.read": (p) => `read ${String(p)}`, plain: () => 1, [MY_TOOL]: () => 2 } });
  assert.deepEqual(sb.run("return [tools.read('x'), plain(), typeof console.log, Object.keys(globalThis).includes('@sdk/our-stuff#myTool')]"), ["read x", 1, "function", false]);
  sb.dispose();
});

test("a run holding imported functions snapshots at every idle point and continues identically", async () => {
  const want = ["my a", "fetched b", "my c", true, true, "undefined", "undefined"];
  for (let k = 0; k < 3; k++) {
    const sb = new Sandbox(await options());
    const run = sb.begin(CODE, {}, { filename: "bound.js" });
    for (let i = 0; i < k; i++) {
      const c = run.calls[0];
      run.settle(c.id, { ok: true, value: c.fn === MY_TOOL ? `my ${String(c.args[0])}` : `fetched ${String(c.args[0])}` });
      await run.advance();
    }
    const snap = run.snapshot();
    sb.dispose();
    const again = new Sandbox(await options());
    const resumed = again.loadSnapshot(snap);
    while (resumed.state === "idle") {
      const c = resumed.calls[0];
      resumed.settle(c.id, { ok: true, value: c.fn === MY_TOOL ? `my ${String(c.args[0])}` : `fetched ${String(c.args[0])}` });
      await resumed.advance();
    }
    assert.equal(resumed.state, "done", String(resumed.error));
    assert.deepEqual(resumed.value, want, `snapshotted after ${k} results`);
    again.dispose();
  }
});
