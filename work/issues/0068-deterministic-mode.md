# 0068: Deterministic mode for VM and Sandbox

status: in-progress
agent: determinism
branch: issue/0068-deterministic-mode
depends-on: 0067
touches: crates/djs-core/src/vm/, crates/djs-core/src/heap/ (weak refs: strong or absent; no FR scheduling), crates/djs-wasm/, js/src/sandbox.ts, js/pi/, docs/host.md

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] A VM option: weak refs absent or strong, no finalization scheduling, `Vm::collect` doesn't clear weak refs. Sandbox `deterministic: {startedAt, seed}`: the clock advances only with results, time zone 0. A `randomSeed` option (default: entropy).
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
