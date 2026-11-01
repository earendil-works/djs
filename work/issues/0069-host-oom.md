# 0069: Running out of host memory is not a JS out-of-memory error

status: open
agent: -
branch: issue/0069-host-oom
depends-on: -
touches: crates/djs-core/src/heap/ (arena growth, GC to-space), crates/djs-core/src/vm/, crates/djs-wasm/, js/src/sandbox.ts

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] `HeapError::HostOutOfMemory` for arena growth and GC to-space failures. The run stops with its own uncatchable status, never a catchable error or a silently skipped collection.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
