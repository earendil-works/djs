// Golden dumps of host-driven code mode runs (0291, D3596): tests/dump/v2/golden-host/ has
// codemode bodies (`NAME.body.js`) that run in the scope below (namespace and bound host
// functions by identity, `values`, the JSON boundary) and the dumps a Sandbox wrote of them
// at every idle point (`NAME.K.dump.js`); some import host modules (0292). Each dump
// loads in a fresh sandbox of the same scope, the open calls are answered, and the output
// up to idle point K plus the loaded run's must equal the uninterrupted run's. A golden dump is never rewritten to make a
// build pass; new programs get their dumps with
// `DJS_GOLDEN_WRITE=1 node --test js/test/golden-host.test.ts` (only programs with none).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { loadModule, type Run, Sandbox, type SandboxOptions } from "../src/sandbox.ts";

const DIR = new URL("../../tests/dump/v2/golden-host/", import.meta.url);

/** The programs whose dumps are written with the body's text (`inlineSources`): they load with no `sources`. */
const INLINE = new Set(["inline", "imports-inline"]);

const ALL_TOOLS = [
  { name: "read", description: "Reads a file" },
  { name: "ask", description: "Asks a person" },
  { name: "list", description: "Lists a directory" },
];

/** The host's answer to an open call: a pure function of the call. */
function answer(fn: string, args: unknown[]): { ok: boolean; value: unknown } {
  const a = (args[0] ?? {}) as { path?: string; q?: string };
  switch (fn) {
    case "tools.read":
      return { ok: true, value: `<${a.path}>` };
    case "tools.list":
      return { ok: true, value: `[${a.path}]` };
    case "tools.ask":
      return { ok: false, value: Object.assign(new Error("denied"), { structuredOutput: { by: "person" } }) };
    case "@mcp/github#search_issues":
      return { ok: true, value: [{ title: `issue about ${a.q}` }] };
    case "@mcp/linear#create_issue":
      return { ok: true, value: `made ${(args[0] as { title: string }).title}` };
    case "@mcp/linear#list":
      return { ok: true, value: [`${a.path} 1`, `${a.path} 2`] };
  }
  throw new Error(`no answer for ${fn}`);
}

/**
 * The scope every program runs in; `lines` gets the output. The store is the host's state:
 * a loaded run gets the writes made before its dump again, from the lines.
 */
function scope(module: Pick<SandboxOptions, "module" | "dumpModule">, lines: string[], name: string, body: string | undefined): SandboxOptions {
  const stored = new Map<string, string>();
  for (const line of lines) {
    const m = /^store (\S+) (.*)$/.exec(line);
    if (m) stored.set(m[1], m[2]);
  }
  return {
    ...module,
    frozenBuiltins: true,
    jsonBoundary: true,
    inlineSources: INLINE.has(name),
    console: { log() {}, info() {}, warn() {}, error() {} },
    functions: {
      text: (s) => void lines.push(String(s)),
      store: (k, v) => {
        const json = JSON.stringify(v);
        lines.push(`store ${String(k)} ${json}`);
        stored.set(String(k), json);
      },
      load: (k) => {
        const json = stored.get(String(k));
        return json === undefined ? undefined : JSON.parse(json);
      },
    },
    asyncFunctions: { "tools.read": () => 0, "tools.ask": () => 0, "tools.list": () => 0, "@mcp/github#search_issues": () => 0 },
    globals: { text: "text", store: "store", load: "load", tools: "tools", console: "console" },
    bindings: { search_issues: "@mcp/github#search_issues" },
    values: { ALL_TOOLS },
    // Host modules (imports): functions made on first import, by `specifier#export`.
    modules: (spec) =>
      spec === "@mcp/linear"
        ? { exports: { create_issue: {}, list: {}, default: "tools.list" } }
        : spec === "@mcp/github"
          ? { exports: { search_issues: "@mcp/github#search_issues" } }
          : undefined,
    sources: body === undefined ? undefined : (n) => (n === `${name}.js` ? body : undefined),
  };
}

