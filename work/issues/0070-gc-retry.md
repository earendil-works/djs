# 0070: Collect and retry at a failing allocation

status: open
agent: -
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

## Follow-ups

## Result
