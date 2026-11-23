// The packed package's smoke test (D3108): plain JavaScript that imports the package by
// name and uses its API, so it runs unchanged in node, Bun, Deno and a browser
// (scripts/npm-test.sh, js/pkg-test/browser.js). `smoke()` returns what it checked and
// throws at the first failure.

import { Sandbox, SandboxError, helpers, scriptFor } from "@earendil-works/djs";

function eq(actual, expected, what) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${what}: got ${a}, expected ${e}`);
}

/**
 * `tools.wait(v)`: never settles while `w.hold` (the run waits there to be dumped), else
 * resolves to `got v`; a resumed call resolves to `resumed v`.
 */
function waiter() {
  const w = {
    hold: true,
    fn: {
      call(v) {
        this.state = { v };
        return w.hold ? new Promise(() => {}) : `got ${v}`;
      },
      resume: ({ v }) => `resumed ${v}`,
    },
  };
  return w;
}

const SCRIPT = "const a = await tools.wait(x); const b = await tools.wait(a + '!'); return [a, b];";

export async function smoke() {
  const done = [];

  // The package's module by default, sync and async runs, errors.
  const sb = await Sandbox.create();
  eq(sb.run("return a + b", { a: 40, b: 2 }), 42, "run");
  const sync = new Sandbox(); // the default module is loaded now
  eq(sync.run("return [1, 2, 3].map((x) => x * 2)"), [2, 4, 6], "new Sandbox()");
  sync.dispose();
  const logged = [];
  const log = (...a) => logged.push(a.join(" "));
  const io = await Sandbox.create({
    asyncFunctions: { "tools.fetch": async (u) => ({ url: u, items: [1, 2, 3] }) },
    console: { log, info: log, warn: log, error: log },
  });
  eq(await io.runAsync("const r = await tools.fetch(u); console.log('n', r.items.length); return r", { u: "x" }), { url: "x", items: [1, 2, 3] }, "runAsync");
  eq(logged, ["n 3"], "console");
  const ts = await Sandbox.create({ typescript: true });
  eq(ts.run("const n: number = 2; return n ** 10 as number"), 1024, "typescript");
  try {
    sb.run("throw new TypeError('no')");
    throw new Error("no SandboxError");
  } catch (e) {
    if (!(e instanceof SandboxError) || e.kind !== "threw") throw e;
  }
  try {
    (await Sandbox.create({ maxFuel: 10_000 })).run("for (;;) {}");
    throw new Error("no fuel error");
  } catch (e) {
    if (!(e instanceof SandboxError) || e.kind !== "fuel") throw e;
  }
  eq(sb.run("return typeof BigInt64Array"), "function", "full builtins");
  for (const s of [sb, io, ts]) s.dispose();
  done.push("run, runAsync, console, typescript, errors");

  // The package's one module has the dumper: a task suspended into a dump and resumed.
  const w = waiter();
  const opts = { frozenBuiltins: true, asyncFunctions: { "tools.wait": w.fn } };
  const a = await Sandbox.create(opts);
  const task = a.start(SCRIPT, { x: "a" });
  const dump = await task.suspend();
  a.dispose();
  w.hold = false;
  if (!dump.startsWith('"djs-dump";')) throw new Error(`not a dump: ${dump.slice(0, 40)}`);
  const resumed = await Sandbox.resume(dump, opts);
  eq(await resumed.result, ["resumed a", "got resumed a!"], "resume");
  resumed.sandbox.dispose();
  done.push("suspend and resume");

  // The dump continued in the host's own engine, with the dump runtime's helpers.
  const g = globalThis;
  g.$djs = {
    ...helpers,
    seed() {},
    resume: (_id, _name, _args, { v }) => `host resumed ${v}`,
    source: () => function script() {},
    host: (name) => {
      if (name === "tools") return { wait: async (v) => `host ${v}` };
      if (name === "tools.wait") return async (v) => `host ${v}`;
      throw new Error(`host: ${name}`);
    },
  };
  g.tools = { wait: async (v) => `host ${v}` };
  try {
    eq(await (0, eval)(`"use strict"; ${dump}`), ["host resumed a", "host host resumed a!"], "dump in the host engine");
  } finally {
    delete g.$djs;
    delete g.$dump;
    delete g.tools;
  }
  if (typeof scriptFor(dump) !== "string") throw new Error("scriptFor: no Script");
  done.push("the dump runtime: the dump continued in the host engine");
  return done;
}
