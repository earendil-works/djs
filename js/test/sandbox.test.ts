// Sandbox tests against a djs-wasm build: $DJS_WASM, or the `wasm` profile build
// (scripts/gate.sh builds a debug one and sets DJS_WASM).

import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Sandbox, SandboxError, defaultWasmPath, loadModule } from "../src/sandbox.ts";

const path = defaultWasmPath();
assert.ok(
  existsSync(typeof path === "string" ? path : fileURLToPath(path)),
  `djs-wasm not built at ${path}: run scripts/build-wasm.sh or set DJS_WASM`,
);

function collect() {
  const lines: string[] = [];
  const fmt = (args: unknown[]) => args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" ");
  return {
    lines,
    console: {
      log: (...a: unknown[]) => lines.push(fmt(a)),
      info: (...a: unknown[]) => lines.push("info " + fmt(a)),
      warn: (...a: unknown[]) => lines.push("warn " + fmt(a)),
      error: (...a: unknown[]) => lines.push("error " + fmt(a)),
    },
  };
}

test("the module is compiled once", async () => {
  assert.equal(loadModule(), loadModule());
  const a = await Sandbox.create();
  const b = await Sandbox.create();
  assert.equal(a.run("return 1 + 2"), 3);
  assert.equal(b.run("return 'b'"), "b");
  a.dispose();
  b.dispose();
});

test("the synchronous constructor", async () => {
  const module = await loadModule();
  const sb = new Sandbox({ module });
  assert.equal(sb.run("return 'sync'"), "sync");
  // After loadModule() of the default location, the module is optional.
  const sb2 = new Sandbox();
  assert.equal(sb2.run("return 2"), 2);
  const restored = new Sandbox({}, sb.bytes());
  assert.equal(restored.run("return 3"), 3);
  for (const s of [sb, sb2, restored]) s.dispose();
});

test("console and parameters", async () => {
  const c = collect();
  const sb = await Sandbox.create({ console: c.console });
  const r = sb.run("console.log('hi', n + 1); console.warn('w'); return n * 2", { n: 20 });
  assert.equal(r, 40);
  assert.deepEqual(c.lines, ["hi 21", "warn w"]);
  sb.dispose();
});

test("wire round trips through the VM", async () => {
  const sb = await Sandbox.create();
  const data = {
    s: "ascii",
    u: "Ünïcödé 日本 😀 \uD800",
    n: [0, -1, 1.5, 2 ** 40, -0],
    nested: { a: [1, { b: null }], t: true, f: false },
    big: "x".repeat(100_000),
  };
  assert.deepEqual(sb.run("return data", { data }), data);
  assert.equal(sb.run("return data.nested.a[1].b === null && data.u.length", { data }), data.u.length);
  assert.deepEqual(sb.run("let o = {k: [data.s, data.n.length]}; return o", { data }), { k: ["ascii", 5] });
  sb.dispose();
});

test("host functions", async () => {
  const c = collect();
  const sb = await Sandbox.create({
    console: c.console,
    functions: {
      add: (a, b) => (a as number) + (b as number),
      info: () => ({ name: "djs", tags: ["a", "b"] }),
      fail: () => {
        throw new TypeError("nope");
      },
      bad: () => () => 1,
    },
  });
  assert.equal(sb.run("return add(2, 3) * 10"), 50);
  assert.deepEqual(sb.run("let i = info(); return i.tags.length + i.name"), "2djs");
  assert.throws(() => sb.run("fail(); return 1"), (e: unknown) => {
    assert.ok(e instanceof SandboxError);
    assert.equal(e.kind, "threw");
    assert.equal(e.message, "TypeError: nope");
    return true;
  });
  // A result the wire format cannot carry becomes an error in the VM.
  assert.throws(() => sb.run("bad(); return 1"), /cannot encode a function/);
  // The sandbox is still usable after errors thrown in the VM.
  sb.defineFunction("twice", (x) => (x as number) * 2);
  assert.equal(sb.run("return twice(21)"), 42);
  sb.dispose();
});

