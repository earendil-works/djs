# 0100: Generated native builtins keep values in Rust locals (0081 P8) + more run_hot ops

status: open
agent: -
branch: issue/0100-native-codegen-locals
depends-on: 0093
touches: crates/djs-bgen/src/emit_rust.rs (codegen), crates/djs-core/src/builtins/gen/natives.rs (regenerated), crates/djs-core/src/interp/run/native.rs, crates/djs-core/src/interp/run/hot.rs

## Goal
After 0093, plain bytecode is at 0.44–0.63x quickjs-wasi and calls ~1x; pi-like is ~1.5x,
and what's left there is the builtin share: generated natives still cost hundreds of
instructions per element (0081 F3: every local a checked arena slot with a NaN-box round
trip, fast-path results pushed then popped, out-of-line slot accesses in wasm, 6 state arms
per element). 0081 P8: keep values in Rust locals between resume points (they must be in
frame slots only across resume points, since GC and suspension happen only there, D1632).
Also the next run_hot candidates from 0093 (object, global-cache misses, concat).

## Acceptance criteria
- [ ] bgen's Rust backend keeps locals and temporaries in Rust variables within a state
      arm and across arms that can't suspend or collect, spilling to frame slots only
      before resume points (calls that may run user code, allocation points that may
      collect: check the D1632 contract), with the D1700/D1900 retry rules unchanged.
- [ ] Per-element instruction counts for the native builtins (perelem.py) and the engine
      track, wasm first; both backends identical; stress and determinism checks
      identical; code size per native (scripts/native-sizes.sh), which should go down.
- [ ] The next run_hot ops if measurement supports them.
- [ ] Coordinate with infra (bgen and the natives are its code) through the coordinator.

## Log

## Follow-ups

## Result
