// pi's follow-ups to 0295 (0296, D3620-D3623): an argument a plain host function cannot
// read throws at the call, the globals the sandbox binds are read-only, an Error's name and
// message are coerced like String(), and namespaces have a null prototype.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, type Run, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

const quiet = { log() {}, info() {}, warn() {}, error() {} };

async function finish(run: Run, answer: (fn: string, args: unknown[]) => unknown): Promise<void> {
  while (run.state === "idle" && run.calls.length > 0) {
    for (const c of run.calls) run.settle(c.id, { ok: true, value: answer(c.fn, c.args) });
    await run.advance();
  }
}

// ---- (1) an unreadable argument throws at the call ------------------------------------

test("a cyclic argument to a plain host function is a TypeError at the call", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, { except: ["text"] }]) {
    let called = 0;
    const lines: unknown[][] = [];
    const sb = new Sandbox({
      module,
      console: { ...quiet, log: (...a) => void lines.push(a) },
      frozenBuiltins: true,
      jsonBoundary,
      functions: { text: () => void called++ },
      asyncFunctions: { "tools.plain": () => called++ },
      ...(jsonBoundary ? {} : {}),
    });
    const run = sb.begin(`
      const c = {}; c.self = c;
      const out = [];
      try { text("ok", c); } catch (e) { out.push([e.name, e.message.startsWith("argument 2: cyclic value at $.self")]); }
      console.log("still", c);
      ${jsonBoundary ? "" : `try { await tools.plain(c); } catch (e) { out.push([e.name, e.message.startsWith("argument 1: cyclic value")]); }`}
      return out;
    `);
    await finish(run, () => 0);
    assert.equal(run.state, "done", String(run.error));
    const expected = jsonBoundary ? [["TypeError", true]] : [["TypeError", true], ["TypeError", true]];
    assert.deepEqual(run.value, expected);
    assert.equal(called, 0, "the host function is not called");
    // console gets the reason as text, and the run goes on.
    assert.equal(lines[0][0], "still");
    assert.match(String(lines[0][1]), /^\[cyclic value at \$\.self/);
    sb.dispose();
  }
});

// ---- (2) read-only host globals --------------------------------------------------------

test("the globals the sandbox binds are non-writable and non-configurable, also after a load", async () => {
  const module = await loadModule();
  const opts: SandboxOptions = {
    module,
    console: quiet,
    frozenBuiltins: true,
    functions: { text: () => 0 },
    asyncFunctions: { "tools.ask": () => 0 },
    values: { ALL_TOOLS: [{ name: "ask" }] },
  };
  const probe = `
    const out = [];
    for (const name of ["tools", "console", "text", "ALL_TOOLS"]) {
      const d = Object.getOwnPropertyDescriptor(globalThis, name);
      let threw = false;
      try { globalThis[name] = null; } catch (e) { threw = e instanceof TypeError; }
      let deleted = true;
      try { deleted = delete globalThis[name]; } catch (e) { deleted = false; }
      out.push([name, d.writable, d.configurable, threw, deleted, typeof globalThis[name]]);
    }
    return out;
  `;
  const expected = [
    ["tools", false, false, true, false, "object"],
    ["console", false, false, true, false, "object"],
    ["text", false, false, true, false, "function"],
    ["ALL_TOOLS", false, false, true, false, "object"],
  ];
  const sb = new Sandbox(opts);
  assert.deepEqual(sb.run(probe), expected);
  // A dump and load: the loading sandbox binds them the same way.
  const run = sb.begin(`const a = await tools.ask(); ${probe.replace("return out;", "return [a, out];")}`);
  const dump = run.dump();
  sb.dispose();
  const again = new Sandbox(opts);
  const loaded = again.loadRun(dump);
  await finish(loaded, () => "yes");
  assert.deepEqual(loaded.value, ["yes", expected]);
  again.dispose();
  // `globals` binds the same way.
  const g = new Sandbox({ module, console: quiet, functions: { "x.f": () => 1 }, globals: { myF: "x.f", X: "x" } });
  assert.deepEqual(g.run("try { myF = 1; } catch (e) { return [e instanceof TypeError, typeof X, myF()]; }"), [true, "object", 1]);
  g.dispose();
});

// ---- (3) name and message coerced like String() ---------------------------------------