test("async host functions with runAsync", async () => {
  const sb = await Sandbox.create({
    functions: {
      fetchLater: async (x) => {
        await new Promise((r) => setTimeout(r, 5));
        return `got ${x}`;
      },
    },
  });
  assert.equal(await sb.runAsync("return fetchLater(1) + ', ' + fetchLater(2)"), "got 1, got 2");
  assert.throws(() => sb.run("return fetchLater(1)"), /returned a promise; use runAsync/);
  sb.dispose();
});

test("async host functions run concurrently", async () => {
  let running = 0;
  let maxRunning = 0;
  const c = collect();
  const sb = await Sandbox.create({
    console: c.console,
    asyncFunctions: {
      tool: async (name, ms) => {
        running++;
        maxRunning = Math.max(maxRunning, running);
        await new Promise((r) => setTimeout(r, ms as number));
        running--;
        if (name === "bad") throw new RangeError("bad tool");
        return { name, ms };
      },
      // An async host function may answer synchronously too.
      now: () => 7,
    },
  });
  const r = await sb.runAsync(`
    const calls = [tool("a", 30), tool("b", 10), tool("c", 20)];
    calls[1].then((v) => console.log("b first", v.name));
    tool("bad", 5).catch((e) => console.log("caught", e.name, e.message));
    console.log("sync end", typeof calls[0].then);
    return Promise.all([Promise.all(calls), now()]).then((r) => r[0].map((v) => v.name).join() + r[1]);
  `);
  assert.equal(r, "a,b,c7");
  assert.equal(maxRunning, 4, "all four calls were outstanding at once");
  assert.deepEqual(c.lines, ["sync end function", "caught RangeError bad tool", "b first b"]);
  // A rejected result promise is an error; a sync run can serve async functions that
  // answer synchronously, but not ones that return promises.
  await assert.rejects(sb.runAsync("return tool('bad', 1)"), (e: unknown) => e instanceof SandboxError && e.kind === "threw" && /bad tool/.test(e.message));
  assert.equal(sb.run("return now().then((n) => n + 1)"), 8);
  assert.throws(() => sb.run("return tool('a', 1)"), /returned a promise; use runAsync/);
  assert.throws(() => sb.run("return new Promise(() => {})"), /never settles/);
  sb.dispose();
});

test("the clock and time zone come from the host", async () => {
  const sb = await Sandbox.create();
  const t = sb.run("return Date.now()") as number;
  assert.ok(Math.abs(t - Date.now()) < 5000, `Date.now() in the VM: ${t}`);
  const fixed = await Sandbox.create({ now: 1_646_177_400_000, timeZoneOffset: 120 });
  assert.equal(fixed.run("const d = new Date(); return d.toISOString() + ' ' + d.getHours()"), "2022-03-01T23:30:00.000Z 1");
  let tick = 0;
  const ticking = await Sandbox.create({ now: () => 1000 * ++tick, timeZoneOffset: 0 });
  assert.equal(ticking.run("return Date.now()"), 1000);
  assert.equal(ticking.run("return Date.now()"), 2000);
  for (const s of [sb, fixed, ticking]) s.dispose();
});

test("unhandled rejections", async () => {
  const c = collect();
  const src = "Promise.reject(new TypeError('lost')); Promise.reject(2).catch(() => {}); return 1";
  const warn = await Sandbox.create({ console: c.console });
  assert.equal(warn.run(src), 1);
  assert.deepEqual(c.lines, ["warn Possibly unhandled promise rejection: TypeError: lost"]);
  // A rejected result is the run's error, not also an unhandled rejection.
  assert.throws(() => warn.run("return Promise.reject(new RangeError('result'))"), /RangeError: result/);
  assert.equal(c.lines.length, 1);
  const thrower = await Sandbox.create({ unhandledRejections: "throw" });
  assert.throws(
    () => thrower.run(src),
    (e: unknown) => e instanceof SandboxError && e.kind === "threw" && e.message === "Possibly unhandled promise rejection: TypeError: lost",
  );
  assert.equal(thrower.run("return 'usable'"), "usable");
  const collector = await Sandbox.create({ unhandledRejections: "collect" });
  collector.run("Promise.reject(3); Promise.reject('four')");
  assert.deepEqual(collector.takeUnhandledRejections(), [3, "four"]);
  assert.deepEqual(collector.takeUnhandledRejections(), []);
  for (const s of [warn, thrower, collector]) s.dispose();
});

