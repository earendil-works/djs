# 0248: Top-level await, JSON modules and source positions for modules (M8)

status: done
agent: modules, then coordinator
branch: issue/0248-modules-tla-json
depends-on: 0241, 0242, 0243
touches: crates/djs-core/src/compiler, builtins/module.ts, crates/djs-core/src/vm, crates/djs-embed, crates/djs-test262, tests/test262.toml
guide: docs/guide/language.md, docs/guide/rust-api.md

## Goal

M8 of djs-hooks.md §3, as far as feasible: top-level `await` (async module evaluation per
ECMA-262 16.2.1.5.3: async parents, [[PendingAsyncDependencies]], the evaluation order of
async siblings), JSON modules (`import data from "./x.json" with { type: "json" }`: import
attributes passed to the host loader, the host returns JSON text, the engine makes a synthetic
module with the parsed default export), and a per-module filename and line offset for stack
traces. test262 `top-level-await`, `json-modules`, `import-attributes` into active as far as
supported.

## Log

- (modules agent) Stack traces of module code: a module body's frame is named
  `<anonymous>` like a Script's, positions are the module's own (key:line:col); tested in
  djs-embed (fd0a8453). Per-module filenames were already the keys.
- (coordinator, continuing the agent's uncommitted WIP after the agent was stopped) JSON
  modules (D3245): import attributes parsed in full, the `type` attribute passed to
  `ModuleLoader::resolve`, `ModuleSource { text, kind: ModuleKind }` with `Json`, a synthetic
  module with the parsed default export, mismatch TypeErrors, other attributes a
  SyntaxError at load; `import()` passes `type` as the host call's third argument.
  djs-embed: `ModuleSource::json`, `ModuleMap::json`. Tests: vm::module_tests::json_modules,
  json_modules_through_import, djs-embed modules::json_modules; test262 import-attributes
  and json-modules active (610/610 in those dirs on djs; qjs fails 8). COMPAT_VERSION 8 (the
  record's flags slot).
- (coordinator) Top-level await (D3246): a sync first parse finds [[HasTLA]]
  (`Parser::await_operator`), such a module compiles as an async body; the evaluation
  algorithm with async modules in builtins/module.ts; record slots CYCLE_ROOT, ASYNC_ORDER,
  PENDING, PARENTS, CAPABILITY and flag TLA; reactions as `%ModuleOp` closures with
  `[op, operand]` data (`%NsOp` 8), the evaluation counter `%NsOp` 7; `import()` resolves
  from the evaluation's promise. Tests: vm::module_tests::top_level_await (plain and with an
  image round trip at every 7 fuel), top_level_await_errors,
  top_level_await_in_cycles_and_imports; e2e modules-tla.js (qjs's transcript; the e2e
  module mode now answers `import()`), all four e2e modes; djs-embed
  modules::top_level_await. test262: `top-level-await` active, everything under
  language/module-code, language/import, dynamic-import passes, the 4 ratchet entries that
  waited for TLA are gone (40 left). Sizes: shipped 426,902 gzip (ceiling 426,977), slim
  360,597 (ceiling 360,697): the evaluation code is embed-only.

## Follow-ups
- HTML-like comments are still accepted in module code (0241's follow-up).
- The dumper knows nothing of module records (0241's follow-up).

## Result
M8 done: per-module positions in stack traces (module keys, `<anonymous>` bodies), JSON
modules (D3245: `with { type: "json" }`, the `type` attribute through
`ModuleLoader::resolve`, `ModuleSource { kind: ModuleKind::Json }`; djs-embed
`ModuleSource::json`, `ModuleMap::json`) and top-level await (D3246) per the
specification's async module evaluation. test262 `import-attributes`, `json-modules` and
`top-level-await` are active and pass (39,325 of 39,365 active tests pass overall).
