// The harvested codemode corpus (tests/corpus/codemode-harvest) in pi's code mode setup
// (pi-setup.ts), each case moved to a fresh sandbox through a snapshot at every idle point
// (D3720): its output and outcome are the uninterrupted run's, and nothing refuses.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { loadModule } from "../src/sandbox.ts";
import { piRun } from "./pi-setup.ts";

const DIR = new URL("../../tests/corpus/codemode-harvest/", import.meta.url);

test("the harvest in pi's setup continues the same through snapshots", async () => {
  const module = await loadModule();
  const files = readdirSync(DIR).filter((f) => f.endsWith(".js")).sort();
  let points = 0;
  const differ: string[] = [];
  for (const f of files) {
    const code = readFileSync(new URL(f, DIR), "utf8");
    // The same clock and random numbers in both (the cases read them).
    const det = { deterministic: { startedAt: 1_700_000_000_000, seed: 7 }, now: 1_700_000_000_000 };
    const straight = await piRun(module, code, false, undefined, det);
    const moved = await piRun(module, code, true, undefined, det);
    points += moved.sizes.length;
    try {
      assert.deepEqual(moved.out, straight.out);
      assert.deepEqual(moved.outcome, straight.outcome);
    } catch {
      differ.push(`${f}: ${JSON.stringify(moved.out.at(-1))} vs ${JSON.stringify(straight.out.at(-1))}`);
    }
  }
  assert.deepEqual(differ, [], `${differ.length} of ${files.length} differ`);
  assert.ok(points > 1000, `${points} idle points`);
});
