# 0292: Code mode bodies import host modules: `import { x } from "@mcp/github"`, namespaces, dumps

status: done
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
- [x] Code mode bodies (the async body goal with top-level `return`/`await`) accept static
      import declarations: named, renamed, namespace (`* as ns`), `import type` (TypeScript,
      erased) and default if the host module has one. Decide and record the goal: imports
      in a body are not ECMAScript modules (no `export`, the body still returns); say how
      they differ.
- [x] A host resolver, called lazily for the specifiers a script imports:
      `(specifier) => { exports: Record<exportName, identity> } | undefined` (or similar),
      so the host never lists every module up front. Unknown module or export: an early
      error the host sees as a compile error (a SyntaxError with the specifier/name).
- [x] Namespace imports give one frozen namespace object per specifier per VM, a host
      root in dumps; `===` across dump and load holds for functions and namespaces.
- [x] `import("spec")` of a host module resolves to that namespace (or a clear error), so
      scripts can probe lazily. (Done as the clear error: it rejects; see Follow-ups.)
- [x] Dumps of runs with imports load with no source rewrite on the host side; golden dumps
      added. The source hash covers the script as written.
- [x] Guide: language.md (what code mode bodies accept), js-api.md (the resolver), dumps.md
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
- Design (D3597): leading import declarations of a body are skipped by the engine
  (`Parser::skip_body_imports`, reusing the strip parser's `strip_import`, which the shipped
  module has already: +49 B gz, 432,882 of the 432,899 ceiling) and read by the Sandbox
  (js/src/imports.ts), which binds each name as a body parameter after `params` and
  `bindings`. The script compiles as written; `$djs.source`'s hash and params (which name
  the imports) come from the script, so no host rewrite. ES module records were rejected
  (no module code in djs-wasm; the dumper knows none); an engine-side import list too
  (wasm size).
- `SandboxOptions.modules(specifier) => { exports: { name: identity | { fn?, sync? } } }`,
  asked once per specifier; exports defined lazily as `specifier#export`; namespaces one
  frozen object per specifier (enumerable members), a host root named by the specifier.
  Errors: `syntax` SandboxErrors with positions (Cannot find module, does not provide an
  export named, already declared).
- Dumps: K/origin name imports like parameters (`$djs.host("spec#x")`, `$djs.host("spec")`);
  the dumper's body text (`Sources::fn_text`) blanks the import declarations
  (`compiler::body_imports_end`, dump builds only) so the inlined body and the maps compile.
  `loadDump` pre-scans the dump's `$djs.host`/`$djs.resume` names to make module values
  before the Script runs (the helper cannot run while it does).
- `import()` stays a rejection ("could not load module"): the acceptance's "or a clear
  error"; see Follow-ups.
- Tests: compiler::tests::body_imports_*, js/test/imports.test.ts (parser, resolver,
  errors, TS, dumps at every idle point inline/refs x JSON boundary x both packagings, a
  load without the module), golden host dumps `imports` (refs) and `imports-inline`.
- Guide: language.md ("Imports in a body", the modules paragraph and the left-out table),
  js-api.md ("Host modules" with the resolver and a `.d.ts` example from the same
  descriptor), dumps.md ("Imports"), host.md (compiling a body with imports), pi.md (MCP
  servers as host modules).

## Follow-ups
- `import("spec")` of a host module: needs an engine hook from the prelude's
  `DynamicImport` to the host (a realm slot or intrinsic, base size); today it rejects.
- Import bindings are assignable parameters; read-only would need the engine to read the
  clause names (a const flag on those parameters).
- `import x = require()` in a TypeScript body now reports "expecting 'from'" instead of
  the CommonJS message (the skip runs before the statement parser); a check costs bytes
  the size ceiling does not have.

## Result
Static imports of host modules in codemode bodies: named, renamed, string names,
namespace, default, side-effect, `import type`. API: `SandboxOptions.modules` (types
`HostModule`, `HostModuleFunction`). Dumps of runs with imports load with no rewrite
(source refs and inlined), golden dumps added. D3597. Known gaps: see Follow-ups.
