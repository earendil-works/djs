// Continuation dumps through the Sandbox (0175, docs/continuation-dumps.md): at every idle
// point of a run, dump it, resume the dump in a fresh sandbox and continue there. The
// transcript (console output, the open calls at each idle point, the result) must equal
// the uninterrupted run's.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, type OpenCall, type Run, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

/** The tools' answers: deterministic in their arguments; "fail" rejects. */
function answer(c: OpenCall): { ok: boolean; value: unknown } {
  const [a] = c.args as [unknown];
  if (a === "fail") return { ok: false, value: new RangeError(`no ${String(a)}`) };
  if (c.fn === "tools.read") return { ok: true, value: `body of ${String(a)}\nTODO x\nTODO y` };
  if (c.fn === "tools.list") return { ok: true, value: ["a.ts", "b.ts", "c.ts"] };
  return { ok: true, value: JSON.stringify(c.args) };
}

const CASES: Record<string, string> = {
  "parallel tools": `
    const files = await tools.list();
    const todos = async (f) => (await tools.read(f)).split("\\n").filter((l) => l.includes("TODO")).length;
    const counts = await Promise.all(files.map(todos));
    console.log("counts", counts.join());
    return counts.reduce((a, b) => a + b, 0);
  `,
  loops: `
    const seen = new Map([["x", 1], ["y", 2]]);
    const out = [];
    for (const [k, v] of seen) {
      for (let i = 0; i < 2; i++) out.push(k + v + i + ":" + (await tools.echo(k, i)));
    }
    let n = 0;
    while (n < 2) n += (await tools.echo(n)).length > 0 ? 1 : 0;
    return [out, n];
  `,
  generators: `
    function* ids() { yield "a"; yield "b"; yield "c"; }
    async function* pages() {
      for (const id of ids()) yield await tools.read(id);
    }
    const got = [];
    for await (const p of pages()) got.push(p.split("\\n")[0]);
    const it = ids();
    it.next();
    const later = await tools.echo("mid");
    got.push(later, it.next().value);
    return got;
  `,
  classes: `
    class Counter {
      static made = 0;
      constructor(label) { this.label = label; this.count = 0; Counter.made++; }
      async bump(by) { this.count += (await tools.echo(by)).length; return this.count; }
      get n() { return this.count; }
    }
    class Named extends Counter { describe() { return this.label + "=" + this.n + "/" + super.constructor.name; } }
    const c = new Named("c");
    await c.bump(1);
    await c.bump(22);
    return [c.describe(), Counter.made, c instanceof Counter];
  `,
  rejections: `
    const r = [];
    try { await tools.read("fail"); } catch (e) { r.push(e.name + ": " + e.message); }
    const settled = await Promise.allSettled([tools.read("fail"), tools.echo("ok")]);
    r.push(settled.map((s) => s.status).join());
    const caught = tools.read("fail").catch((e) => "caught " + e.message);
    r.push(await tools.echo("between"), await caught);
    return r;
  `,
};

interface Transcript {
  lines: string[];
  result: unknown;
}

function sandboxOptions(module: WebAssembly.Module, lines: string[]): SandboxOptions {
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const tool = () => 0; // a host-driven run never calls them
  return {
    module,
    console: { log, info: log, warn: log, error: log },
    deterministic: { startedAt: 1000, seed: 1 },
    now: 2000,
    asyncFunctions: { "tools.read": tool, "tools.list": tool, "tools.echo": tool },
  };
}

/**
 * Runs `code` one result per idle point (the oldest open call first). With `dumpEvery`,
 * the run moves to a fresh sandbox through a dump at every idle point.
 */
async function drive(module: WebAssembly.Module, code: string, dumpEvery: boolean): Promise<Transcript & { dumps: number }> {
  const lines: string[] = [];
  let sb = new Sandbox(sandboxOptions(module, lines));
  let run: Run = sb.begin(code);
  let dumps = 0;
  while (run.state === "idle") {
    lines.push(`idle: ${run.calls.map((c) => `#${c.id} ${c.fn}(${JSON.stringify(c.args)})`).join(", ")}`);
    if (dumpEvery) {
      const text = run.dump();
      sb.dispose();
      ({ sandbox: sb, run } = await Sandbox.resume(text, sandboxOptions(module, lines)));
      dumps++;
    }
    const c = run.calls[0];
    run.settle(c.id, answer(c));
    await run.advance();
  }
  sb.dispose();
  if (run.state === "threw") throw run.error;
  return { lines, result: run.value, dumps };
}

for (const [name, code] of Object.entries(CASES)) {
  test(`dump and resume at every idle point: ${name}`, async () => {
    const module = await loadModule();
    const plain = await drive(module, code, false);
    const dumped = await drive(module, code, true);
    assert.ok(dumped.dumps >= 2, `${dumped.dumps} idle points`);
    assert.deepEqual(dumped.lines, plain.lines);
    assert.deepEqual(dumped.result, plain.result);
  });
}

// Instance fields (public or private) of a class declared in a waiting function are not
// written by the exporter yet (its field initializer is an internal function).
const FIELDS = `
  class P { #n = 0; x = 1; async add() { this.#n += (await tools.echo(1)).length; return this.#n + this.x; } }
  const p = new P();
  return [await p.add(), await p.add()];
`;

test("dump and resume at every idle point: class fields", { todo: "the exporter refuses instance fields of a class in a waiting function" }, async () => {
  const module = await loadModule();
  const plain = await drive(module, FIELDS, false);
  const dumped = await drive(module, FIELDS, true);
  assert.deepEqual(dumped.lines, plain.lines);
  assert.deepEqual(dumped.result, plain.result);
});

test("refusals are SandboxErrors", async () => {
  const module = await loadModule();
  const lines: string[] = [];
  const sb = new Sandbox(sandboxOptions(module, lines));
  // A weak collection cannot be written.
  const run = sb.begin("const w = new WeakMap(); w.set({}, 1); await tools.echo(1); return w;");
  assert.equal(run.state, "idle");
  assert.throws(() => run.dump(), (e: unknown) => e instanceof SandboxError && e.kind === "misuse" && /weak collection/.test(e.message));
  // The run goes on after a refused dump.
  run.settle(run.calls[0].id, { ok: true, value: 1 });
  await run.advance();
  assert.equal(run.state, "done");
  // An idle-only operation on a finished run, a dump naming a tool the host lacks, and a
  // text that is no dump.
  assert.throws(() => run.dump(), /the run is done/);
  const waiting = sb.begin("return await tools.echo(1);");
  const text = waiting.dump();
  await assert.rejects(Sandbox.resume(text, { module }), (e: unknown) => e instanceof SandboxError && /unknown host function tools.echo/.test(e.message));
  await assert.rejects(Sandbox.resume("return 1", { module }), (e: unknown) => e instanceof SandboxError && /not a djs dump/.test(e.message));
  // A dump that a build cannot resume at its site refuses with the reason.
  const broken = text.replace(/at: "await#1"/, 'at: "await#9"');
  await assert.rejects(Sandbox.resume(broken, sandboxOptions(module, [])), (e: unknown) => e instanceof SandboxError && e.kind === "misuse");
});
