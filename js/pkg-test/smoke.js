// The packed package's smoke test (D3108): plain JavaScript that imports the package by
// name and uses each entry point, so it runs unchanged in node, Bun, Deno and a browser
// (scripts/npm-test.sh, js/pkg-test/browser.js). `smoke()` returns what it checked and
// throws at the first failure.

import { Sandbox, SandboxError, loadModule } from "@earendil-works/djs";
import { Sandbox as SlimSandbox } from "@earendil-works/djs/slim";
import { Sandbox as DumpSandbox, loadModule as loadDumpModule } from "@earendil-works/djs/dump";
import { helpers } from "@earendil-works/djs/dump-runtime";

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

  // The main entry point: its own module by default, sync and async runs, errors.
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
  eq(sb.run("return typeof BigInt64Array"), "function", "main module: full builtins");
  for (const s of [sb, io, ts]) s.dispose();
  done.push("main entry point: run, runAsync, console, typescript, errors");

  // The slim entry point: its own module, without the full families.
  const slim = await SlimSandbox.create();
  eq(slim.run("return [typeof BigInt64Array, 'ok'.padStart(4)]"), ["undefined", "  ok"], "slim");
  slim.dispose();
  if ((await loadModule()) === (await loadDumpModule())) throw new Error("the entry points share a module");
  done.push("slim entry point");

  // The dump entry point (packaging A): a task suspended into a dump and resumed.
  const w = waiter();
  const opts = { frozenBuiltins: true, asyncFunctions: { "tools.wait": w.fn } };
  const a = await DumpSandbox.create(opts);
  const task = a.start(SCRIPT, { x: "a" });
  const dump = await task.suspend();
  a.dispose();
  w.hold = false;
  if (!dump.startsWith('"djs-dump";')) throw new Error(`not a dump: ${dump.slice(0, 40)}`);
  const resumed = await DumpSandbox.resume(dump, opts);
  eq(await resumed.result, ["resumed a", "got resumed a!"], "dump entry point: resume");
  resumed.sandbox.dispose();
  done.push("dump entry point: suspend and resume");

  // The main module with the dump module as its dumpModule (packaging B).
  const w2 = waiter();
  const optsB = { frozenBuiltins: true, asyncFunctions: { "tools.wait": w2.fn }, dumpModule: await loadDumpModule() };
  const b = await Sandbox.create(optsB);
  const dumpB = await b.start(SCRIPT, { x: "b" }).suspend();
  b.dispose();
  w2.hold = false;
  const resumedB = await Sandbox.resume(dumpB, optsB);
  eq(await resumedB.result, ["resumed b", "got resumed b!"], "dumpModule: resume");
  resumedB.sandbox.dispose();
  done.push("main entry point with the dump module: suspend and resume");

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
    eq(await (0, eval)(`"use strict"; ${dumpB}`), ["host resumed b", "host host resumed b!"], "dump in the host engine");
  } finally {
    delete g.$djs;
    delete g.$dump;
    delete g.tools;
  }
  done.push("dump-runtime: the dump continued in the host engine");
  return done;
}
