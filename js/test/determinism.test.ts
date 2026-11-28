// Determinism of host-driven runs, the fallback when a snapshot no longer loads (D1680,
// D3745): a deterministic run begun again from its script with the deliveries of the
// original (`BeginOptions.deliveries`), on another fuel slice, makes the same calls, prints
// the same output and reaches the same state at every idle point: the same canonical hash
// and the same snapshot bytes. A snapshot loaded at an idle point and continued with the
// same results makes the same calls and output to the same end. A re-run that does something else is
// `diverged`, naming the first difference.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { type Delivery, loadModule, type Run, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";
import { answer, piOptions } from "./pi-setup.ts";

const DIR = new URL("../../tests/corpus/codemode-harvest/", import.meta.url);

/** Deterministic, with a clock that moves at every result (the deliveries record it). */
function det(): Partial<SandboxOptions> {
  let t = 1_700_000_000_000;
  return { deterministic: { startedAt: t, seed: 7 }, now: () => (t += 1000) };
}

interface Point {
  out: string[];
  calls: string;
  hash: string;
  snapshot: Uint8Array;
}

/** What the run looks like at an idle point of sandbox `sb`. */
function point(sb: Sandbox, run: Run, out: string[]): Point {
  return { out: [...out], calls: JSON.stringify(run.calls), hash: sb.canonicalHash(), snapshot: run.snapshot() };
}

/** The original: pi's setup, one call answered at every idle point; its points and deliveries. */
async function original(module: WebAssembly.Module, code: string) {
  const out: string[] = [];
  const sb = new Sandbox(piOptions(module, out, undefined, det()));
  const run = sb.begin(code, {}, { filename: "codemode.js" });
  const points: Point[] = [];
  try {
    while (run.state === "idle" && run.calls.length > 0) {
      points.push(point(sb, run, out));
      answer(run);
      await run.advance();
    }
  } catch (e) {
    if (!(e instanceof SandboxError)) throw e;
    out.push(`step: ${e.kind}`);
  }
  const deliveries = [...run.deliveries];
  const outcome = JSON.stringify([run.state, run.value, run.error?.message]);
  sb.dispose();
  return { points, deliveries, out, outcome };
}

const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((x, i) => x === b[i]);

test("re-runs of the harvest from their deliveries reach every idle point of the original, on another fuel slice", async () => {
  const module = await loadModule();
  const files = readdirSync(DIR).filter((f) => f.endsWith(".js")).sort();
  let points = 0;
  const differ: string[] = [];
  for (const f of files) {
    const code = readFileSync(new URL(f, DIR), "utf8");
    const o = await original(module, code);
    for (let k = 0; k < o.points.length; k++) {
      const out: string[] = [];
      const sb = new Sandbox(piOptions(module, out, undefined, { ...det(), fuelSlice: 997 }));
      try {
        const run = sb.begin(code, {}, { filename: "codemode.js", deliveries: o.deliveries.slice(0, k) });
        const p = point(sb, run, out);
        const want = o.points[k];
        const what = `${f} at idle point ${k}`;
        if (JSON.stringify(p.out) !== JSON.stringify(want.out)) differ.push(`${what}: output`);
        else if (p.calls !== want.calls) differ.push(`${what}: calls ${p.calls} vs ${want.calls}`);
        else if (p.hash !== want.hash) differ.push(`${what}: canonical hash`);
        else if (!same(p.snapshot, want.snapshot)) differ.push(`${what}: snapshot bytes`);
        assert.equal(run.deliveries.length, k);
        points++;
      } finally {
        sb.dispose();
      }
    }
  }
  assert.deepEqual(differ.slice(0, 10), [], `${differ.length} idle points differ`);
  assert.ok(points > 1000, `${points} idle points`);
});

test("a whole re-run ends as the original did; a loaded snapshot continued with the recorded results stays on its path", async () => {
  const module = await loadModule();
  const files = readdirSync(DIR).filter((f) => f.endsWith(".js")).sort();
  const differ: string[] = [];
  for (const f of files) {
    const code = readFileSync(new URL(f, DIR), "utf8");
    const o = await original(module, code);
    // The whole re-run, then the rest of the run driven by hand.
    const out: string[] = [];
    const sb = new Sandbox(piOptions(module, out, undefined, { ...det(), fuelSlice: 1009 }));
    const run = sb.begin(code, {}, { filename: "codemode.js", deliveries: o.deliveries });
    try {
      while (run.state === "idle" && run.calls.length > 0) {
        answer(run);
        await run.advance();
      }
    } catch (e) {
      if (!(e instanceof SandboxError)) throw e;
      out.push(`step: ${e.kind}`);
    }
    if (JSON.stringify(out) !== JSON.stringify(o.out) || JSON.stringify([run.state, run.value, run.error?.message]) !== o.outcome) {
      differ.push(`${f}: re-run`);
    }
    sb.dispose();
    // A snapshot from the middle, loaded and given the original's results with their
    // clocks: the same calls and output at every later idle point, and the same end.
    if (o.points.length === 0) continue;
    const k = o.points.length >> 1;
    const out2 = [...o.points[k].out];
    const sb2 = new Sandbox(piOptions(module, out2, undefined, det()));
    const loaded = sb2.loadSnapshot(o.points[k].snapshot);
    try {
      for (let j = k; loaded.state === "idle" && loaded.calls.length > 0; j++) {
        const want = o.points[j];
        if (JSON.stringify(loaded.calls) !== want.calls || JSON.stringify(out2) !== JSON.stringify(want.out)) {
          differ.push(`${f}: loaded at ${k}, idle point ${j}`);
          break;
        }
        const d = o.deliveries[j];
        loaded.settle(d.id, { ok: d.ok, value: d.value, now: d.now });
        await loaded.advance();
      }
    } catch (e) {
      if (!(e instanceof SandboxError)) throw e;
      out2.push(`step: ${e.kind}`);
    }
    if (JSON.stringify(out2) !== JSON.stringify(o.out) || JSON.stringify([loaded.state, loaded.value, loaded.error?.message]) !== o.outcome) {
      differ.push(`${f}: loaded at ${k}: the end`);
    }
    sb2.dispose();
  }
  assert.deepEqual(differ.slice(0, 10), [], `${differ.length} differ`);
});

