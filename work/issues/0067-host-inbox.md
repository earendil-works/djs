# 0067: Async host results as macrotasks (the host inbox)

status: in-progress
agent: determinism
branch: issue/0067-host-inbox
depends-on: -
touches: crates/djs-core/src/vm/, crates/djs-core/src/interp/ (job queue / run loop), crates/djs-core/src/heap/ (an inbox root, additive), crates/djs-wasm/, js/src/sandbox.ts, docs/host.md, docs/compiler.md 7.5

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] `resolve_host_call` appends to a heap inbox. `run` applies one result only when the job queue is empty, drains it, then takes the next. Unhandled rejections are checked after each result. Optional `resolve_host_call_at` carries a clock value. `async_interleave.js` and `sandbox_interleave.ts` (scripts/determinism) give the same output for any slice size and delivery timing, batched delivery behaves like separate deliveries, and an image with queued results restores. Amends D1374/D1370 (record a decision).
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
