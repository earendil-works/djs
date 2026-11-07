# 0248: Top-level await, JSON modules and source positions for modules (M8)

status: in-progress
agent: modules
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

## Follow-ups

## Result
