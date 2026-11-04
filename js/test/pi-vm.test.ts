// The pi codemode adapter (js/pi/vm.ts) running pi's real prelude (tests/pi, copied from
// pi-private) through a small host like pi's in-thread launcher: tool calls, output,
// store/load, errors, interrupts, determinism, and continuation dumps (the next mode). pi's own test suite runs
// against the adapter with `node js/pi/pi-tests.ts` (docs/host.md "pi codemode").

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule } from "../src/sandbox.ts";
import {
  resumeFromDump,
  type SessionDump,
  startVm,
  type HostToWorkerMessage,
  type VmHooks,
  type VmSession,
  type VmStartData,
  type WorkerToHostMessage,
} from "../pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";
import { PRELUDE_NEXT } from "../../tests/pi/prelude-next.ts";

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
  frozenBuiltins?: boolean;
  prelude?: string;
}

interface Outcome {
  messages: WorkerToHostMessage[];
  output: unknown[];
  calls: { name: string; args: unknown }[];
  end: Extract<WorkerToHostMessage, { type: "done" } | { type: "crash" }>;
  session: VmSession;
}

const wasm = await loadModule();
const STARTED_AT = Date.UTC(2026, 0, 15, 10, 0, 0);
const later = (ms: number, v: unknown) => new Promise((resolve) => setTimeout(() => resolve(v), ms));

/**
 * The adapter's modes: pi's prelude as it is; prelude-next (D2565, D2650); prelude-next with
 * a dump and a resume in a new instance at every idle point (D2651). Every test runs in each.
 */
interface Mode {
  name: string;
  prelude: string;
  hooks: Pick<VmHooks, "shape" | "dumpEveryIdle">;
}
const MODES: Mode[] = [
  { name: "pi", prelude: PRELUDE_SOURCE, hooks: {} },
  { name: "next", prelude: PRELUDE_NEXT, hooks: { shape: "next" } },
  { name: "next, dump every idle", prelude: PRELUDE_NEXT, hooks: { shape: "next", dumpEveryIdle: true } },
];

for (const mode of MODES) suite(mode);

