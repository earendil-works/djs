# 0129: pi-like hot spots after GC work (interpreter, native calls, JSON)

status: open
agent: -
branch: issue/0129-pi-like-hot-spots
depends-on: 0116, 0120, 0127
guide: docs/guide/interpreter.md / builtins.md if mechanisms change
touches: crates/djs-core/src/vm/ (run_hot, call paths), crates/djs-core/src/builtins/ (JSON, strings), crates/djs-bgen/ (native call glue); heap owns heap/ and djs-wasm/src/allocator.rs (0128), bloat owns Unicode/fmt (0126)

## Goal
Back to speed, with the size ratchet in force (no growth without a recorded trade).
Re-profile pi-like in wasm on current main (GC is now old-generation, so the
picture changed) and attack the top non-GC costs: the hot interpreter loop on user code
(~6.5%), the native call path (~3.5%), JSON parse tokenizer/quoting/escapes (~8%),
`alloc_latin1_pieces` in slice().join() (2.3%). Also try the one-pass JSON string
scan+copy idea from 0116.

## Acceptance criteria
- [ ] Fresh wasm profile of pi-like recorded.
- [ ] Wins landed with wasm instruction counts on all workloads (no workload slower than
      +1%), size ratchet respected (or trades recorded).
- [ ] Ratio to quickjs-wasi on pi-like re-measured (it was ~1.4–1.5x instructions).

## Log

## Follow-ups

## Result
