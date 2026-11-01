# 0075: Build identity and Float32 NaN hardening

status: open
agent: -
branch: issue/0075-build-identity
depends-on: -
touches: crates/djs-core/build.rs, crates/djs-core/src/builtins/ (typed arrays, Math.fround)

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] Cargo features and the libm version in BUILD_HASH. Float32 encoding and `Math.fround` write the canonical f32 NaN explicitly. Small.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

## Follow-ups

## Result
