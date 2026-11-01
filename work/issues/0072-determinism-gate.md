# 0072: Determinism checks in the gate

status: open
agent: -
branch: issue/0072-determinism-gate
depends-on: part 2: 0067, 0071
touches: crates/djs-core/tests/e2e.rs, crates/djs-difftest/, scripts/determinism/, scripts/gate.sh

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] Part 1 (now): collected-image hashes compared across the e2e modes, a native-vs-wasm image check with the twin drivers, difftest `--determinism`. Part 2: async delivery cases and raw-image equality.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
