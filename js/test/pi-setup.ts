// pi's code mode setup for the snapshot tests (snapshot*.test.ts): the sandbox options of
// pi-mono's packages/codemode/src/run.ts `sandboxOptions`, and a host-driven run that can
// move to a fresh sandbox through a snapshot at every idle point (D3720).

import assert from "node:assert/strict";
import { type Run, RunExit, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";

const TOOLS = [
  { name: "read", jsName: "read", description: "Reads a file. ".repeat(40) },
  { name: "my-tool", jsName: "my_tool", description: "Does a thing. ".repeat(40) },
];

/** The sandbox options of pi's code mode (pi-mono packages/codemode/src/run.ts `sandboxOptions`). */
export function piOptions(module: WebAssembly.Module, out: string[], fuel = 1e9, extra: Partial<SandboxOptions> = {}): SandboxOptions {
  const host = () => {};
  const asyncFunctions: Record<string, () => void> = { "@mcp/github#search": host, searchTools: host, later: host };
  const aliases: Record<string, string[]> = {};
  const listed = TOOLS.map((t) => ({ name: t.jsName, description: t.description }));
  for (const t of TOOLS) {
    asyncFunctions[`tools.${t.jsName}`] = host;
    if (t.name !== t.jsName) aliases[`tools.${t.jsName}`] = [t.name];
  }
  const log = (...a: unknown[]) => void out.push(a.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).join(" "));
  return {
    module,
    maxFuel: fuel,
    frozenBuiltins: true,
    typescript: true,
    console: { log, info: log, warn: log, error: log, debug: log },
    unhandledRejections: "collect",
    functions: {
      text: (v: unknown) => void out.push(`text ${typeof v === "string" ? v : JSON.stringify(v)}`),
      image: () => {},
      exit: () => {
        throw new RunExit("exited");
      },
      store: () => {},
      load: () => undefined,
    },
    asyncFunctions,
    aliases,
    missingMember: (ns, member) => (ns === "tools" ? `tools.${member} is not a tool. ALL_TOOLS lists every tool.` : undefined),
    values: { ALL_TOOLS: listed },
    modules: (specifier) => (specifier === "@mcp/github" ? { exports: { search: "@mcp/github#search" } } : undefined),
    jsonBoundary: { except: ["text", "image", "store", "later"] },
    ...extra,
  };
}

/** Answers the oldest open call as the test host does: `later(v, rejected)` with v, a tool with its arguments. */
export function answer(run: Run): void {
  const c = run.calls[0];
  if (c.fn === "later") run.settle(c.id, c.args[1] === true ? { ok: false, value: c.args[0] } : { ok: true, value: c.args[0] });
  else run.settle(c.id, { ok: true, value: { fn: c.fn, args: c.args } });
}

/**
 * Runs `code` in pi's setup to its end, answering one call at every idle point: straight through,
 * or (`moves`) snapshotting it there and going on in a fresh sandbox each time. Its
 * output, outcome and the snapshots' sizes.
 */
export async function piRun(module: WebAssembly.Module, code: string, moves: boolean, fuel?: number, extra: Partial<SandboxOptions> = {}) {
  let k = 0;
  const out: string[] = [];
  let sb = new Sandbox(piOptions(module, out, fuel, extra));
  let run = sb.begin(code, {}, { filename: "codemode.js", start: false });
  const sizes: number[] = [];
  let hostText = false;
  try {
    await run.advance();
    while (run.state === "idle") {
      if (moves) {
        // Every other one without the script's text: found by its hash.
        const snap = run.snapshot({ sources: k++ % 2 === 0 });
        sizes.push(snap.length);
        hostText ||= new TextDecoder("latin1").decode(snap).includes("Does a thing");
        const used = run.fuelUsed;
        sb.dispose();
        sb = new Sandbox(piOptions(module, out, fuel, extra));
        run = sb.loadSnapshot(snap);
        assert.equal(run.fuelUsed, used);
      }
      answer(run);
      await run.advance();
    }
  } catch (e) {
    if (!(e instanceof SandboxError)) throw e;
    out.push(`step: ${e.kind}`);
  }
  const outcome = run.state === "done" ? { value: run.value, exited: run.exited } : { error: run.error?.message };
  sb.dispose();
  return { out, outcome, sizes, hostText };
}

