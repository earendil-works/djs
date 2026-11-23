// Code mode bodies import host modules (0292, D3597): leading import declarations bind
// host values the `modules` resolver names, as the body's parameters; dumps name them by
// identity (`specifier#export`) and namespaces by specifier, and load with no rewrite.

import { test } from "node:test";
import assert from "node:assert/strict";
import { type HostModule, loadModule, type Run, Sandbox, SandboxError, type SandboxOptions } from "../src/sandbox.ts";
import { leadingImports } from "../src/imports.ts";

const quiet = { log() {}, info() {}, warn() {}, error() {} };

async function finish(run: Run, answer: (fn: string, args: unknown[]) => unknown): Promise<void> {
  while (run.state === "idle" && run.calls.length > 0) {
    for (const c of run.calls) run.settle(c.id, { ok: true, value: answer(c.fn, c.args) });
    await run.advance();
  }
}

test("leadingImports reads the import declarations before the first statement", () => {
  const code = `#!/usr/bin/env djs
// tools
import { search_issues, "get issue" as get, default as d } from "@mcp/github";
import * as linear from '@mcp/linear'
import base, { x as y } from "base" /* c */ ;
import "side-effect";
import type { T } from "types";
import { type U, v } from "mixed";
const later = 1;
import { never } from "parsed";
`;
  assert.deepEqual(
    leadingImports(code, true).map((d) => [d.specifier, d.bindings.map((b) => [b.name, b.local])]),
    [
      ["@mcp/github", [["search_issues", "search_issues"], ["get issue", "get"], ["default", "d"]]],
      ["@mcp/linear", [["*", "linear"]]],
      ["base", [["default", "base"], ["x", "y"]]],
      ["side-effect", []],
      ["mixed", [["v", "v"]]],
    ],
  );
  assert.deepEqual(leadingImports("const a = import('x'); import.meta;"), []);
  assert.deepEqual(leadingImports("import('x').then(() => 1)"), []);
  assert.throws(() => leadingImports('import { a } from "x" with { type: "json" }'), /attributes/);
  assert.throws(() => leadingImports("import { a from 'x'"), /expecting/);
});

/** The test modules: a module with functions defined on first import, one naming the sandbox's own functions. */
function modules(seen: string[] = []): (spec: string) => HostModule | undefined {
  return (spec) => {
    seen.push(spec);
    if (spec === "@mcp/github") return { exports: { search_issues: {}, get_issue: {}, default: {} } };
    if (spec === "@mcp/linear") return { exports: { create_issue: {}, list: {} } };
    if (spec === "local") return { exports: { read: "tools.read", now: { sync: true, fn: () => 7 } } };
    if (spec === "tools") return { exports: { read: "tools.read" } };
    return undefined;
  };
}

test("imports bind host functions and namespaces; the resolver is asked once, lazily", async () => {
  const seen: string[] = [];
  const sb = new Sandbox({
    module: await loadModule(),
    console: quiet,
    asyncFunctions: { "tools.read": () => 0 },
    modules: modules(seen),
  });
  const code = `
    import { search_issues, get_issue as get } from "@mcp/github";
    import gh from "@mcp/github";
    import * as linear from "@mcp/linear";
    import { read, now } from "local";
    const hits = await search_issues({ q: "dumps" });
    const made = await linear.create_issue({ title: hits[0] });
    return [hits, made, now(), read === tools.read, typeof get, typeof gh, Object.keys(linear), Object.isFrozen(linear),
      linear.create_issue === linear.create_issue, typeof search_issues];
  `;
  const run = sb.begin(code, { extra: 1 });
  assert.deepEqual(run.calls.map((c) => [c.fn, c.args]), [["@mcp/github#search_issues", [{ q: "dumps" }]]]);
  await finish(run, (fn) => (fn === "@mcp/github#search_issues" ? ["issue 1"] : "made"));
  assert.equal(run.state, "done", String(run.error));
  assert.deepEqual(run.value, [["issue 1"], "made", 7, true, "function", "function", ["create_issue", "list"], true, true, "function"]);
  // Asked once per specifier, only for what is imported.
  const again = sb.begin(`import { list } from "@mcp/linear"; return typeof list;`);
  assert.equal(again.value, "function");
  assert.deepEqual(seen, ["@mcp/github", "@mcp/linear", "local"]);
  sb.dispose();
});

