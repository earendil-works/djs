// The pi codemode adapter (js/pi/vm.ts) running pi's real prelude (tests/pi, copied from
// pi-private) through a small host like pi's in-thread launcher: tool calls, output,
// store/load, errors, interrupts, determinism and snapshots. pi's own test suite runs
// against the adapter with `node js/pi/pi-tests.ts` (docs/host.md "pi codemode").

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule } from "../src/sandbox.ts";
import { resumeVm, startVm, type HostToWorkerMessage, type VmSession, type WorkerToHostMessage } from "../pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

type Tool = (args: unknown) => unknown;

interface Options {
  tools?: Record<string, Tool>;
  globals?: Record<string, Tool>;
  store?: Record<string, unknown>;
  deterministic?: { startedAt: number; seed: number };
  /** Deterministic runs: the clock of result n is startedAt + 1000 * n. */
  memoryLimitBytes?: number;
  interruptBudget?: number;
  /** Called with the session after every message, outside the VM. */
  onMessage?: (message: WorkerToHostMessage, session: VmSession) => void;
  /** Leave calls to these tools unanswered. */
  hold?: string[];
}

interface Outcome {
  messages: WorkerToHostMessage[];
  output: unknown[];
  calls: { name: string; args: unknown }[];
  end: Extract<WorkerToHostMessage, { type: "done" } | { type: "crash" }>;
  session: VmSession;
}

const wasm = await loadModule();

/** Starts `code` and answers its calls like pi's host: in a later task, never inside the VM. */
function start(code: string, options: Options = {}) {
  const tools = options.tools ?? {};
  const globals = options.globals ?? {};
  const messages: WorkerToHostMessage[] = [];
  let session: VmSession | undefined;
  let seq = 0;
  let polls = 0;
  let settle!: (end: Outcome["end"]) => void;
  const ended = new Promise<Outcome["end"]>((resolve) => (settle = resolve));
  const answer = (message: Extract<WorkerToHostMessage, { type: "call" }>) => {
    const table = message.target === "tool" ? tools : globals;
    const args = message.args === undefined ? undefined : JSON.parse(message.args);
    new Promise((resolve) => resolve(table[message.name](args))).then(
      (value) => deliver({ type: "result", id: message.id, ok: true, payload: value === undefined ? undefined : JSON.stringify(value) }),
      (error: Error) => deliver({ type: "result", id: message.id, ok: false, payload: error.message }),
    );
  };
  const deliver = (message: HostToWorkerMessage) => {
    if (options.deterministic) {
      seq++;
      message = { ...message, seq, now: options.deterministic.startedAt + 1000 * seq };
    }
    session!.deliver(message);
  };
  const ready = startVm({
    wasm,
    prelude: PRELUDE_SOURCE,
    interruptBudget: options.interruptBudget,
    data: {
      code,
      tools: Object.keys(tools).map((name) => ({ name, jsName: name, description: `the ${name} tool` })),
      globals: Object.keys(globals).map((name) => ({ name, spread: false })),
      memoryLimitBytes: options.memoryLimitBytes,
      store: Object.fromEntries(Object.entries(options.store ?? {}).map(([k, v]) => [k, JSON.stringify(v)])),
      deterministic: options.deterministic,
    },
    post: (message) => {
      messages.push(message);
      setTimeout(() => {
        if (message.type === "done" || message.type === "crash") settle(message);
        if (message.type === "call" && !options.hold?.includes(message.name)) answer(message);
        options.onMessage?.(message, session!);
      });
    },
    shouldInterrupt: () => options.interruptBudget !== undefined && ++polls > options.interruptBudget,
  });
  return {
    messages,
    ready: ready.then((s) => (session = s)),
    async outcome(): Promise<Outcome> {
      await ready;
      const end = await ended;
      return {
        messages,
        end,
        session: session!,
        output: messages.flatMap((m) => (m.type === "output" ? [m.item] : [])),
        calls: messages.flatMap((m) =>
          m.type === "call" ? [{ name: m.name, args: m.args === undefined ? undefined : JSON.parse(m.args) }] : [],
        ),
      };
    },
  };
}

async function run(code: string, options: Options = {}): Promise<Outcome> {
  const outcome = await start(code, options).outcome();
  outcome.session.dispose();
  return outcome;
}

