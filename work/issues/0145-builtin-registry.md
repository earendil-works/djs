# 0145: Stable, append-only builtin registry

status: done
agent: bloat
branch: issue/0145-builtin-registry
depends-on: 0143, 0144
guide: docs/guide/builtins.md (a short section), snapshots.md if exports use it
touches: crates/djs-prelude/ (registry generation at base build), crates/djs-core/src/vm/ or builtins/ (lookup API: id <-> cell, id <-> spec path), tests/ (a committed registry file + gate check), scripts/; coordinate: perf (0146) and heap (0147) work in heap/, interp/, vm/

## Goal
Owner-approved plan (docs/frozen-builtins.md phase 0, docs/empty-heap.md step 1): every
builtin object and function reachable from the realm (and the internal intrinsics/closure
kinds that exports must name, docs/goldilocks-snapshots.md) gets a stable id keyed by its
spec path (`%Array.prototype.map%`, `%Promise%`, internal: `%PromiseResolveFunction%`...).
The table is committed (e.g. tests/builtins/registry.tsv) and **append-only**: ids never
change or get reused; removed builtins are tombstoned. The gate fails if a build renumbers
or drops an id without a tombstone.

## Acceptance criteria
- [x] Registry generated from the base image build, committed, append-only check in the gate.
- [x] API: id -> cell (in this build's base image) and cell -> id, spec path <-> id; used
      by `djs heap` to print builtin references by path.
- [x] Unit tests; docs (TESTING.md for the gate check, guide section).

## Log
- djs_core::vm::registry (new file in vm/, plus `pub mod registry;`; perf and heap told via
  the coordinator): the file format, spec-style paths (globals, realm roots, registered
  prelude functions, `[[Prototype]]`, captures `~N`), canonical naming by staged
  breadth-first walks, resolving paths on any heap, `check`, `update` (append, tombstone).
- djs-prelude: feature `registry` (default; off in djs-wasm, no module size change),
  build.rs computes base offsets per id, `registry` module API, `registry` example
  (update/check), integration test.
- tests/builtins/registry.tsv: 1,409 entries: 958 values (globals 535, realm roots and
  registered prelude functions, 220 captured helpers) and 451 intrinsics. Unnamed: two
  realm-internal objects (global lexical record, registries), per-VM state.
- Gate: `scripts/registry.py check main` (append-only), union merge + `registry.py merge`
  and the update in scripts/regen.sh; tested on a scratch repository (append, tombstone,
  drop, union-merged tails).
- `djs heap` seeds its path tree with registry paths (`%Array.prototype.map%`).
- Docs: D2371, TESTING.md "Builtin registry", guide builtins.md section, docs/cli.md.

## Follow-ups
- Exports (0142 tier 2, relocations) can name builtins by registry id now; the portable
  prototype (scripts/snapshot-research/portable) still uses its own path walk.
- With 0147's immutable prefix, `djs_prelude::registry::base_offset` names prefix cells
  exactly; `djs heap` could then label by offset instead of resolving paths.
- pi's lockdown turns overridable data properties into accessors, so in a pi heap those
  builtins resolve only under `get `/`set ` paths; moot once pi drops its lockdown (D022).

## Result
tests/builtins/registry.tsv: 1,409 stable ids (958 builtin objects/functions/symbols by
spec-style path, 451 intrinsics by name), append-only with tombstones (D2371).
`scripts/registry-update.sh [reason]` appends and tombstones; a djs-prelude test fails
when the registry is out of date; the gate step `scripts/registry.py check main` fails on
renumbering or dropping; union merges are renumbered by scripts/regen.sh. API:
djs_core::vm::registry (paths, resolve on any heap via `name_cells`, check, update) and
djs_prelude::registry (feature `registry`, not in djs-wasm: path/id/intrinsic lookups and
base-image offsets computed at build). `djs heap` prints builtins as `%Array.prototype.map%`.
Docs: TESTING.md, guide builtins.md, docs/cli.md. Module size unchanged.
