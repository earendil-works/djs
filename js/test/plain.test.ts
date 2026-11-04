// Plain dumps (D2830, D2890): a dump is an ordinary Script; the sandbox loads it by
// running it with `$djs`: the dump runtime's helpers (D2903) and the hooks
// `$djs.resume(id, name, args, state)` and `$djs.seed(lo, hi)`, which reopen the dump's
// open calls under their ids and set Math.random's state. These dumps are written by hand
// in the exporter's shape (docs/plain-js-dumps.md section 2).

import { test } from "node:test";
import assert from "node:assert/strict";
import { type AsyncHostFunction, type HostCall, loadModule, plainDump, Sandbox, SandboxError } from "../src/sandbox.ts";

/** `main` stopped at its first await of `job(1)`, call 4 with state {n: 1}. */
const DUMP = `"djs-dump";
$djs.seed(5, 7);
globalThis.$dump = {};
try {
$dump.p4 = $djs.resume(4, "job", [1], { "n": 1 });
$dump.Ka = async function () {
const job2 = job;
const a = (await $dump.p4);
const b = await job2(2);
log(a, b, typeof $djs, Math.random());
return a + b;
};
$dump.Ka.call(undefined);
} finally { delete globalThis.$dump; delete globalThis.$djs; }
`;

function host() {
  const lines: string[] = [];
  const seen: { calls: HostCall[]; resumed: [unknown, HostCall][] } = { calls: [], resumed: [] };
  const job: AsyncHostFunction<{ n: number }> = {
    call(n) {
      seen.calls.push(this);
      this.state = { n: Number(n) };
      return Promise.resolve(Number(n) * 10);
    },
    resume(state, call) {
      seen.resumed.push([state, call]);
      return Promise.resolve(state.n * 100);
    },
  };
  return { lines, seen, opts: { functions: { log: (...a: unknown[]) => void lines.push(a.map(String).join(" ")) }, asyncFunctions: { job } } };
}

test("plainDump reads the header", () => {
  assert.deepEqual(plainDump(DUMP), { typescript: false });
  assert.deepEqual(plainDump(`"djs-dump typescript";\n`), { typescript: true });
  assert.equal(plainDump(`"djs-dump 2";\n`), undefined);
  assert.equal(plainDump(`"use strict";\n"djs-dump";\n`), undefined);
});

test("Sandbox.resume continues a plain dump through the hooks", async () => {
  const module = await loadModule();
  // A module without the dumper loads plain dumps too: they are code.
  const nodump = process.env.DJS_WASM_NODUMP;
  if (nodump) {
    const h = host();
    const task = await Sandbox.resume(DUMP, { module: await loadModule(nodump), ...h.opts });
    assert.equal(await task.result, 120);
    task.sandbox.dispose();
  }
  const h = host();
  const task = await Sandbox.resume(DUMP, { module, ...h.opts });
  assert.equal(await task.result, 120);
  assert.deepEqual(h.seen.resumed.map(([s, c]) => [s, c.id, c.fn, c.args, c.state]), [[{ n: 1 }, 4, "job", [1], { n: 1 }]]);
  // The new call comes after the dump's ids; $djs is gone once the dump loaded.
  assert.deepEqual(h.seen.calls.map((c) => [c.id, c.args]), [[5, [2]]]);
  const [line] = h.lines;
  assert.match(line, /^100 20 undefined 0\.\d+$/);
  // Math.random continues from the dumped state: a VM given that state draws the same.
  const sb = new Sandbox({ module });
  sb.load(`"djs-dump";\n$djs.seed(5, 7);\nglobalThis.r = Math.random();\ndelete globalThis.$djs;\n`);
  assert.equal(String(sb.run(`return r`)), line.split(" ")[3]);
  task.sandbox.dispose();
  sb.dispose();
});

