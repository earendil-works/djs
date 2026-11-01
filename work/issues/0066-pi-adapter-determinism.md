# 0066: pi adapter determinism fixes

status: open
agent: -
branch: issue/0066-pi-adapter-determinism
depends-on: -
touches: js/pi/, js/test/, docs/host.md

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] Cold, cache-filling and warm starts give the same interrupt poll count, and the same timeout outcome at a budget between the old counts (a js/test case). The fingerprint includes a hash of the adapter's injected HOST_GLOBALS code. docs/host.md: the stale WeakRef row and the interrupt-budget notes fixed.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