function value(outcome: Outcome): unknown {
  assert.equal(outcome.end.type, "done", JSON.stringify(outcome.end));
  const end = outcome.end as Extract<WorkerToHostMessage, { type: "done" }>;
  assert.ok(end.ok, JSON.stringify(end));
  return end.value === undefined ? undefined : JSON.parse(end.value);
}

function error(outcome: Outcome): { name?: string; message: string; stack?: string } {
  assert.equal(outcome.end.type, "done", JSON.stringify(outcome.end));
  const end = outcome.end as Extract<WorkerToHostMessage, { type: "done"; ok: false }>;
  assert.equal(end.ok, false);
  return JSON.parse(end.error);
}

const echo: Tool = (args) => args;
const later = (ms: number, v: unknown) => new Promise((resolve) => setTimeout(() => resolve(v), ms));
const PNG = "iVBORw0KGgo=";

test("pi: the return value, top-level await and output in order", async () => {
  assert.deepEqual(value(await run("return { a: 1, b: [true, 'x'] }")), { a: 1, b: [true, "x"] });
  assert.equal(value(await run("")), undefined);
  const outcome = await run(`
    const x = await Promise.resolve(41);
    console.log("hello", 1, { a: 1 });
    text({ json: true });
    text(7);
    image("data:image/png;base64,${PNG}");
    console.error(new Error("bad"));
    return x + 1;
  `);
  assert.equal(value(outcome), 42);
  assert.deepEqual(outcome.output.slice(0, 4), [
    { type: "text", text: 'hello 1 {"a":1}', console: true },
    { type: "text", text: '{"json":true}' },
    { type: "text", text: "7" },
    { type: "image", data: PNG, mimeType: "image/png" },
  ]);
  const last = outcome.output[4] as { text: string };
  assert.match(last.text, /^Error: bad\n {4}at .*codemode\.js:7/);
});

test("pi: concurrent tool calls are all issued before the first result", async () => {
  const outcome = await run(
    `
    const [a, b, c] = await Promise.all([tools.slow(1), tools.fast(2), tools.slow(3)]);
    return [a, b, c, ALL_TOOLS.map((tool) => tool.name)];
  `,
    { tools: { slow: (n) => later(20, n), fast: (n) => later(1, n) } },
  );
  assert.deepEqual(value(outcome), [1, 2, 3, ["slow", "fast"]]);
  assert.deepEqual(
    outcome.messages.slice(0, 3).map((m) => m.type === "call" && m.args),
    ["1", "2", "3"],
  );
});

test("pi: tool errors are catchable; unknown tools name close matches", async () => {
  const outcome = await run(
    `
    try { await tools.fail({}); } catch (error) { text(error.message); }
    try { tools.ech(1); } catch (error) { text(error.name + ": " + error.message); }
    return "ok";
  `,
    {
      tools: {
        echo,
        fail: () => {
          throw new Error("no such file");
        },
      },
    },
  );
  assert.equal(value(outcome), "ok");
  assert.deepEqual(outcome.output[0], { type: "text", text: "no such file" });
  assert.match((outcome.output[1] as { text: string }).text, /^TypeError: tools\.ech does not exist\. Did you mean tools\.echo\?/);
});

test("pi: globals, including namespaces with spread arguments", async () => {
  const outcome = await run("return [await add(1), await fs.read('a')]", {
    globals: { add: (n) => (n as number) + 1, "fs.read": (args) => `read ${JSON.stringify(args)}` },
  });
  assert.deepEqual(value(outcome), [2, 'read "a"']);
});

test("pi: store and load", async () => {
  const outcome = await run(
    `
    const before = load("count");
    store("count", before + 1);
    store("gone", undefined);
    const copy = load("list"); copy.push(4);
    return [before, load("count"), load("list"), load("missing")];
  `,
    { store: { count: 1, list: [1, 2, 3], gone: true } },
  );
  assert.deepEqual(value(outcome), [1, 2, [1, 2, 3], null]);
  const end = outcome.end as Extract<WorkerToHostMessage, { type: "done"; ok: true }>;
  assert.deepEqual(JSON.parse(end.writes), [["count", "2"], ["gone"]]);
  const big = await run('store("k", "x".repeat(300 * 1024))');
  assert.equal(error(big).name, "RangeError");
});

