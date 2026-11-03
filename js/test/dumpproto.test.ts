// The TypeScript dumper prototype (js/proto/dump.ts) against the Rust one (0167, 0168):
// js/proto/compare.ts round-trips the same cases through both, one Script and REPL-style
// line by line, and fails if either dump loads wrong or a refusal is missing.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

test("the TypeScript dumper prototype agrees with Vm::dump", () => {
  const script = fileURLToPath(new URL("../proto/compare.ts", import.meta.url));
  const r = spawnSync(process.execPath, [script], { encoding: "utf8", env: process.env, timeout: 120_000 });
  assert.equal(r.status, 0, `${r.stdout}\n${r.stderr}`);
  assert.doesNotMatch(r.stdout, /differs|refused|load failed|missing/);
});
