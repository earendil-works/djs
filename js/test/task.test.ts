// Tasks (0189, D2770, D2771): runs the sandbox drives, suspended into a dump while they wait
// on host calls and continued from it. Async host functions declared `{ call, save,
// resume }` carry their state through the dump. In both packagings (D2635), as in
// dumpresume.test.ts.

import { test } from "node:test";
import assert from "node:assert/strict";
import { type AsyncHostFunction, type HostCall, loadModule, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

interface Packaging {
  module: WebAssembly.Module;
  dumpModule?: WebAssembly.Module;
  frozenBuiltins: boolean;
}

async function packagings(): Promise<[string, Packaging][]> {
  const module = await loadModule();
  const out: [string, Packaging][] = [
    // Dumps need frozen builtins (D2836).
    ["A", { module, frozenBuiltins: true }],
    ["B (image transport)", { module, dumpModule: module, frozenBuiltins: true }],
  ];
  const nodump = process.env.DJS_WASM_NODUMP;
  if (nodump) out.push(["B (djs.wasm + djs-dump.wasm)", { module: await loadModule(nodump), dumpModule: module, frozenBuiltins: true }]);
  return out;
}

/** A promise the test resolves by hand. */
function later<T = unknown>(): { promise: Promise<T>; resolve(v: T): void } {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

/** Host functions whose calls the test answers by hand, and what the hooks saw. */
function host(p: Packaging) {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const pending = new Map<string, { resolve(v: unknown): void }>();
  const seen: { calls: HostCall[]; resumed: [unknown, HostCall][]; saved: HostCall[] } = { calls: [], resumed: [], saved: [] };
  const wait = (key: string) => {
    const l = later();
    pending.set(key, l);
    return l.promise;
  };
  // `job(n)`: state { n, step } set at the call; `save` adds the save time's step.
  const job: AsyncHostFunction<{ n: number; step: string }> = {
    call(n) {
      seen.calls.push(this);
      this.state = { n: Number(n), step: "started" };
      return wait(`job ${n}`);
    },
    save(call) {
      seen.saved.push(call);
      return { ...call.state!, step: call.state!.step + ", saved" };
    },
    resume(state, call) {
      seen.resumed.push([state, call]);
      return wait(`resumed ${state.n}`);
    },
  };
  const opts: SandboxOptions = {
    ...p,
    console: { log, info: log, warn: log, error: log },
    asyncFunctions: {
      "tools.job": job,
      // No resume: made again after a load.
      "tools.again": { call: (x) => wait(`again ${x}`) },
      // No hooks: rejected after a load.
      "tools.plain": (x) => wait(`plain ${x}`),
    },
  };
  /** Resolves call `key` once it has been made. */
  const answer = async (key: string, v: unknown) => {
    for (let i = 0; i < 1000 && !pending.has(key); i++) await new Promise((r) => setTimeout(r, 1));
    const p = pending.get(key);
    assert.ok(p, `no call ${key}; open: ${[...pending.keys()].join(", ")}`);
    pending.delete(key);
    p.resolve(v);
  };
  return { opts, lines, seen, answer };
}

const CODE = `
  console.log("start", n);
  const [a, b, c] = await Promise.all([tools.job(n), tools.again("x"), tools.plain("y").catch((e) => "caught " + e.message)]);
  console.log("got", a, b, c);
  return [a, b, c, await tools.job(n + 1)];
`;

test("a task runs like runAsync, hooks or not", async (t) => {
  for (const [pk, p] of await packagings()) {
    await t.test(pk, async () => {
      const h = host(p);
      const sb = new Sandbox(h.opts);
      const task = sb.start(CODE, { n: 1 });
      assert.equal(task.sandbox, sb);
      await h.answer("job 1", "A");
      await h.answer("again x", "B");
      await h.answer("plain y", "C");
      await h.answer("job 2", "D");
      assert.deepEqual(await task.result, ["A", "B", "C", "D"]);
      assert.deepEqual(h.lines, ["start 1", "got A B C"]);
      assert.deepEqual(h.seen.calls.map((c) => [c.fn, c.args, c.state]), [
        ["tools.job", [1], { n: 1, step: "started" }],
        ["tools.job", [2], { n: 2, step: "started" }],
      ]);
      assert.equal(task.calls.length, 0);
      // The sandbox is free again.
      assert.equal(sb.run("return 1 + 1"), 2);
      sb.dispose();
    });
  }
});

test("suspend dumps the call states; resume continues every call by its hooks", async (t) => {
  for (const [pk, p] of await packagings()) {
    await t.test(pk, async () => {
      const h = host(p);
      const sb = new Sandbox(h.opts);
      const task = sb.start(CODE, { n: 1 });
      const dump = await task.suspend();
      assert.match(dump, /\$djs\.resume\(\d+, "tools\.job", \[1\], \{"n":1,"step":"started, saved"\}\)/);
      assert.match(dump, /\$djs\.resume\(\d+, "tools\.again", \["x"\]\);/);
      assert.match(dump, /\$djs\.resume\(\d+, "tools\.plain", \["y"\]\)/);
      await assert.rejects(task.result, (e: unknown) => e instanceof SandboxError && e.kind === "suspended");
      assert.ok(h.seen.calls[0].signal.aborted, "the suspended task stops waiting for its calls");
      assert.equal(h.seen.saved.length, 1);
      await assert.rejects(task.suspend(), /the task has ended/);
      // A late result of the old sandbox goes nowhere.
      await h.answer("job 1", "late");
      sb.dispose();

      // (The plain function's call is rejected at once.)
      const again = await Sandbox.resume(dump, h.opts);
      assert.deepEqual(again.calls.map((c) => [c.fn, c.args, c.state]), [
        ["tools.job", [1], { n: 1, step: "started, saved" }],
        ["tools.again", ["x"], undefined],
      ]);
      assert.deepEqual(h.seen.resumed.map(([s, c]) => [s, c.fn, c.state]), [[{ n: 1, step: "started, saved" }, "tools.job", { n: 1, step: "started, saved" }]]);
      await h.answer("resumed 1", "A2");
      await h.answer("again x", "B2");
      // Dumped again while waiting on job(2): a fresh call's state, saved.
      const second = await again.suspend();
      assert.match(second, /\$djs\.resume\(\d+, "tools\.job", \[2\], \{"n":2,"step":"started, saved"\}\)/);
      again.sandbox.dispose();
      const third = await Sandbox.resume(second, h.opts);
      await h.answer("resumed 2", "D3");
      assert.deepEqual(await third.result, [
        "A2",
        "B2",
        "caught tools.plain: the call was open when the run was dumped, and the host function has no hooks to continue it",
        "D3",
      ]);
      assert.deepEqual(h.lines, ["start 1", "got A2 B2 caught tools.plain: the call was open when the run was dumped, and the host function has no hooks to continue it"]);
      third.sandbox.dispose();
    });
  }
});

test("checkpoint dumps and goes on; a dump that fails leaves the task running", async () => {
  const h = host({ module: await loadModule(), frozenBuiltins: true });
  const sb = new Sandbox(h.opts);
  const task = sb.start("const w = new WeakMap(); const r = await tools.job(1); return [r, await tools.job(2)];");
  // A weak collection cannot be written: the task goes on.
  await assert.rejects(task.checkpoint(), (e: unknown) => e instanceof SandboxError && /weak collection/.test(e.message));
  await h.answer("job 1", "A");
  // One task at a time.
  assert.throws(() => sb.start("return 1"), /a run is in progress/);
  await h.answer("job 2", "B");
  assert.deepEqual(await task.result, ["A", "B"]);

  const t2 = sb.start("const r = await tools.job(5); return r + 1;");
  const cp = await t2.checkpoint();
  assert.match(cp, /"tools\.job", \[5\], \{"n":5/);
  await h.answer("job 5", 41);
  assert.equal(await t2.result, 42);
  // The checkpoint resumes on its own.
  const t3 = await Sandbox.resume(cp, h.opts);
  await h.answer("resumed 5", 1);
  assert.equal(await t3.result, 2);
  // A suspend asked for after the end rejects.
  await assert.rejects(t2.suspend(), /the task has ended/);
  sb.dispose();
});

test("states must be JSON data; save errors reject the dump only", async () => {
  const module = await loadModule();
  const sb = new Sandbox({
    module,
    frozenBuiltins: true,
    asyncFunctions: {
      odd: {
        call() {
          this.state = { big: 1n };
          return new Promise(() => {});
        },
      },
    },
  });
  const t1 = sb.start("return await odd();");
  await assert.rejects(t1.suspend(), /the state of call \d+ is not JSON data/);
  sb.dispose();
  await assert.rejects(t1.result, (e: unknown) => e instanceof SandboxError && /disposed/.test(e.message));
  const sb2 = new Sandbox({ module, frozenBuiltins: true, asyncFunctions: { thrower: { call: () => new Promise(() => {}), save: () => { throw new Error("no state"); } } } });
  const t2 = sb2.start("return await thrower();");
  await assert.rejects(t2.suspend(), /no state/);
  sb2.dispose();
  await assert.rejects(t2.result, /disposed/);
});

test("suspend before an idle point waits for it; a run that ends first rejects it", async () => {
  const module = await loadModule();
  const sb = new Sandbox({ module, asyncFunctions: { now: () => 7 } });
  const t1 = sb.start("return (await now()) + 1;");
  const s = t1.suspend();
  assert.equal(await t1.result, 8);
  await assert.rejects(s, /the task ended before an idle point/);
  // A sync result is not an idle point either: the task ends.
  const t2 = sb.start("let s = 0; for (let i = 0; i < 3; i++) s += await now(); return s;");
  assert.equal(await t2.result, 21);
  // A script error rejects the result as threw.
  const t3 = sb.start("await now(); throw new TypeError('bad');");
  await assert.rejects(t3.result, (e: unknown) => e instanceof SandboxError && e.kind === "threw" && /TypeError: bad/.test(e.message));
  assert.throws(() => sb.start("return ("), (e: unknown) => e instanceof SandboxError && e.kind === "syntax");
  sb.dispose();
});

test("host-driven runs dump states and load them back", async () => {
  const module = await loadModule();
  const opts = { module, frozenBuiltins: true, asyncFunctions: { "tools.sleep": () => 0 } };
  const sb = new Sandbox(opts);
  const run = sb.begin("return await tools.sleep(5) + await tools.sleep(6);");
  const id = run.calls[0].id;
  const text = run.dump({ [id]: { due: 123, note: "é\n\u2028" } });
  assert.match(text, new RegExp(`\\$djs\\.resume\\(${id}, "tools\\.sleep", \\[5\\], \\{"due":123,"note":"é\\\\n\u2028"\\}\\)`));
  assert.throws(() => run.dump({ [id + 1]: 1 }), /a state for a call that is not open/);
  assert.throws(() => run.dump({ [id]: { __proto__: null, ["__proto__"]: 1 } }), /__proto__/);
  const again = new Sandbox(opts).loadRun(text);
  assert.deepEqual(again.calls, [{ id, fn: "tools.sleep", args: [5], state: { due: 123, note: "é\n\u2028" } }]);
  again.settle(id, { ok: true, value: 1 });
  await again.advance();
  again.settle(again.calls[0].id, { ok: true, value: 2 });
  await again.advance();
  assert.equal(again.value, 3);
  sb.dispose();
});
