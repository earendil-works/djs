# 0243: Synthetic modules, import() through the loader, modules in djs-embed (M4, M5)

status: open
agent: -
branch: issue/0243-module-loader-import-embed
depends-on: 0240, 0241
touches: crates/djs-core/src/vm (module API), crates/djs-core/src/interp, builtins/import.ts, builtins/module.ts, crates/djs-embed, crates/djs-test262 (dynamic-import tests), tests/test262.toml, docs/guide/rust-api.md, docs/guide/language.md
guide: docs/guide/rust-api.md, docs/guide/language.md

## Goal

M4, M5 and the embedding side of M2/M7 (`~/Development/gondolin/native/docs/djs-hooks.md` §3):

- **M4 synthetic modules.** The host defines a module without source: a key and its exports
  (names and values: host functions, wire-decoded data, anything the embed API can make), as
  ECMA-262's Synthetic Module Records (the JSON modules proposal's shape). Importers link to it
  like any module; its exports are live (the host may set them before evaluation).
- **M5 `import()`** goes through the same loader: in a VM with modules, `import(spec)` (in
  modules and scripts; the referrer is the active module's key or the script's name) becomes an
  async host call the host answers by loading a graph (its loader, sync) and handing back the
  module, or an error; the engine links and evaluates it in a later run and settles the promise
  with the namespace (per EvaluateImportCall / ContinueDynamicImport; errors reject). Without
  the feature D1582's behavior stays.
- **djs-embed**: `Source::module(...)` for the policy entry and a loader trait (resolve/load,
  in-memory module maps); synthetic modules from host functions (an `@sdk/net`-style module whose
  exports are host functions and constant data); calling an exported function of the entry
  module as the async entry point (`onRequest` exported from the policy module); snapshots that
  carry the evaluated graph (M6); host access to exports (M7); dynamic `import()` answered by the
  embed loader automatically in the drive loop.
- test262: the dynamic-import tests that become possible (0242's runner) into active.

## Acceptance criteria
- [ ] synthetic modules in djs-core and djs-embed
- [ ] import() through the loader, async, with errors rejecting; referrer passed
- [ ] djs-embed module entry points and snapshots of evaluated graphs, tested
- [ ] test262 dynamic-import group active as far as supported; ratchet updated
- [ ] guide pages

## Log

## Follow-ups

## Result
