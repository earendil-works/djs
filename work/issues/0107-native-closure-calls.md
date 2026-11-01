# 0107: Direct native-to-closure calls; cold arms out of line

status: open
agent: -
branch: issue/0107-native-closure-calls
depends-on: 0100
guide: - (the interpreter chapter, once docs writes it: tell the coordinator what changed)
touches: crates/djs-core/src/interp/run/ (native.rs, hot.rs, call paths, intrinsics dispatch), crates/djs-bgen/src/emit_rust.rs (only if the call sites change), crates/djs-core/src/builtins/gen/ (regenerated)

## Goal
0100's findings: every callback call from a native builtin ends the native's run, so the
call protocol (Exec::call, frame push, return into the native) is what's left of the
callback builtins' gap (pi-like ~1.5x quickjs-wasi). Build a direct native-to-closure call
path that shares run_hot's frame entry and returns straight into the native's resume state.
Also num's 0104 size suggestion for this area: move the cold arms of call_intrinsic (37.5 KB),
step_once (35.5 KB) and json_regexp_intrinsic (14.3 KB) out of line, and the remaining
run_hot candidates (object, global-cache misses, concat) where they measure well.

## Acceptance criteria
- [ ] Per-element and engine-track numbers, wasm first (the callback workloads:
      forEach/map/filter/sort/reduce/find, higher-order, pi-like); invisible to images,
      fuel, stress and determinism checks (hot_loop_is_invisible extended).
- [ ] Size before/after; the cold-arm moves must not cost speed.

## Log

## Follow-ups

## Result
