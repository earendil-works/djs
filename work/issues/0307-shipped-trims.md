# 0307: Shipped-module trims; legacy decorators go

status: in-progress
agent: pclean
branch: issue/0307-shipped-trims
depends-on: 0306
touches: crates/djs-core/src/compiler (decorators.rs legacy half, ts_meta.rs, strip types), crates/djs-core/src/vm/call_site.rs, crates/djs-core/src/vm/snapshot (describe), crates/djs-wasm, js/src/sandbox.ts, crates/djs-docs, tests/corpus/ts (legacy decorator cases), docs/guide (typescript, js-api), docs/decisions.md
guide: docs/guide/typescript.md, docs/guide/js-api.md

## Goal
Code in the shipped module that pi never reaches:
- **Legacy TypeScript decorators and `emitDecoratorMetadata`** (owner: "can go for sure"):
  delete them entirely (compiler/ts_meta.rs, the legacy half of decorators.rs, the Sandbox
  options, the goal bits, corpus cases, the qjs parity test); standard decorators stay.
  ~5 KB gz est.
- `describeSnapshot` / `djs_snapshot_describe` (~3 KB gz) and `callSite` / `vm_call_site`
  (~1.3 KB gz): into the docs module only (the docs site uses them; the CLI keeps describe
  natively).
- strip-types mode (~1.9 KB gz): native-only (the CLI and its oracle tests use it).

## Acceptance criteria
- [ ] The above; the size ceiling lowered to the measured size; guide and decisions updated.

## Log

## Follow-ups

## Result
