// The packed package's smoke test (D3108): plain JavaScript that imports the package by
// name and uses its API, so it runs unchanged in node, Bun, Deno and a browser
// (scripts/npm-test.sh, js/pkg-test/browser.js). `smoke()` returns what it checked and
// throws at the first failure.

import { Sandbox, SandboxError } from "@earendil-works/djs";

function eq(actual, expected, what) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${what}: got ${a}, expected ${e}`);
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

  // A host-driven run snapshotted while it waits and continued in a fresh sandbox.
  const opts = { frozenBuiltins: true, asyncFunctions: { "tools.wait": () => {} } };
  const a = await Sandbox.create(opts);
  const run = a.begin(SCRIPT, { x: "a" });
  eq(run.calls.map((c) => [c.fn, c.args]), [["tools.wait", ["a"]]], "calls");
  const snapshot = run.snapshot();
  a.dispose();
  if (!(snapshot instanceof Uint8Array)) throw new Error("not a snapshot");
  const b = await Sandbox.create(opts);
  const loaded = b.loadSnapshot(snapshot);
  while (loaded.state === "idle") {
    const [c] = loaded.calls;
    loaded.settle(c.id, { ok: true, value: `got ${c.args[0]}` });
    await loaded.advance();
  }
  eq(loaded.value, ["got a", "got got a!"], "loaded run");
  b.dispose();
  done.push("snapshot and load");

  return done;
}