test("pi: script errors with the script's line numbers", async () => {
  const thrown = error(await run("const a = 1;\nthrow new TypeError('boom ' + a)"));
  assert.equal(thrown.name, "TypeError");
  assert.equal(thrown.message, "boom 1");
  assert.match(thrown.stack!, /^TypeError: boom 1\n {4}at .*codemode\.js:2/);
  assert.doesNotMatch(thrown.stack!, /codemode-prelude/);
  const syntax = error(await run("const a = 1;\nconst b = ;\nreturn a"));
  assert.equal(syntax.name, "SyntaxError");
  assert.match(syntax.stack!, /codemode\.js:2/);
  assert.deepEqual(error(await run("throw { code: 7 }")), { message: '{"code":7}' });
  assert.equal(error(await run("return 10n")).name, "TypeError");
  const stalled = error(await run("await new Promise(() => {})"));
  assert.match(stalled.message, /can never settle/);
});

test("pi: exit() ends the script with its output and store writes", async () => {
  const outcome = await run('text("before"); store("k", 1); await tools.echo(1); try { exit(); } catch {} text("after")', {
    tools: { echo },
  });
  assert.equal(value(outcome), undefined);
  assert.deepEqual(outcome.output, [{ type: "text", text: "before" }]);
});

test("pi: an interrupt budget stops loops, including microtask loops", async () => {
  for (const code of ["while (true) {}", "for (;;) await null", "let n = 0; for (;;) { try { n++; } catch {} }"]) {
    const outcome = await run(code, { interruptBudget: 300 });
    assert.deepEqual(outcome.end, { type: "crash", message: "Error: Execution interrupted" }, code);
  }
  // Work below the budget is not interrupted (the prelude does not count, D1660).
  assert.equal(value(await run("let n = 0; for (let i = 0; i < 1000; i++) n += i; return n", { interruptBudget: 300 })), 499500);
});

test("pi: a budget means the same on cold and warm starts (prelude images)", async () => {
  // A start that reuses a prelude image skips the prelude, so the prelude must not use up
  // interrupt polls (D1660): otherwise a replay after a restart (cold) could time out where
  // the original run (warm) did not.
  const code = "let s = 0; for (let i = 0; i < 100000; i++) s += i; const r = await tools.echo(s); return r";
  // A store no other test uses, so the first start below really is cold.
  const store = { "0066": JSON.stringify(Math.random()) };
  const once = async (reuseImages: boolean, budget: number) => {
    let polls = 0;
    let session: VmSession | undefined;
    const ended = new Promise<WorkerToHostMessage>((resolve) => {
      startVm({
        wasm,
        prelude: PRELUDE_SOURCE,
        reuseImages,
        interruptBudget: budget,
        data: {
          code,
          tools: [{ name: "echo", jsName: "echo", description: "" }],
          globals: [],
          memoryLimitBytes: undefined,
          store,
          deterministic: { startedAt: STARTED_AT, seed: 3 },
        },
        post: (message) =>
          setTimeout(() => {
            if (message.type === "done" || message.type === "crash") resolve(message);
            if (message.type === "call") session!.deliver({ type: "result", id: message.id, ok: true, payload: message.args, seq: 1, now: STARTED_AT });
          }),
        shouldInterrupt: () => ++polls > budget,
      }).then((s) => (session = s));
    });
    const end = await ended;
    session!.dispose();
    return { polls, end: end.type === "done" && end.ok ? end.value : end.type };
  };
  const cold = await once(false, Infinity);
  const filling = await once(true, Infinity);
  const warm = await once(true, Infinity);
  assert.equal(cold.end, "4999950000");
  assert.ok(cold.polls > 10, `${cold.polls} polls`);
  assert.deepEqual([filling, warm], [cold, cold]);
  // At the exact budget all three finish; one poll less, all three are interrupted.
  for (const reuse of [false, true, true]) {
    assert.equal((await once(reuse, cold.polls)).end, "4999950000");
    assert.equal((await once(reuse, cold.polls - 1)).end, "crash");
  }
});

