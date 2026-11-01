# 0137: Per-element cost of callbacks from natives (map/filter/forEach) without size growth

status: in-progress
agent: perf
branch: issue/0137-callback-cost
depends-on: 0129
guide: docs/guide/builtins.md / interpreter.md if the call mechanism changes
touches: crates/djs-core/src/vm/ (native→bytecode call path, run_hot re-entry), crates/djs-bgen/ (native accessor glue), crates/djs-core/src/builtins/ (array iteration natives); heap (0136) is in heap/

## Goal
After 0129, pi-like is 0.94x quickjs-wasi but callback-heavy code is not: higher-order
2.6x, foreach 1.36x per element. About a third is the out-of-line native accessors
(D1634); inlining them costs +4.6 to +10.7 KB gz (owner trade, pending). Find wins that
don't grow the module: e.g. a dedicated callback loop (the native sets up one frame and
re-enters the bytecode function per element without the generic call machinery), fewer
accessor calls per element (hoisting checks out of the loop), cheaper argument passing
for 1–3 arg callbacks, or moving the hot iteration loops into bytecode intrinsics if that
measures better.

## Acceptance criteria
- [ ] Profile of the per-element cost in wasm, split by component.
- [ ] Wins landed within the size ceiling (or offset), no workload slower than +1%.
- [ ] Updated numbers for the inlining trade on top of whatever lands, for the owner.

## Log

## Follow-ups

## Result
