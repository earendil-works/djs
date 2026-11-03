// Sandbox tests against a djs-wasm build: $DJS_WASM, or the `wasm` profile build
// (scripts/gate.sh builds a debug one and sets DJS_WASM).

import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Sandbox, SandboxError, defaultWasmPath, loadModule, type JournalEntry } from "../src/sandbox.ts";

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

test("heapView walks the arena; collect frees garbage", async () => {
  const sb = await Sandbox.create();
  // Every cell starts with its kind (u8) and its size (u32 at +4), back to back.
  const walk = () => {
    const h = sb.heapView();
    const dv = new DataView(h.buffer, h.byteOffset, h.byteLength);
    let cells = 0;
    let off = 0;
    while (off + 8 <= h.length) {
      const size = dv.getUint32(off + 4, true);
      if (size === 0) break;
      off += size;
      cells++;
    }
    assert.equal(off, h.length);
    return cells;
  };
  const before = walk();
  sb.run("globalThis.keep = []; for (let i = 0; i < 1000; i++) { const g = { i }; if (i % 100 === 0) keep.push(g); }");
  assert.ok(walk() > before + 900);
  assert.equal(sb.collect(), true);
  assert.ok(walk() < before + 100);
  assert.equal(sb.run("return keep.length"), 10);
  sb.dispose();
});

