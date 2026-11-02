# 0154: Slow spots found by pico's benchmarks

status: open
agent: -
branch: issue/0154-slow-spots
depends-on: 0137, 0152
guide: -
touches: crates/djs-core/src/interp/run/iter.rs, crates/djs-core/src/builtins/ (Array.from, callback loops), crates/djs-core/src/vm/ (leaf intrinsic call path); bloat (0151 L4) changes bytecode operand encoding in compiler/ and interp/ run_hot: coordinate via the coordinator

## Goal
docs/lessons-from-pico.md lesson 1:
1. `[...Array(n).keys()]` / `Array.from(Array(n).keys())` are 26x slower than quickjs-wasi
   (562 vs 21.6 ms for 1M): the D1800 fast iteration path gives up on holes even for keys
   iterators, which never read the element.
2. `Array.from({length: n}, f)` 2.9x slower.
3. `map`/`reduce` on 0–2 element arrays 2x slower (0137 follow-up 1: leaf intrinsic calls
   from bytecode go step_once → step → call_intrinsic; filter checks species twice).

## Acceptance criteria
- [ ] Each fixed or explained, measured in wasm vs quickjs-wasi (pico's cases rewritten in
      work/scratch/0152), corpus cases added for the fixed paths.
- [ ] No workload slower than +1%; size ratchet respected.

## Log

## Follow-ups

## Result