test("an unknown module or export, a duplicate name or a misplaced import is a syntax error", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, modules: modules(), bindings: {}, asyncFunctions: { "tools.read": () => 0 } });
  const syntax = (re: RegExp) => (e: unknown) => e instanceof SandboxError && e.kind === "syntax" && re.test(e.message);
  assert.throws(() => sb.begin(`import { a } from "@mcp/nope";`), syntax(/^SyntaxError: Cannot find module '@mcp\/nope' at 1:19$/));
  assert.throws(() => sb.begin(`\nimport { nope } from "@mcp/github";`), syntax(/does not provide an export named 'nope' at 2:10/));
  assert.throws(() => sb.begin(`import { list } from "@mcp/linear"; import { list } from "@mcp/linear";`), syntax(/'list' has already been declared/));
  assert.throws(() => sb.begin(`import { list } from "@mcp/linear";`, { list: 1 }), syntax(/'list' has already been declared/));
  assert.throws(() => sb.begin(`const a = 1;\nimport { list } from "@mcp/linear";`), syntax(/SyntaxError/));
  // A namespace import of a specifier that names another host value.
  assert.throws(() => sb.begin(`import * as t from "tools";`), syntax(/cannot be imported as a namespace/));
  assert.equal(sb.begin(`import { read } from "tools"; return read === tools.read;`).value, true);
  const none = new Sandbox({ module: await loadModule(), console: quiet });
  assert.throws(() => none.run(`import { a } from "x"; return a`), syntax(/Cannot find module 'x'/));
  // `import()` rejects: host modules are imported statically.
  await assert.rejects(none.runAsync(`return await import("@mcp/github")`), /could not load module/);
  none.dispose();
  sb.dispose();
});

test("TypeScript bodies erase type-only imports", async () => {
  const sb = new Sandbox({ module: await loadModule(), console: quiet, typescript: true, modules: modules() });
  const code = `import type { Issue } from "@mcp/github";\nimport { type Other, list } from "@mcp/linear";\nconst n: number = 1;\nreturn [typeof list, n];`;
  assert.deepEqual(sb.run(code), ["function", 1]);
  sb.dispose();
});

test("a run with imports dumps at every idle point and loads with no rewrite", async () => {
  const code = `// imports, then the body
import { search_issues } from "@mcp/github";
import * as linear from "@mcp/linear";
const search = search_issues;
const hits = await search({ q: "a" });
const ns = linear;
const made = await Promise.all([linear.create_issue({ t: hits }), ns.list()]);
return [made, search === search_issues, ns === linear, Object.keys(ns)];`;
  const answer = (fn: string, args: unknown[]) => `${fn} ${JSON.stringify(args)}`;
  const want = [
    ['@mcp/linear#create_issue [{"t":"@mcp/github#search_issues [{\\"q\\":\\"a\\"}]"}]', "@mcp/linear#list []"],
    true,
    true,
    ["create_issue", "list"],
  ];
  const module = await loadModule();
  for (const inline of [false, true]) {
    for (const json of [false, true]) {
      const opts = (): SandboxOptions => ({
        module,
        frozenBuiltins: true,
        console: quiet,
        inlineSources: inline,
        jsonBoundary: json,
        modules: modules(),
        sources: (n) => (n === "imports.js" ? code : undefined),
      });
      for (let k = 0; k < 2; k++) {
        const what = `inline ${inline} json ${json} at idle point ${k + 1}`;
        const sb = new Sandbox(opts());
        const run = sb.begin(code, {}, { filename: "imports.js" });
        for (let i = 0; i < k; i++) {
          for (const c of run.calls) run.settle(c.id, { ok: true, value: answer(c.fn, c.args) });
          await run.advance();
        }
        const dump = run.dump();
        if (!inline) assert.match(dump, /\$djs\.source\("imports\.js", "[0-9a-f]{8}", "search_issues,linear"\)/, what);
        else assert.doesNotMatch(dump, /\bimport\s*\{/, `${what}: the inlined body has no import`);
        if (k === 1) assert.match(dump, /\$djs\.host\("@mcp\/linear"\)/, what);
        assert.match(dump, /\$djs\.host\("@mcp\/github#search_issues"\)/, what);
        sb.dispose();
        // A fresh sandbox has made none of the modules' values: the load makes them.
        const again = new Sandbox(opts());
        const resumed = again.loadRun(dump);
        await finish(resumed, answer);
        assert.equal(resumed.state, "done", `${what}: ${String(resumed.error)}`);
        assert.deepEqual(resumed.value, want, what);
        again.dispose();
      }
    }
  }
});

test("a dump that names a module the loading sandbox cannot resolve does not load", async () => {
  const code = `import { search_issues } from "@mcp/github"; return await search_issues();`;
  const sb = new Sandbox({ module: await loadModule(), frozenBuiltins: true, console: quiet, modules: modules(), inlineSources: true });
  const dump = sb.begin(code).dump();
  sb.dispose();
  const without = new Sandbox({ module: await loadModule(), frozenBuiltins: true, console: quiet });
  assert.throws(() => without.loadRun(dump), /search_issues/);
  without.dispose();
});