test("$djs holds the dump runtime while a plain dump loads", async () => {
  const module = await loadModule();
  const sb = new Sandbox({ module });
  // The helpers the dump calls (D2903): every one js/src/dump-runtime.js exports.
  const { helpers } = await import("../src/dump-runtime.js");
  sb.load(`"djs-dump";
globalThis.r = [Object.getOwnPropertyNames($djs).sort().join(), $djs.R(Math.max, undefined, [1, 2]), $djs.advance([1, 2, 3].values(), 1).next().value];
delete globalThis.$djs;
`);
  const want = [...Object.keys(helpers), "resume", "seed", "source"].sort().join();
  assert.deepEqual(sb.run(`return r`), [want, 2, 2]);
  sb.dispose();
});

test("seed sets Math.random's state whatever the sandbox's seed", async () => {
  const module = await loadModule();
  const draws = [1, 2].map((randomSeed) => {
    const sb = new Sandbox({ module, randomSeed });
    sb.load(`"djs-dump";\n$djs.seed(5, 7);\nglobalThis.r = Math.random();\ndelete globalThis.$djs;\n`);
    const r = sb.run(`return [r, typeof $djs]`);
    sb.dispose();
    return r;
  });
  assert.deepEqual(draws[0], draws[1]);
  assert.equal((draws[0] as unknown[])[1], "undefined");
});

test("loadRun keeps the dump's ids and states", async () => {
  const module = await loadModule();
  const h = host();
  const sb = new Sandbox({ module, ...h.opts });
  const run = sb.loadRun(DUMP);
  assert.equal(run.state, "idle");
  assert.deepEqual(run.calls, [{ id: 4, fn: "job", args: [1], state: { n: 1 } }]);
  run.settle(4, { ok: true, value: 7 });
  await run.advance();
  assert.deepEqual(run.calls.map((c) => [c.id, c.fn, c.args]), [[5, "job", [2]]]);
  run.settle(5, { ok: true, value: 8 });
  await run.advance();
  assert.equal(run.state, "done");
  assert.equal(run.value, 15);
  assert.equal(h.seen.calls.length + h.seen.resumed.length, 0);
  sb.dispose();
});

test("a plain function's open call is rejected on resume", async () => {
  const module = await loadModule();
  const lines: string[] = [];
  const dump = DUMP.replace(`"job"`, `"plain"`)
    .replace("const job2 = job;", "const job2 = plain;")
    .replace("(await $dump.p4)", "await $dump.p4.catch((e) => e.message)");
  const task = await Sandbox.resume(dump, {
    module,
    functions: { log: (...a: unknown[]) => void lines.push(a.map(String).join(" ")) },
    asyncFunctions: { plain: async (n: unknown) => Number(n) },
  });
  assert.equal(await task.result, "plain: the call was open when the run was dumped, and the host function has no hooks to continue it2");
  task.sandbox.dispose();
});

test("a dump that does not load", async () => {
  const module = await loadModule();
  const h = host();
  for (const [text, why] of [
    [`"djs-dump";\n$djs.resume(1, "nope", []);\n`, "cannot load the dump: the dump threw: Error: resume: unknown host function nope"],
    [`"djs-dump";\n$djs.resume(1, "job", [1]); $djs.resume(1, "job", [2]);\n`, "cannot load the dump: the dump threw: Error: resume: the call is open already"],
    [`"djs-dump";\n$djs.seed("x");\n`, "cannot load the dump: the dump threw: TypeError: seed: not two int32 halves"],
    [`"djs-dump";\n1;\n`, "the dump holds no run (no handle named run)"],
  ]) {
    const sb = new Sandbox({ module, ...h.opts });
    assert.throws(() => sb.loadRun(text), (e: unknown) => e instanceof SandboxError && e.kind === "misuse" && e.message === why, text);
    sb.dispose();
  }
});

