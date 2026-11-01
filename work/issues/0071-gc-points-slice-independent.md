# 0071: GC points and raw images independent of fuel slice size

status: in-progress
agent: num
branch: issue/0071-gc-points-slice-independent
depends-on: -
touches: crates/djs-core/src/vm/ (run start), crates/djs-core/src/interp/ + regexp/builtins (progress records), crates/djs-core/src/heap/ (threshold accounting)

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] Remove the collection at the start of `run`. Progress records of suspended regexps/builtins don't count towards the GC threshold. Raw images are identical at slices max/997/101/1.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