test("the synchronous constructor", async () => {
  const module = await loadModule();
  const sb = new Sandbox({ module });
  assert.equal(sb.run("return 'sync'"), "sync");
  // After loadModule() of the default location, the module is optional.
  const sb2 = new Sandbox();
  assert.equal(sb2.run("return 2"), 2);
  for (const s of [sb, sb2]) s.dispose();
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

test("async results land at the same point for any fuel slice (D1670)", async () => {
  // A result the host passes back between two fuel slices waits in the VM's inbox until
  // the job queue is empty, so the slice size does not change the order.
  const code = `
    tool(1).then((v) => console.log("a", v));
    let x = 0;
    for (let i = 0; i < 20000; i++) x += i;
    Promise.resolve().then(() => { console.log("b1"); Promise.resolve().then(() => console.log("b2")); });
    await null; await null; await null;
  `;
  for (const fuelSlice of [1_000_000, 1000, 50]) {
    for (const tool of [(x: unknown) => (x as number) * 2, async (x: unknown) => (x as number) * 2]) {
      const c = collect();
      const sb = await Sandbox.create({ console: c.console, fuelSlice, now: 0, asyncFunctions: { tool } });
      await sb.runAsync(code);
      assert.deepEqual(c.lines, ["b1", "b2", "a 2"], `fuelSlice ${fuelSlice}`);
      sb.dispose();
    }
  }
});

test("deterministic mode and random seeds (D1680)", async () => {
  const code = `
    const out = [Date.now(), new Date(0).getHours(), Math.random()];
    let x = 0;
    for (let i = 0; i < 20000; i++) x += i;
    out.push(Date.now());
    const r = await tool(1);
    out.push(Date.now(), r);
    const w = new WeakRef({ a: 1 });
    await tool(2);
    out.push(Date.now(), typeof w.deref(), Math.random());
    return out;
  `;
  const once = async (fuelSlice: number) => {
    let t = 5000;
    const sb = await Sandbox.create({
      fuelSlice,
      timeZoneOffset: 120,
      now: () => (t += 1000),
      asyncFunctions: { tool: async (x: unknown) => x },
      deterministic: { startedAt: 1000, seed: 42 },
    });
    const r = (await sb.runAsync(code)) as unknown[];
    sb.dispose();
    return r;
  };
  const a = await once(1_000_000);
  // The clock starts at startedAt, does not move with fuel slices or sync code, and moves
  // to `now` when a result is delivered; the time zone is UTC; weak refs are strong.
  assert.deepEqual([a[0], a[1], a[3], a[4], a[5], a[6], a[7]], [1000, 0, 1000, 6000, 1, 7000, "object"]);
  assert.deepEqual(await once(1000), a);
  assert.deepEqual(await once(97), a);
  // Random seeds: given ones repeat, the default comes from entropy.
  const random = async (opts: { randomSeed?: number | bigint }) => {
    const sb = await Sandbox.create(opts);
    const r = sb.run("return [Math.random(), Math.random()]");
    sb.dispose();
    return r;
  };
  assert.deepEqual(await random({ randomSeed: 7n }), await random({ randomSeed: 7 }));
  assert.notDeepEqual(await random({}), await random({}));
  // Weak refs can also be absent.
  const sb = await Sandbox.create({ weakRefs: "absent" });
  assert.equal(sb.run("return typeof WeakRef + typeof FinalizationRegistry"), "undefinedundefined");
  sb.dispose();
});

test("frozenBuiltins (D2304): builtins behave as frozen, with the override fix", async () => {
  const code = `
    const o = {}; o.toString = () => "own";
    let r; try { Math.PI = 3; r = "assigned"; } catch (e) { r = e.constructor.name; }
    return [Object.isFrozen(Array.prototype), String(o), r, Object.isExtensible(globalThis)].join();
  `;
  const sb = await Sandbox.create({ frozenBuiltins: true });
  assert.equal(sb.run(code), "true,own,TypeError,true");
  sb.dispose();
  const plain = await Sandbox.create({});
  assert.equal(plain.run(code), "false,own,TypeError,true");
  plain.dispose();
});

test("journals: record, replay, divergence (D1730)", async () => {
  const code = `
    const a = tool("a", 30), b = tool("b", 10);
    const r = [await b, sync(1)];
    let x = 0;
    for (let i = 0; i < 20000; i++) x += i;
    r.push(await a, Date.now(), Math.random());
    try { await tool("bad", 1); } catch (e) { r.push(e.name + ": " + e.message); }
    globalThis.state = (globalThis.state ?? 0) + r.length;
    return r;
  `;
  // While replaying, the host functions must not be called.
  let replaying = false;
  const host = () => {
    if (replaying) throw new Error("replay called a host function");
  };
  // Each sandbox has its own clock, so equal runs see equal times.
  const options = (fuelSlice: number) => {
    let t = 1_000_000;
    return {
      fuelSlice,
      console: collect().console,
      deterministic: { startedAt: 1000, seed: 3 },
      now: () => (t += 1000),
      functions: { sync: (n: unknown) => (host(), (n as number) + 1) },
      asyncFunctions: {
        tool: async (name: unknown, ms: unknown) => {
          host();
          await new Promise((r) => setTimeout(r, ms as number));
          if (name === "bad") throw new RangeError("bad tool");
          return name;
        },
      },
    };
  };
  // A first run, then the recorded run.
  const sb = await Sandbox.create(options(1_000_000));
  await sb.runAsync(code);
  const journal: JournalEntry[] = [];
  const result = await sb.runAsync(code, {}, { journal });
  assert.deepEqual(journal[0].type, "run");
  assert.deepEqual(journal.at(-1)!.type, "end");
  assert.ok(journal.some((e) => e.type === "check"));
  assert.ok(journal.some((e) => e.type === "sync"));
  // Replayed in a sandbox in the same state (made the same way, given the same first run),
  // at another fuel slice: the same result, calls and states, without calling the host
  // functions.
  const prepared = async () => {
    replaying = false;
    const s = await Sandbox.create(options(97));
    await s.runAsync(code);
    replaying = true;
    return s;
  };
  assert.deepEqual(await (await prepared()).replay(journal), result);
  // A different state before the run, a changed result, or other calls diverge.
  const fresh = await Sandbox.create(options(97));
  await assert.rejects(fresh.replay(journal), (e: unknown) => e instanceof SandboxError && e.kind === "diverged");
  const changed = journal.map((e) => (e.type === "async" && e.value === "a" ? { ...e, value: "A" } : e));
  await assert.rejects((await prepared()).replay(changed), /diverged/);
  const otherCall = journal.map((e) => (e.type === "sync" ? { ...e, args: [2] } : e));
  await assert.rejects(
    (await prepared()).replay(otherCall),
    /the script called sync\(\[1\]\), the recording sync\(\[2\]\)/,
  );
  // Recording needs a deterministic sandbox.
  const plain = await Sandbox.create();
  assert.throws(() => plain.run("return 1", {}, { journal: [] }), /deterministic/);
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

test("legacy decorators", async () => {
  const src =
    "const log = []; const d = (t, k, i) => { log.push(typeof t + ' ' + k + ' ' + i); };" +
    "Reflect.metadata = (k, v) => () => { log.push(k); };" +
    "class C { m(@d a: number) {} } return log.join()";
  const legacy = await Sandbox.create({ typescript: true, experimentalDecorators: true });
  assert.equal(legacy.run(src), "object m 0");
  legacy.dispose();
  const meta = await Sandbox.create({ typescript: true, emitDecoratorMetadata: true });
  assert.equal(meta.run(src), "design:returntype,design:paramtypes,design:type,object m 0");
  meta.dispose();
  const standard = await Sandbox.create({ typescript: true });
  assert.throws(() => standard.run(src), (e: unknown) => e instanceof SandboxError && e.kind === "syntax");
  standard.dispose();
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
    // Debug modules report the panic message; the shipped module a fixed one (D2332).
    (e: unknown) =>
      e instanceof SandboxError && e.kind === "trap" && /djs_debug_panic called|djs-wasm panicked/.test(e.message),
  );
  assert.throws(() => sb.run("return 1"), /unusable: trapped/);
  // Other sandboxes have their own instance.
  const other = await Sandbox.create();
  assert.equal(other.run("return 7"), 7);
  other.dispose();
});

test("large host strings land in the heap in place when ASCII (D2551), others through the wire", async () => {
  const ascii = '{"k": "' + "abc".repeat(200_000) + '"}';
  const cases = [ascii, "x".repeat(1023), "y".repeat(1024), "é".repeat(5000), "a".repeat(3000) + "\u2014", "b".repeat(2000) + "\ud800"];
  const sb = await Sandbox.create({ functions: { get: (i: unknown) => cases[i as number] }, asyncFunctions: { later: async (i: unknown) => cases[i as number] } });
  for (let i = 0; i < cases.length; i++) {
    const s = cases[i];
    const r = (await sb.runAsync(`const a = get(${i}), b = await later(${i}); return [a === b, a.length, a.charCodeAt(a.length - 1), a.slice(1, 4), ({[a]: 7})[b]]`, { p: s })) as unknown[];
    assert.deepEqual(r, [true, s.length, s.charCodeAt(s.length - 1), s.slice(1, 4), 7], `case ${i}`);
    assert.equal(sb.run("return p === p.slice(0) && p.length", { p: s }), s.length);
  }
  assert.equal(sb.run(`return JSON.parse(get(0)).k.length`), 600_000);
  sb.dispose();
});

test("slices of a long text reach the host and survive dumps (D2542)", async () => {
  const text = Array.from({ length: 200 }, (_, i) => `line ${i} `.padEnd(60, "x")).join("\n");
  const sb = await Sandbox.create();
  const seen: unknown[] = [];
  sb.defineFunction("see", (v) => {
    seen.push(v);
    return null;
  });
  sb.run(`globalThis.lines = text.split("\\n"); globalThis.doc = JSON.parse(json); see([lines[3], doc.a]);`, {
    text,
    json: JSON.stringify({ a: text.slice(0, 300), b: [1, 2] }),
  });
  assert.deepEqual(seen, [[text.split("\n")[3], text.slice(0, 300)]]);
  const restored = await Sandbox.create({ functions: { see: () => null } });
  restored.load(sb.dump());
  assert.deepEqual(restored.run("return [lines.length, lines[199], doc.a.length, lines[5].slice(2, 40)]"), [
    200,
    text.split("\n")[199],
    300,
    text.split("\n")[5].slice(2, 40),
  ]);
  sb.dispose();
  restored.dispose();
});
