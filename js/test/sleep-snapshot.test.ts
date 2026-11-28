// The sleep-snapshot example (examples/sleep-snapshot) in one process with a short sleep:
// the run is snapshotted while the tool sleeps, the sandbox dropped, and the run continued
// from the snapshot in a new sandbox, whose host settles the open sleep at the deadline the
// snapshot kept as the call's state; and the command line's two halves in two processes.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { Sandbox } from "../src/sandbox.ts";
import { options, resume, suspend } from "../../examples/sleep-snapshot/sleep-snapshot.ts";
import { docsModule } from "./docs-module.ts";

test("examples/sleep-snapshot: snapshot while the tool sleeps, load, continue", async () => {
  const lines: string[] = [];
  const log = (...a: unknown[]) => void lines.push(a.map(String).join(" "));
  const t0 = Date.now();
  const snapshot = await suspend(50, options(log));
  assert.deepEqual(lines, ["first sleeps 50 ms"]);
  if (await docsModule()) {
    // The open call with its deadline, the two waiting functions by their bytecode, the
    // script with its text, the host's values by name (the README's description).
    const sb = await Sandbox.create(options(log));
    const described = sb.describeSnapshot(snapshot);
    const i = sb.inspectSnapshot(snapshot);
    sb.dispose();
    const data = i.vm.fields.hostData as { v: string };
    const m = /^\{"states":\[\[2,\{"due":(\d+)\}\]\],"fuelUsed":\d+\}$/.exec(data.v);
    assert.ok(m, data.v);
    assert.ok(Number(m[1]) >= t0 + 50 && Number(m[1]) <= Date.now() + 50, data.v);
    assert.deepEqual(i.functions.filter((f) => f.hashKind === "bytecode").map((f) => f.label), ["step", "the body"]);
    assert.deepEqual([i.scripts[0].name, i.scripts[0].params], ["sleep.js", ["ms"]]);
    assert.deepEqual(i.host.map((h) => h.name), ["console", "tools", "tools.sleep"]);
    assert.match(described, /step\(label = "first", ms = 50\) is waiting for tools\.sleep\(50\)/);
    const body = i.view.calls[0].children[0].children[0];
    assert.equal(body.doing, 'inside a for…of loop over ["first", "second"], at item 1 of 2, halfway through results.push(…)');
  }
  const result = await resume(snapshot, options(log));
  assert.deepEqual(result, ["first: slept 50 ms", "second: slept 50 ms"]);
  assert.deepEqual(lines, ["first sleeps 50 ms", "second sleeps 50 ms", "done after at least 100 ms: first: slept 50 ms, second: slept 50 ms"]);
  assert.ok(Date.now() - t0 >= 100);
});

test("examples/sleep-snapshot: two processes, the second resuming the snapshot the first wrote", () => {
  // As `run.sh --restart`: process 2 gets the script from the snapshot.
  const run = (...args: string[]) =>
    execFileSync(process.execPath, [fileURLToPath(new URL("../../examples/sleep-snapshot/sleep-snapshot.ts", import.meta.url)), ...args], {
      cwd: fileURLToPath(new URL("../..", import.meta.url)),
      encoding: "utf8",
      timeout: 30_000,
    })
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => l.replace(/\(\d+ bytes\)/, "(N bytes)"));
  assert.deepEqual(run("start", "30"), [
    "host: starting sleep.js with ms = 30",
    "  [vm] first sleeps 30 ms",
    "host: snapshotted the run into examples/sleep-snapshot/out/sleep.snapshot (N bytes) and dropped the sandbox",
  ]);
  assert.deepEqual(run("resume"), [
    "host: resuming examples/sleep-snapshot/out/sleep.snapshot in a new sandbox",
    "  [vm] second sleeps 30 ms",
    "  [vm] done after at least 60 ms: first: slept 30 ms, second: slept 30 ms",
    "host: the run returned [ 'first: slept 30 ms', 'second: slept 30 ms' ]",
  ]);
});