test("a task suspended into a plain dump (the exporter's) resumes through the hooks, ids and all", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1); log("a", a); const b = await sleep(2); log("b", b, Math.random() < 1); return a + b;`;
  const pending: { n: number; resolve(v: number): void }[] = [];
  const resumed: [unknown, HostCall][] = [];
  const calls: HostCall[] = [];
  const lines: string[] = [];
  const sleep: AsyncHostFunction<{ n: number }> = {
    call(n) {
      calls.push(this);
      this.state = { n: Number(n) };
      return new Promise((resolve) => pending.push({ n: Number(n), resolve }));
    },
    resume(state, call) {
      resumed.push([state, call]);
      return Promise.resolve(state.n * 100);
    },
  };
  const opts = { module, frozenBuiltins: true, functions: { log: (...a: unknown[]) => void lines.push(a.join(" ")) }, asyncFunctions: { sleep } };
  const task = new Sandbox(opts).start(code);
  while (pending.length === 0) await new Promise((r) => setTimeout(r, 1));
  const dump = await task.suspend();
  await assert.rejects(task.result, (e: unknown) => e instanceof SandboxError && e.kind === "suspended");
  task.sandbox.dispose();
  assert.match(dump, /^"djs-dump";\n/);
  assert.match(dump, /\$djs\.seed\(-?\d+, -?\d+, 2\);/);
  assert.match(dump, /\$djs\.resume\(1, "sleep", \[1\], \{"n":1\}\)/);
  const again = await Sandbox.resume(dump, opts);
  while (pending.length === 1) await new Promise((r) => setTimeout(r, 1));
  // The next call has the id the uninterrupted run gives it (D2897): `log("a")`, a sync
  // call, takes 2.
  assert.deepEqual(calls.map((c) => [c.id, c.args]), [[1, [1]], [3, [2]]]);
  pending[1].resolve(20);
  assert.equal(await again.result, 120);
  assert.deepEqual(resumed.map(([s, c]) => [s, c.id]), [[{ n: 1 }, 1]]);
  assert.deepEqual(lines, ["a 100", "b 20 true"]);
  again.sandbox.dispose();
});

// A dump names the codemode body it continues by `$djs.source(name, hash, params)` (D2882):
// the sandbox compiles the script it ran (this process's, or the `sources` option's)
// again; one it does not have is a clear error, and `inlineSources` writes the text.
test("a dump names its script by $djs.source; the host compiles it again", async () => {
  const module = await loadModule();
  const code = `const a = await sleep(1);\nlog("got", a);\nreturn a + 1;`;
  const sleep: AsyncHostFunction = { call: (n) => new Promise(() => {}), resume: () => Promise.resolve(41) };
  const lines: string[] = [];
  const opts = { module, frozenBuiltins: true, functions: { log: (...a: unknown[]) => void lines.push(a.join(" ")) }, asyncFunctions: { sleep } };
  const task = new Sandbox(opts).start(code, {}, { filename: "job-7.js" });
  await new Promise((r) => setTimeout(r, 1));
  const dump = await task.suspend();
  await assert.rejects(task.result);
  task.sandbox.dispose();
  assert.match(dump, /\$djs\.source\("job-7\.js", "[0-9a-f]{16}"\)/);
  assert.equal(dump.split("log(\"got\", a)").length, 2, dump);
  // This process ran it: it loads.
  const again = await Sandbox.resume(dump, opts);
  assert.equal(await again.result, 42);
  again.sandbox.dispose();
  // A script the host does not have, then given by `sources`.
  const other = dump.replace(/\$djs\.source\("job-7\.js"/, '$djs.source("job-8.js"');
  await assert.rejects(Sandbox.resume(other, opts), (e: unknown) => e instanceof SandboxError && /job-8\.js.*does not have/.test(e.message));
  const given = await Sandbox.resume(other, { ...opts, sources: (name: string) => (name === "job-8.js" ? code : undefined) });
  assert.equal(await given.result, 42);
  given.sandbox.dispose();
  // Another text under that name is refused.
  await assert.rejects(
    Sandbox.resume(other, { ...opts, sources: () => code + "\n" }),
    (e: unknown) => e instanceof SandboxError && /not the one the dump refers to/.test(e.message),
  );
  // Inline: the text, no reference.
  const t2 = new Sandbox({ ...opts, inlineSources: true }).start(code, {}, { filename: "job-7.js" });
  await new Promise((r) => setTimeout(r, 1));
  const inline = await t2.suspend();
  await assert.rejects(t2.result);
  t2.sandbox.dispose();
  assert.doesNotMatch(inline, /\$djs\.source/);
  assert.equal(inline.split("log(\"got\", a)").length, 3, inline);
});
