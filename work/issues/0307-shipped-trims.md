# 0307: Shipped-module trims; legacy decorators go

status: done
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
- [x] The above; the size ceiling lowered to the measured size; guide and decisions updated.

## Log
- Legacy decorators (D3750): removed compiler/ts_meta.rs, the thunk half of decorators.rs
  (Thunk, DecParams, LegacyElem, the emit/metadata functions) and its uses in class.rs,
  function.rs, ts.rs, parser.rs; the builtins ApplyLegacyDecorators, DecoratorParam,
  DecoratorParamCall (builtin closure kind 24), DecoratorMetadata and the
  DecoratorParamData record; CompileOptions' two booleans (Script::OPTIONS: privileged
  bit 3, debugger 4); vm_compile_file goal bits 4/8 (debugger 16 -> 4), ABI 14; Sandbox
  options; the CLI's two flags and the REPL's `.decorators` (outside `touches:`, but they
  only fed the removed options); difftest's directives and job fields, qjs_driver.js; the
  four tests/corpus/ts/decorators-legacy* cases; ts_tests' legacy tests (the error cases
  that hold for standard decorators stay as `decorator_errors`). Parameter decorators stay
  a SyntaxError with qjs's message, for parity. regen: intrinsics renumbered, registry,
  layout, prelude.
- describe/callSite (D3751): `vm_call_site` and `djs_snapshot_describe` behind djs-wasm's
  `docs` feature (op numbers kept). Sandbox methods throw `misuse` without the op. The
  gate's `build:wasm` (dev module, js tests) adds `docs`. Tests: js/test/docs-module.ts
  detects the module; snapshot/debugger/sleep-snapshot tests check the refusal on the
  shipped module (checked by running js/test on a dev module without docs: all pass).
  npm-test.sh js-tests copies docs-module.ts. The sleep-snapshot example no longer
  describes the snapshot (it runs on the shipped module); its README shows `djs describe`
  on the file (checked: the CLI reads the wasm-written snapshot).
- strip-types (D3752): new djs-core feature `strip-types` (default on, djs-cli on,
  djs-wasm off, not in the build hash); `Parser::stripping()` and `erased_mut()` are
  constant-false without it; `strip_types` and `ts::blank` are cfg'd. `strip_import` stays
  (codemode body imports, D3597).
- Sizes (gzip -9 of the shipped module, as the gate's build:size): main fedf4616
  509,843 B -> 498,108 B (-11,735 B; raw 1,190,940 -> 1,161,233). Parts measured on this
  branch: strip-types 1,831 B gz (4,402 raw), describe + call site 4,076 B gz (11,233 raw),
  so legacy decorators about 5.8 KB gz. Ceiling 511,676 -> 499,132
  (tests/scores/size.tsv); status sizes recorded.
- Guide: typescript.md (legacy decorators not supported), js-api.md (options table;
  describe/callSite need the docs module), cli.md (flags, `.decorators`), host.md (ops
  table, the module builds), persistence.md. Also ARCH.md, TESTING.md, docs/cli.md,
  docs/host.md, docs/compiler.md 7.11.

## Follow-ups
- docs/codemode-feature-usage.md still lists "TS legacy" decorators in its survey table
  (a research document; 0308's archive cleanup may take it).
- docs/decisions-index.md: D1651 superseded by D3750; D3750-D3752 to add (0308).

## Result
Landed: TypeScript legacy decorators and `emitDecoratorMetadata` are gone (compiler,
builtins, options, CLI, difftest, corpus); `describeSnapshot`/`callSite` exist only in the
docs module (djs-wasm `docs`; the shipped module's Sandbox throws `misuse`); strip-types
is a native djs-core feature. Shipped module 509,843 -> 498,108 B gzip; ceiling 499,132.
ABI 14 (vm_compile_file goal bits: 1 body, 2 TypeScript, 4 `debugger;` pauses).

What pi must change: nothing in its code (run.ts passes only `typescript: true` and calls
neither `describeSnapshot` nor `callSite`); it takes the new package, whose module and
sandbox.ts move together to ABI 14. Snapshots from the previous build do not load (the
build hash and the renumbered intrinsics); a deterministic run falls back to re-running
from its deliveries (D3746).

Known gaps: none. Parameter decorators keep qjs's message, which names
`experimentalDecorators`.