test("pi: the memory limit", async () => {
  const outcome = await run("const a = []; for (;;) a.push({ a: 'x'.repeat(1000) + a.length })", {
    memoryLimitBytes: 8 << 20,
  });
  // The script's own error: the engine collects before the handler runs and opens its
  // reserve below the cap, so the prelude can report it (D1600).
  assert.match(error(outcome).message, /out of memory/);
  // At other limits too, and when the script catches it and returns.
  for (const mb of [2, 5, 12]) {
    const fill = "const a = []; for (;;) a.push({ a: 'x'.repeat(1000) + a.length })";
    assert.match(error(await run(fill, { memoryLimitBytes: mb << 20 })).message, /out of memory/, `${mb} MiB`);
    const caught = await run(`const a = []; try { ${fill} } catch (e) { a.length = 0; return String(e) }`, {
      memoryLimitBytes: mb << 20,
    });
    assert.equal(value(caught), "InternalError: out of memory", `${mb} MiB`);
  }
  await assert.rejects(startVm({
    wasm,
    prelude: PRELUDE_SOURCE,
    data: { code: "", tools: [], globals: [], memoryLimitBytes: 1024, store: {} },
    post: () => {},
    shouldInterrupt: () => false,
  }));
});

test("pi: host globals quickjs-wasi provides", async () => {
  const outcome = await run(`
    return [btoa("hello"), atob("aGVsbG8="), typeof performance.now(), await new Promise((r) => queueMicrotask(() => r(7))),
      Object.isFrozen(performance)];
  `);
  assert.deepEqual(value(outcome), ["aGVsbG8=", "hello", "number", 7, true]);
});

const STARTED_AT = Date.UTC(2026, 0, 15, 10, 0, 0);
const DETERMINISTIC_CODE = `
  const values = [Date.now(), performance.now(), Math.random(), Math.random(), new Date().toISOString()];
  const first = await tools.echo(values);
  text(JSON.stringify([Date.now(), performance.now(), Math.random()]));
  return await tools.echo(first.concat(Math.random(), new Date(0).getHours()));
`;

test("pi: deterministic runs repeat exactly, with QuickJS's random numbers", async () => {
  const det = { startedAt: STARTED_AT, seed: 1 };
  const one = await run(DETERMINISTIC_CODE, { tools: { echo }, deterministic: det });
  const two = await run(DETERMINISTIC_CODE, { tools: { echo }, deterministic: det });
  assert.deepEqual(one.messages, two.messages);
  const [now, elapsed, random, , iso] = one.calls[0].args as unknown[];
  assert.deepEqual([now, elapsed, iso], [STARTED_AT, 0, "2026-01-15T10:00:00.000Z"]);
  // The values pi's tests pin for quickjs-wasi (seed 1's first draw, seed 0's first two).
  assert.equal(random, 0.34227450605335275);
  assert.deepEqual(value(await run("return [Math.random(), Math.random()]", { deterministic: { startedAt: 0, seed: 0 } })), [
    0.6711372530266764, 0.7258461452833667,
  ]);
  // The clock moves with the results: result 1 arrives at startedAt + 1000.
  assert.deepEqual(JSON.parse((one.output[0] as { text: string }).text).slice(0, 2), [STARTED_AT + 1000, 1000]);
  const started = one.messages[0];
  assert.equal(started.type, "started");
  assert.match((started as { fingerprint: string }).fingerprint, /^djs [0-9a-f]{8}, prelude [0-9a-f]{8}, host [0-9a-f]{8}, deterministic 2/);
  assert.deepEqual(
    one.messages.filter((m) => m.type === "idle"),
    [
      { type: "idle", seq: 0, issued: 1 },
      { type: "idle", seq: 1, issued: 2 },
      // As in pi's vm.ts: after the result that ended the script too (the host ignores it).
      { type: "idle", seq: 2, issued: 2 },
    ],
  );
  const reseeded = await run(DETERMINISTIC_CODE, { tools: { echo }, deterministic: { ...det, seed: 2 } });
  assert.notEqual((reseeded.calls[0].args as number[])[2], random);
  // As pi's quickjs VM: no WeakRef or FinalizationRegistry in deterministic runs only.
  const weak = "return [typeof WeakRef, typeof FinalizationRegistry]";
  assert.deepEqual(value(await run(weak, { deterministic: det })), ["undefined", "undefined"]);
  assert.deepEqual(value(await run(weak)), ["function", "function"]);
  // Not deterministic: real randomness.
  const a = value(await run("return Math.random()"));
  const b = value(await run("return Math.random()"));
  assert.notEqual(a, b);
});

