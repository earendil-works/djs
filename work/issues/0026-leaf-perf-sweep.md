# 0026: Leaf performance sweep (arena inlining, opt-level, allocation, stringify)

status: in-progress
agent: num
branch: issue/0026-leaf-perf-sweep
depends-on: 0023
touches: crates/djs-core/src/heap/ (arena accessors, allocation), crates/djs-core/src/json/, crates/djs-core/src/wire/, Cargo.toml ([profile.wasm] only), crates/djs-bench/ (new benchmarks)

## Goal
Collected performance follow-ups from 0012, 0018, 0021 and 0023 that are all about the
leaf layer (the arena and the data paths), measured both natively and in wasm.

## Acceptance criteria
- [ ] `#[inline(always)]` (or restructuring) on the hot arena accessors
      (`Arena::read/write::<T>`, `layout::read_array`, …). Re-measure at opt-level z and
      s in wasm. If z gets within ~10% of s, propose switching back (with numbers;
      coordinator decision).
- [ ] Allocation without zero-fill where the caller initializes every byte, and arena
      reserve costs (0018 follow-up: they dominate object decoding and JSON parse).
- [ ] `json::stringify_fast` on number arrays (0021: 11 ms for 500k) and on object-heavy
      data (0012: ~50% of node). Profile and fix what is cheap.
- [ ] Benchmarks: add the wire decode and the reader-side benchmarks to djs-bench (0019
      follow-up), and wasm variants via the size/engine tracks or a new `wasm` group
      that runs in node, if practical.
- [ ] Record before/after in tests/scores/bench.tsv (`scripts/bench.sh --record` on main
      before and after).
- [ ] No behavior changes: all corpora (num, json, regexp, wire, reader) still pass.

## Notes
- 0022 (heap agent) is working in interp/ and vm/ at the same time. Stay out of those.
- The machine is shared with other agents compiling, so measure carefully (D1190 notes
  on noise) and repeat runs.

## Log

## Follow-ups

## Result
