# 0070: Collect and retry at a failing allocation

status: done
agent: heap
branch: issue/0070-gc-retry
depends-on: 0069
touches: crates/djs-core/src/heap/, crates/djs-core/src/interp/ (allocation sites at safepoint-safe points only)

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] `oom.js` (scripts/determinism) at a 2 MB cap prints all `.`; D1601's OOM-point determinism test still passes. Respect the GC contract: retry only where all live values are rooted.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- oom.js at 2 MB failed in `call_intrinsic Set` (Array.prototype.fill's element growth)
  with ~1 MB of collectable garbage. D1700: instruction-level retry. Before `step`, the
  operands of a few allocate-then-commit instructions (put_elem, put_field, define_field,
  add, object, call_intrinsic of 8 property-write/allocation intrinsics) are copied
  aside. On a cap OOM, the loop puts pc/sp and the operands back into the stack (so they
  are rooted), collects, and runs the instruction once more; a second failure throws
  (D1600). Tests: vm/tests.rs `a_failing_allocation_collects_and_retries` (oom.js at
  2/2.5 MB prints 40 dots; identical at slice 7919 and with restores), D1600/D1601 tests
  unchanged. e2e three modes pass.

## Follow-ups

- Native builtins (0063) have no retry: a native frame's allocation failure is an
  immediate OOM. Natives keep their state in frame slots, so retrying their current
  state would be possible where the state allocates before it commits.
- The whitelist could grow (array literals, `concat`, string builtins) when cases show up.

## Result

A failing allocation in a retry-safe instruction now collects and runs the instruction
again (D1700). oom.js at a 2 MB cap prints all `.`. Out-of-memory points stay
deterministic.
