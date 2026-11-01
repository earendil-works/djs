# 0035: Interpreter call-path performance

status: open
agent: -
branch: issue/0035-interp-call-perf
depends-on: 0030 (parts 1-3 merged)
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/ (internal only), crates/djs-bench/ (benchmarks)

## Goal
Calls are the bottleneck of the whole builtin design (D1085: slow paths and builtins are
JS functions). Measured: 3.4M plain JS calls take 0.6 s vs qjs 0.06 s; `sort` with a
comparator on 200k elements takes 1.8 s vs 0.04 s (0034). djs-bench's engine track shows
calls 9.0x, loop 8.5x and props 4.0x slower than qjs (0016). Profile and cut the per-call
and per-instruction overhead, without changing semantics or the suspension/snapshot
properties.

## Acceptance criteria
- [ ] Profiles (native release, and wasm via the 0026 `wasm/` group) of: the calls,
      loop and props microbenchmarks, a comparator sort, and an Array.prototype.map over
      100k elements. Write the top costs in the Log before changing anything.
- [ ] Fixes for what dominates. Candidates (measure first): frame setup/teardown cost,
      re-deriving offsets, bounds checks in the dispatch loop (hoisting, or caching the
      code slice per frame), fuel accounting per instruction, the debug-only checks
      leaking into release, the safepoint check cost, argument copying, `this`
      handling, and intrinsic dispatch overhead for builtin-language code
      (`call_intrinsic` → Rust).
- [ ] No semantic changes: e2e (three modes), difftest and test262 ratchets unchanged
      (or better), and gc-stress / suspension still identical.
- [ ] Numbers before/after in tests/scores/bench.tsv (`scripts/bench.sh --record`), plus
      the djs/qjs ratios for the engine track. Target: a sizable cut in the calls ratio
      (for example 9x → under 4x); report what was achieved and what is structural
      (things that need inline caches, shapes or superinstructions, i.e. later issues).

## Notes
- Interleave measurements (D1190 noise notes); the machine is shared.
- Keep the code readable. A big rewrite of the dispatch loop needs a decision entry
  first: propose it in the Log and ask.

## Log

## Follow-ups

## Result
