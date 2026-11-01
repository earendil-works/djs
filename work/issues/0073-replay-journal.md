# 0073: Replay journal and image hash

status: in-progress
agent: determinism
branch: issue/0073-replay-journal
depends-on: 0067
touches: crates/djs-core/src/vm/, crates/djs-wasm/, js/src/sandbox.ts, scripts/ or a tool crate

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] `Vm::canonical_hash()` that doesn't modify the VM, a journal format, a record/replay tool that checks the calls issued.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