test("syntax errors", async () => {
  const sb = await Sandbox.create();
  assert.throws(() => sb.run("return )"), (e: unknown) => e instanceof SandboxError && e.kind === "syntax");
  assert.equal(sb.run("return 'still fine'"), "still fine");
  sb.dispose();
});

test("typescript", async () => {
  const ts = await Sandbox.create({ typescript: true });
  assert.equal(ts.run("enum E { A = 2 } const x: number = E.A as number; return x * 21"), 42);
  ts.dispose();
  const js = await Sandbox.create();
  assert.throws(() => js.run("let x: number = 1"), (e: unknown) => e instanceof SandboxError && e.kind === "syntax");
  js.dispose();
});

test("snapshots", async () => {
  const c = collect();
  const sb = await Sandbox.create({ console: c.console, functions: { add: (a, b) => (a as number) + (b as number) } });
  sb.setGlobal("config", { greeting: "hello", n: 2 });
  const snap = sb.bytes();
  sb.dispose();
  const c2 = collect();
  const restored = await Sandbox.fromBytes(snap, { console: c2.console, functions: { add: (a, b) => (a as number) - (b as number) } });
  // The global survives; host functions bind by name to the new implementations.
  assert.equal(restored.run("console.log(config.greeting); return add(config.n, 5)"), -3);
  assert.deepEqual(c2.lines, ["hello"]);
  await assert.rejects(Sandbox.fromBytes(snap), /snapshot needs host function add/);
  await assert.rejects(Sandbox.fromBytes(new Uint8Array(20)), /not a sandbox snapshot/);
  // A damaged image is refused (the heap verifies images on load).
  const broken = snap.slice();
  const meta = new DataView(broken.buffer).getUint32(8, true);
  broken[12 + meta + 12] ^= 0xff; // Roots.VERSION
  await assert.rejects(Sandbox.fromBytes(broken, { functions: { add: () => 0 } }), /cannot restore the VM image/);
  restored.dispose();
});

test("out of memory is a clean error", async () => {
  const sb = await Sandbox.create({ memoryLimit: 2 << 20 });
  assert.throws(
    () => sb.run("let a = []; for (let i = 0; ; i++) a[i] = 'item ' + i; return a"),
    (e: unknown) => e instanceof SandboxError && e.kind === "threw" && /out of memory/i.test(e.message),
  );
  sb.dispose();
});

test("fuel budget", async () => {
  const sb = await Sandbox.create({ fuelSlice: 10_000, maxFuel: 200_000 });
  sb.setGlobal("kept", 41);
  assert.throws(() => sb.run("for (;;) {}"), (e: unknown) => e instanceof SandboxError && e.kind === "fuel");
  // The call was abandoned (vm_abort): the sandbox is usable, its globals intact.
  assert.equal(sb.run("return kept + 1"), 42);
  assert.throws(() => sb.run("let a = 0; for (;;) a++"), (e: unknown) => e instanceof SandboxError && e.kind === "fuel");
  assert.equal(sb.run("return 'again'"), "again");
  sb.dispose();
});

test("an abandoned async run leaves the sandbox usable", async () => {
  const sb = await Sandbox.create({
    fuelSlice: 10_000,
    maxFuel: 200_000,
    functions: { later: async () => 1 },
  });
  await assert.rejects(sb.runAsync("later(); for (;;) {}"), (e: unknown) => e instanceof SandboxError && e.kind === "fuel");
  assert.equal(await sb.runAsync("return later() + 1"), 2);
  sb.dispose();
});

test("a trap is a clean error and kills the sandbox", async () => {
  const sb = await Sandbox.create();
  assert.throws(
    () => sb.debugTrap(),
    (e: unknown) => e instanceof SandboxError && e.kind === "trap" && /djs_debug_panic called/.test(e.message),
  );
  assert.throws(() => sb.run("return 1"), /unusable: trapped/);
  // Other sandboxes have their own instance.
  const other = await Sandbox.create();
  assert.equal(other.run("return 7"), 7);
  other.dispose();
});
