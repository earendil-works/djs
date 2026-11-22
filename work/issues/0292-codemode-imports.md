# 0292: Code mode bodies import host modules: `import { x } from "@mcp/github"`, namespaces, dumps

status: in-progress
agent: pdurable
branch: issue/0292-codemode-imports
depends-on: 0291
touches: crates/djs-core/src/compiler/ (body goal with imports), crates/djs-core/src/vm/ (linking host modules, dump of import bindings), js/src/sandbox.ts, js/test/, tests/dump/, docs/decisions.md, docs/guide
guide: docs/guide/js-api.md, docs/guide/dumps.md, docs/guide/language.md, docs/guide/host.md

## Goal
In pi, `tools.foo` will be reserved for basic harness tools; MCP servers and other
capabilities will be imported from virtual modules that the agent discovers by reading
their (virtual) `.d.ts` files:

```js
import { search_issues } from "@mcp/github";
import * as linear from "@mcp/linear";
const hits = await search_issues({ q: "dumps" });
return await linear.create_issue({ title: hits[0].title });
```

The host decides what a specifier means; djs binds the imports to host functions and
host namespace objects. A run that imported them must dump and load like any other:
an imported function is a host value by identity (D2970-D2972: `specifier#export` is the
documented convention), a namespace import a host root (`$djs.host("@mcp/linear")`).

The 0291 prototype fakes this in pi by blanking leading import lines and passing the
names as body bindings; that bakes the rewrite into every dump's source hash and handles
only `import { a, b as c } from "x"`. This issue replaces it.

## Acceptance criteria
- [ ] Code mode bodies (the async body goal with top-level `return`/`await`) accept static
      import declarations: named, renamed, namespace (`* as ns`), `import type` (TypeScript,
      erased) and default if the host module has one. Decide and record the goal: imports
      in a body are not ECMAScript modules (no `export`, the body still returns); say how
      they differ.
- [ ] A host resolver, called lazily for the specifiers a script imports:
      `(specifier) => { exports: Record<exportName, identity> } | undefined` (or similar),
      so the host never lists every module up front. Unknown module or export: an early
      error the host sees as a compile error (a SyntaxError with the specifier/name).
- [ ] Namespace imports give one frozen namespace object per specifier per VM, a host
      root in dumps; `===` across dump and load holds for functions and namespaces.
- [ ] `import("spec")` of a host module resolves to that namespace (or a clear error), so
      scripts can probe lazily.
- [ ] Dumps of runs with imports load with no source rewrite on the host side; golden dumps
      added. The source hash covers the script as written.
- [ ] Guide: language.md (what code mode bodies accept), js-api.md (the resolver), dumps.md
      (imports in dumps). A worked example of a host that serves `.d.ts` text for its
      modules from the same descriptor that feeds the resolver (host side only; djs does
      not read `.d.ts`).

## Notes
- djs-core already compiles ES modules (compiler/module.rs, vm/module.rs); the dumper knows
  no module records (BOARD), which is why imports should bind host values, not module
  records.
- Coordinate the Sandbox option names with the 0291 result; the pi prototype will switch
  to it right after the merge.

## Log

## Follow-ups

## Result