test("a re-run that does something else diverges, naming the first difference", async () => {
  const module = await loadModule();
  const opts = (): SandboxOptions => ({ module, deterministic: { startedAt: 0, seed: 1 }, asyncFunctions: { "tools.read": () => 0 } });
  const code = `const a = await tools.read(1); const b = await tools.read(a + 1); return b;`;
  const sb = new Sandbox(opts());
  const run = sb.begin(code);
  run.settle(run.calls[0].id, { ok: true, value: 10 });
  await run.advance();
  run.settle(run.calls[0].id, { ok: true, value: 20 });
  await run.advance();
  assert.equal(run.value, 20);
  const deliveries: Delivery[] = [...run.deliveries];
  assert.deepEqual(deliveries.map((d) => [d.id, d.fn, d.args, d.value]), [[1, "tools.read", [1], 10], [2, "tools.read", [11], 20]]);
  sb.dispose();
  const rerun = (code: string, d = deliveries) => {
    const again = new Sandbox(opts());
    try {
      return again.begin(code, {}, { deliveries: d });
    } finally {
      again.dispose();
    }
  };
  assert.equal(rerun(code).value, 20);
  const diverged = (re: RegExp) => (e: unknown) => e instanceof SandboxError && e.kind === "diverged" && re.test(e.message);
  assert.throws(() => rerun(code.replace("a + 1", "a + 2")), diverged(/call 2 is tools\.read\(\[12\]\), the recording's tools\.read\(\[11\]\)/));
  assert.throws(() => rerun(`return 1;`), diverged(/the run ended before delivery 1 of 2/));
  assert.throws(() => rerun(`await tools.read(1); await null; return 2;`), diverged(/the run ended before delivery 2 of 2/));
  assert.throws(() => rerun(code, [{ ...deliveries[0], id: 5 }]), diverged(/delivery 1 of 1 \(call 5, tools\.read\) is for a call the re-run has not made \(open: 1 tools\.read\)/));
  // Only deterministic sandboxes re-run; the sandbox is usable after a divergence.
  const plain = new Sandbox({ module, asyncFunctions: { "tools.read": () => 0 } });
  assert.throws(() => plain.begin(code, {}, { deliveries }), /deterministic/);
  plain.dispose();
  const again = new Sandbox(opts());
  assert.throws(() => again.begin(`return 1;`, {}, { deliveries }), diverged(/ended/));
  assert.equal(again.run("return 2"), 2);
  again.dispose();
});

test("a deterministic snapshot loads only into a deterministic sandbox, and keeps its clock and random numbers", async () => {
  const module = await loadModule();
  const code = `const t0 = Date.now(); const r0 = Math.random(); await tools.read(); return [Date.now() - t0, Math.random() !== r0];`;
  const opts = (extra: Partial<SandboxOptions> = {}): SandboxOptions => ({ module, asyncFunctions: { "tools.read": () => 0 }, ...extra });
  const sb = new Sandbox(opts({ deterministic: { startedAt: 5000, seed: 3 }, now: 9000 }));
  const run = sb.begin(code);
  const snap = run.snapshot();
  run.settle(run.calls[0].id, { ok: true, value: 0 });
  await run.advance();
  const want = run.value;
  assert.deepEqual(want, [4000, true]);
  sb.dispose();
  const plain = new Sandbox(opts());
  assert.throws(() => plain.loadSnapshot(snap), /strong weak references/);
  plain.dispose();
  const again = new Sandbox(opts({ deterministic: { startedAt: 0, seed: 99 }, now: 9000 }));
  const loaded = again.loadSnapshot(snap);
  loaded.settle(loaded.calls[0].id, { ok: true, value: 0 });
  await loaded.advance();
  assert.deepEqual(loaded.value, want);
  assert.deepEqual(loaded.deliveries.map((d) => d.now), [9000]);
  again.dispose();
});