function suite(mode: Mode): void {

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
    ...mode.hooks,
    wasm,
    prelude: options.prelude ?? mode.prelude,
    interruptBudget: options.interruptBudget,
    frozenBuiltins: options.frozenBuiltins,
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
const PNG = "iVBORw0KGgo=";

test(`${mode.name}: the return value, top-level await and output in order`, async () => {
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

test(`${mode.name}: concurrent tool calls are all issued before the first result`, async () => {
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

test(`${mode.name}: tool errors are catchable; unknown tools name close matches`, async () => {
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

test(`${mode.name}: globals, including namespaces with spread arguments`, async () => {
  const outcome = await run("return [await add(1), await fs.read('a')]", {
    globals: { add: (n) => (n as number) + 1, "fs.read": (args) => `read ${JSON.stringify(args)}` },
  });
  assert.deepEqual(value(outcome), [2, 'read "a"']);
});

test(`${mode.name}: store and load`, async () => {
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

test(`${mode.name}: script errors with the script's line numbers`, async () => {
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

test(`${mode.name}: exit() ends the script with its output and store writes`, async () => {
  const outcome = await run('text("before"); store("k", 1); await tools.echo(1); try { exit(); } catch {} text("after")', {
    tools: { echo },
  });
  assert.equal(value(outcome), undefined);
  assert.deepEqual(outcome.output, [{ type: "text", text: "before" }]);
});

test(`${mode.name}: an interrupt budget stops loops, including microtask loops`, async () => {
  for (const code of ["while (true) {}", "for (;;) await null", "let n = 0; for (;;) { try { n++; } catch {} }"]) {
    const outcome = await run(code, { interruptBudget: 300 });
    assert.deepEqual(outcome.end, { type: "crash", message: "Error: Execution interrupted" }, code);
  }
  // Work below the budget is not interrupted (the prelude does not count, D1660).
  assert.equal(value(await run("let n = 0; for (let i = 0; i < 1000; i++) n += i; return n", { interruptBudget: 300 })), 499500);
});

test(`${mode.name}: a budget means the same on cold and warm starts (prelude images)`, async () => {
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
        ...mode.hooks,
        wasm,
        prelude: mode.prelude,
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

test(`${mode.name}: the memory limit`, async () => {
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
    ...mode.hooks,
    wasm,
    prelude: mode.prelude,
    data: { code: "", tools: [], globals: [], memoryLimitBytes: 1024, store: {} },
    post: () => {},
    shouldInterrupt: () => false,
  }));
});

test(`${mode.name}: host globals quickjs-wasi provides`, async () => {
  const outcome = await run(`
    return [btoa("hello"), atob("aGVsbG8="), typeof performance.now(), await new Promise((r) => queueMicrotask(() => r(7))),
      Object.isFrozen(performance)];
  `);
  assert.deepEqual(value(outcome), ["aGVsbG8=", "hello", "number", 7, true]);
});

const DETERMINISTIC_CODE = `
  const values = [Date.now(), performance.now(), Math.random(), Math.random(), new Date().toISOString()];
  const first = await tools.echo(values);
  text(JSON.stringify([Date.now(), performance.now(), Math.random()]));
  return await tools.echo(first.concat(Math.random(), new Date(0).getHours()));
`;

test(`${mode.name}: deterministic runs repeat exactly, with QuickJS's random numbers`, async () => {
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
  assert.match((started as { fingerprint: string }).fingerprint, /^djs [0-9a-f]{8}, prelude [0-9a-f]{8}, host [0-9a-f]{8}, deterministic 3/);
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

/** pi's prelude without its lockdown: what it would run on frozen builtins (D2304). */
function withoutLockdown(prelude: string): string {
  const start = prelude.indexOf("\t(function lockdown() {");
  const close = "\n\t})();";
  const end = prelude.indexOf(close, start);
  assert.ok(start > 0 && end > start, "the prelude's lockdown");
  return prelude.slice(0, start) + prelude.slice(end + close.length);
}

test(`${mode.name}: frozen builtins without the prelude's lockdown (D2304)`, { skip: mode.hooks.shape === "next" && "prelude-next has no lockdown; the VM always has frozen builtins" }, async () => {
  const code = `
    class E extends Error { constructor(m) { super(m); this.name = "E"; } }
    const o = {}; o.toString = () => "own";
    let added; try { Array.prototype.extra = 1; added = "added"; } catch (e) { added = e.constructor.name; }
    globalThis.mine = 1;
    return [String(new E("x")), String(o), added, Object.isFrozen(Array.prototype), Object.isFrozen(globalThis), mine,
      typeof Object.getOwnPropertyDescriptor(Object.prototype, "toString").value];
  `;
  const prelude = withoutLockdown(mode.prelude);
  const frozen = value(await run(code, { frozenBuiltins: true, prelude }));
  assert.deepEqual(frozen, ["E: x", "own", "TypeError", true, false, 1, "function"]);
  // The lockdown gives the same answers, except that its override accessors replace the data
  // properties.
  assert.deepEqual(value(await run(code)), ["E: x", "own", "TypeError", true, false, 1, "undefined"]);
  // Without either, builtins are ordinary.
  assert.deepEqual(value(await run(code, { prelude })), ["E: x", "own", "added", false, false, 1, "function"]);
});
}

// ---- Continuation dumps of next-mode sessions (D2650): the session is dumped while a call
// is open, the instance thrown away, and the dump resumed in a new one; the host then
// decides what happens to the open call.

const DUMP_CODE = `
const seen = [];
for (const n of [1, 2]) seen.push(await tools.slow(n));
text("seen " + seen.join());
store("k", { seen });
let caught;
try { await tools.flaky(3); } catch (e) { caught = e.message + "/" + e.code; }
return { seen, caught, kept: load("k"), at: Date.now() };
`;

/** A next-mode session of DUMP_CODE that answers its calls itself, except `held` ones. */
function nextSession(data: VmStartData, onCall: (m: Extract<WorkerToHostMessage, { type: "call" }>) => void) {
  const messages: WorkerToHostMessage[] = [];
  let end!: (m: WorkerToHostMessage) => void;
  const ended = new Promise<WorkerToHostMessage>((r) => (end = r));
  const hooks = {
    wasm,
    shape: "next" as const,
    shouldInterrupt: () => false,
    post: (m: WorkerToHostMessage) => {
      messages.push(m);
      setTimeout(() => {
        if (m.type === "call") onCall(m);
        if (m.type === "done" || m.type === "crash") end(m);
      });
    },
  };
  return { messages, ended, hooks };
}

const DUMP_DATA: VmStartData = {
  code: DUMP_CODE,
  tools: ["slow", "flaky"].map((name) => ({ name, jsName: name, description: "" })),
  globals: [],
  memoryLimitBytes: undefined,
  store: { old: "1" },
  deterministic: { startedAt: STARTED_AT, seed: 7 },
};

function answerOf(m: Extract<WorkerToHostMessage, { type: "call" }>, seq: number): HostToWorkerMessage {
  const now = STARTED_AT + 1000 * seq;
  if (m.name === "flaky") return { type: "result", id: m.id, ok: false, payload: "flaky failed", properties: JSON.stringify({ code: 7 }), seq, now };
  return { type: "result", id: m.id, ok: true, payload: JSON.stringify(Number(JSON.parse(m.args!)) * 10), seq, now };
}

/** Runs DUMP_CODE; with `policy`, dumps at the first call and resumes, applying it. */
async function dumpRun(policy?: "deliver" | "reissue" | "reject"): Promise<{ messages: WorkerToHostMessage[]; end: WorkerToHostMessage; dumped?: SessionDump }> {
  let seq = 0;
  let session!: VmSession;
  const deliver = (m: Extract<WorkerToHostMessage, { type: "call" }>) => session.deliver(answerOf(m, ++seq));
  let held: Extract<WorkerToHostMessage, { type: "call" }> | undefined;
  const run = nextSession(DUMP_DATA, (m) => {
    if (policy !== undefined && held === undefined) held = m;
    else deliver(m);
  });
  session = await startVm({ ...run.hooks, prelude: PRELUDE_NEXT, data: DUMP_DATA });
  if (policy === undefined) return { messages: run.messages, end: await run.ended };
  while (held === undefined) await later(1, null);
  // A restart: the dump (JSON, as a host would store it) is all that survives.
  const dumped = JSON.parse(JSON.stringify(session.dumpSession())) as SessionDump;
  session.dispose();
  session = resumeFromDump(dumped, { ...run.hooks, prelude: PRELUDE_NEXT, data: DUMP_DATA });
  const open = session.openCalls();
  assert.deepEqual(open, [held]);
  if (policy === "deliver") deliver(held);
  else if (policy === "reissue") {
    // Issued again: the host's own record says what to call (here the same answer).
    deliver(open[0]);
  } else session.deliver({ type: "result", id: held.id, ok: false, payload: "interrupted", seq: ++seq, now: STARTED_AT + 1000 * seq });
  return { messages: run.messages, end: await run.ended, dumped };
}

test("next: a session dumped while a call is open resumes from the dump in a new instance", async () => {
  const plain = await dumpRun();
  const value = (end: WorkerToHostMessage) => {
    assert.equal(end.type, "done", JSON.stringify(end));
    return JSON.parse((end as { value: string }).value);
  };
  assert.deepEqual(value(plain.end), { seen: [10, 20], caught: "flaky failed/7", kept: { seen: [10, 20] }, at: STARTED_AT + 3000 });
  for (const policy of ["deliver", "reissue"] as const) {
    const resumed = await dumpRun(policy);
    // The same messages as without the restart: output, calls, store writes, result.
    assert.deepEqual(resumed.messages, plain.messages, policy);
    assert.match(resumed.dumped!.dump, /^"djs-dump 2";/);
    // The open call carries its pi id as its state (D2770); the adapter keeps no call table.
    assert.match(resumed.dumped!.dump, new RegExp(`\\$dump\\.pending\\(\\d+, "call", \\["call", "slow", "1"\\], \\{"id":${(resumed.messages.find((m) => m.type === "call") as { id: number }).id}\\}\\)`));
    assert.equal("ids" in resumed.dumped!.state.next, false);
  }
  const rejected = await dumpRun("reject");
  assert.equal(rejected.end.type, "done");
  assert.equal((rejected.end as { ok: boolean }).ok, false);
  assert.match((rejected.end as { error: string }).error, /interrupted/);
});

test("next: a dump keeps Math.random's sequence", async () => {
  const code = "const a = Math.random(); await tools.echo(1); return [a, Math.random()];";
  const data: VmStartData = { ...DUMP_DATA, code, tools: [{ name: "echo", jsName: "echo", description: "" }] };
  let session!: VmSession;
  const run = nextSession(data, (m) => session.deliver({ type: "result", id: m.id, ok: true, payload: "1", seq: 1, now: STARTED_AT }));
  session = await startVm({ ...run.hooks, dumpEveryIdle: true, prelude: PRELUDE_NEXT, data });
  const end = await run.ended;
  const [a, b] = JSON.parse((end as { value: string }).value);
  assert.notEqual(a, b);
});