/** Answers every open call, oldest first, and advances: one idle point further. */
async function step(run: Run): Promise<void> {
  for (const c of run.calls) run.settle(c.id, answer(c.fn, c.args));
  await run.advance();
}

/** The run to its end: the lines, and the output's length at each idle point. */
async function uninterrupted(module: WebAssembly.Module, name: string, body: string): Promise<{ want: string[]; marks: number[] }> {
  const lines: string[] = [];
  const marks: number[] = [];
  const sb = new Sandbox(scope({ module }, lines, name, body));
  const run = sb.begin(body, {}, { filename: `${name}.js` });
  while (run.state === "idle") {
    marks.push(lines.length);
    await step(run);
  }
  assert.equal(run.state, "done", `${name}: ${String(run.error)}`);
  lines.push(`result ${JSON.stringify(run.value)}`);
  sb.dispose();
  return { want: lines, marks };
}

const programs = readdirSync(DIR)
  .filter((f) => f.endsWith(".body.js"))
  .map((f) => f.slice(0, -".body.js".length));
const dumps = readdirSync(DIR).filter((f) => f.endsWith(".dump.js"));

test("write the dumps of golden host programs that have none", { skip: process.env.DJS_GOLDEN_WRITE === undefined }, async () => {
  const module = await loadModule();
  for (const name of programs) {
    if (dumps.some((d) => d.startsWith(`${name}.`))) continue;
    const body = readFileSync(new URL(`${name}.body.js`, DIR), "utf8");
    const { marks } = await uninterrupted(module, name, body);
    for (let k = 1; k <= marks.length; k++) {
      const sb = new Sandbox(scope({ module }, [], name, body));
      const run = sb.begin(body, {}, { filename: `${name}.js` });
      for (let i = 1; i < k; i++) await step(run);
      writeFileSync(new URL(`${name}.${k}.dump.js`, DIR), run.dump());
      sb.dispose();
    }
  }
});

test("golden host dumps load and finish as their programs do", async () => {
  const module = await loadModule();
  const packagings: [string, Pick<SandboxOptions, "module" | "dumpModule">][] = [["A", { module }]];
  const nodump = process.env.DJS_WASM_NODUMP;
  if (nodump) packagings.push(["B", { module: await loadModule(nodump), dumpModule: module }]);
  for (const name of programs) {
    assert.ok(dumps.some((d) => d.startsWith(`${name}.`)), `${name}: no dump`);
  }
  for (const file of dumps) {
    const m = /^(.+)\.(\d+)\.dump\.js$/.exec(file);
    assert.ok(m && programs.includes(m[1]), `${file}: no program`);
    const [, name, kText] = m;
    const k = Number(kText);
    const body = readFileSync(new URL(`${name}.body.js`, DIR), "utf8");
    const { want, marks } = await uninterrupted(module, name, body);
    assert.ok(k >= 1 && k <= marks.length, `${file}: ${name} has ${marks.length} idle points`);
    const text = readFileSync(new URL(file, DIR), "utf8");
    assert.ok(text.startsWith('"djs-dump";\n'), `${file}: not a dump`);
    if (INLINE.has(name)) assert.doesNotMatch(text, /\$djs\.source\(/, `${file}: inlined`);
    for (const [label, mod] of packagings) {
      const lines = want.slice(0, marks[k - 1]);
      // An inlined dump needs no script; one with references gets it from `sources`.
      const sb = new Sandbox(scope(mod, lines, name, INLINE.has(name) ? undefined : body));
      const run = sb.loadRun(text);
      while (run.state === "idle") await step(run);
      assert.equal(run.state, "done", `${file} (${label}): ${String(run.error)}`);
      lines.push(`result ${JSON.stringify(run.value)}`);
      assert.deepEqual(lines, want, `${file} (${label})`);
      sb.dispose();
    }
  }
});