test("an Error's name and message that are not strings arrive as String() makes them", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const sb = new Sandbox({ module, console: quiet, jsonBoundary });
    const value = (code: string) => {
      try {
        sb.run(code);
      } catch (e) {
        assert.ok(e instanceof SandboxError);
        return e.value as Error;
      }
      assert.fail("no throw");
    };
    const a = value("const e = new Error('x'); e.message = 42; throw e");
    assert.deepEqual([a.name, a.message], ["Error", "42"]);
    const b = value("const e = new TypeError('x'); e.name = null; e.message = 10n; throw e");
    assert.deepEqual([b.name, b.message], ["null", "10"]);
    const c = value("const e = new Error('x'); e.message = undefined; e.name = undefined; throw e");
    assert.deepEqual([c.name, c.message], ["Error", ""]);
    const d = value("const e = new Error('x'); e.message = 1.5; e.name = false; throw e");
    assert.deepEqual([d.name, d.message], ["false", "1.5"]);
    // An object would need its toString run: the default.
    const o = value("const e = new Error('x'); e.message = { toString() { return 'm'; } }; throw e");
    assert.equal(o.message, "");
    sb.dispose();
  }
});

// ---- (4) null-prototype namespaces ----------------------------------------------------

test("namespaces have a null prototype; missingMember is not asked about Object.prototype's names", async () => {
  const module = await loadModule();
  const asked: string[] = [];
  const opts: SandboxOptions = {
    module,
    console: quiet,
    frozenBuiltins: true,
    missingMember: (ns, m) => (asked.push(`${ns}.${m}`), `${ns}.${m} does not exist`),
    asyncFunctions: { "tools.ask": () => 0 },
    modules: (s) => (s === "@m/x" ? { exports: { one: "tools.ask" } } : undefined),
  };
  const probe = `
    const out = [Object.getPrototypeOf(tools), Object.getPrototypeOf(console), Object.getPrototypeOf(x),
      tools.toString, tools.hasOwnProperty, "toString" in tools, Object.keys(tools)];
    try { tools.nope; } catch (e) { out.push(e.message); }
    return out;
  `;
  const expected = [null, null, null, undefined, undefined, false, ["ask"], "tools.nope does not exist"];
  const sb = new Sandbox(opts);
  const run = sb.begin(`import * as x from "@m/x"; const a = await tools.ask(); ${probe.replace("return out;", "return [a, out];")}`);
  const dump = run.dump();
  sb.dispose();
  const again = new Sandbox(opts);
  const loaded = again.loadRun(dump);
  await finish(loaded, () => "yes");
  assert.equal(loaded.state, "done", String(loaded.error));
  assert.deepEqual(loaded.value, ["yes", expected]);
  assert.deepEqual(asked, ["tools.nope"]);
  again.dispose();
  // Without missingMember too.
  const plain = new Sandbox({ module, console: quiet, functions: { "tools.f": () => 1 } });
  assert.deepEqual(plain.run("return [Object.getPrototypeOf(tools) === null, tools.toString === undefined, tools.f()]"), [true, true, 1]);
  plain.dispose();
});

// ---- 0297: the missing-member error's stack starts at the script ----------------------

test("missingMember's TypeError has the script's frames only, in the VM and at the host", async () => {
  const module = await loadModule();
  for (const jsonBoundary of [false, true]) {
    const lines: unknown[][] = [];
    const sb = new Sandbox({
      module,
      console: { ...quiet, log: (...a) => void lines.push(a) },
      frozenBuiltins: true,
      jsonBoundary,
      missingMember: (ns, m) => `${ns}.${m} does not exist`,
      functions: { "tools.echo": (x) => x },
    });
    const stack = "    at f (codemode.js:1:28)\n    at <eval> (codemode.js:2:";
    // The thrown error as the host gets it.
    const run = sb.begin("function f() { return tools.Echo(1) }\nreturn f()", {}, { filename: "codemode.js" });
    assert.equal(run.state, "threw");
    const e = run.error!.value as Error;
    assert.deepEqual([e.name, e.message, e.stack], ["TypeError", "tools.Echo does not exist", `${stack}9)\n`]);
    // As the script reads it, and as console gets it.
    const inVm = sb.begin("function f() { return tools.Echo(1) }\ntry { f() } catch (e) { console.log(e); return e.stack }", {}, { filename: "codemode.js" });
    assert.equal(inVm.value, `${stack}8)\n`);
    assert.equal((lines[0][0] as Error).stack, `${stack}8)\n`);
    sb.dispose();
  }
});

test("a stack the script set is the stack the host reads", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet });
  const run = sb.begin("const e = new Error('x'); e.stack = 'custom'; throw e");
  assert.equal((run.error!.value as Error).stack, "custom");
  sb.dispose();
});