test("pi: a snapshot of a waiting session resumes in a new instance", async () => {
  // Reference: the deterministic run without a snapshot.
  const det = { startedAt: STARTED_AT, seed: 9 };
  const reference = await run(DETERMINISTIC_CODE, { tools: { echo }, deterministic: det });

  // The same run, but snapshotted while the first call waits; the original is disposed and
  // the call answered on the resumed session.
  const first = start(DETERMINISTIC_CODE, { tools: { echo }, deterministic: det, hold: ["echo"] });
  await first.ready;
  while (!first.messages.some((m) => m.type === "idle")) await later(1, null);
  const session = await first.ready;
  const bytes = session.snapshot();
  session.dispose();

  const messages: WorkerToHostMessage[] = first.messages.slice();
  let resumed!: VmSession;
  let seq = 0;
  const done = new Promise<void>((resolve) => {
    resumed = resumeVm(bytes, {
      wasm,
      post: (message) => {
        messages.push(message);
        if (message.type === "call") {
          setTimeout(() => {
            seq++;
            resumed.deliver({ type: "result", id: message.id, ok: true, payload: message.args, seq, now: STARTED_AT + 1000 * seq });
          });
        }
        if (message.type === "done" || message.type === "crash") resolve();
      },
      shouldInterrupt: () => false,
    });
  });
  const pending = first.messages.find((m) => m.type === "call")!;
  assert.equal(pending.type, "call");
  seq++;
  resumed.deliver({
    type: "result",
    id: (pending as { id: number }).id,
    ok: true,
    payload: (pending as { args: string }).args,
    seq,
    now: STARTED_AT + 1000 * seq,
  });
  await done;
  resumed.dispose();
  assert.deepEqual(messages, reference.messages);
});

test("pi: snapshot() collects first; continuing or resuming the session changes nothing", async () => {
  // D1890: a snapshot holds only live cells. The collection it makes is invisible to the
  // script: the same messages and the same number of interrupt polls (fuel) whether the
  // session is not snapshotted, snapshotted and continued, or snapshotted and resumed.
  const garbage = `let junk = 0;
for (let i = 0; i < 3000; i++) junk += JSON.stringify({ i, s: "x".repeat(i % 50) }).length;
const kept = Array.from({ length: 50 }, (_, i) => ({ i }));
const r = await tools.echo(junk);
let more = 0;
for (let i = 0; i < 3000; i++) more += JSON.stringify({ i, kept: kept.length }).length;
return [r, more];`;
  const once = async (code: string, mode: "none" | "continue" | "resume") => {
    let polls = 0;
    let size = 0;
    const messages: WorkerToHostMessage[] = [];
    let session!: VmSession;
    const hooks = {
      wasm,
      interruptBudget: Infinity,
      shouldInterrupt: () => ++polls < 0,
      post: (message: WorkerToHostMessage) => {
        messages.push(message);
        if (message.type !== "call") return;
        setTimeout(() => {
          if (mode !== "none") {
            const bytes = session.snapshot();
            size = bytes.length;
            if (mode === "resume") {
              session.dispose();
              session = resumeVm(bytes, hooks);
            }
          }
          session.deliver({ type: "result", id: message.id, ok: true, payload: message.args, seq: 1, now: STARTED_AT + 1000 });
        });
      },
    };
    const ended = new Promise<void>((resolve) => {
      const post = hooks.post;
      hooks.post = (message) => {
        post(message);
        if (message.type === "done" || message.type === "crash") setTimeout(resolve);
      };
    });
    session = await startVm({
      ...hooks,
      prelude: PRELUDE_SOURCE,
      data: {
        code,
        tools: [{ name: "echo", jsName: "echo", description: "" }],
        globals: [],
        memoryLimitBytes: undefined,
        store: {},
        deterministic: { startedAt: STARTED_AT, seed: 5 },
      },
    });
    await ended;
    session.dispose();
    return { messages, polls, size };
  };
  const plain = await once(garbage, "none");
  assert.ok(plain.messages.some((m) => m.type === "done" && m.ok), JSON.stringify(plain.messages));
  assert.ok(plain.polls > 10, `${plain.polls} polls`);
  const continued = await once(garbage, "continue");
  const resumed = await once(garbage, "resume");
  assert.deepEqual([continued.messages, continued.polls], [plain.messages, plain.polls]);
  assert.deepEqual([resumed.messages, resumed.polls], [plain.messages, plain.polls]);
  // The garbage of the first loop is not in the image: it is about as large as the image of
  // a script that made none (they differ by the source text and `kept`).
  const small = await once("return await tools.echo(1)", "continue");
  assert.ok(continued.size - small.size < 16 * 1024, `${continued.size} vs ${small.size} bytes`);
});
