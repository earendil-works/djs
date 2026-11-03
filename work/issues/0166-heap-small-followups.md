# 0166: Heap follow-ups: parser depth, GC threshold rounding, stack-overflow errors

status: open
agent: -
branch: issue/0166-heap-followups
depends-on: 0158, 0161, 0151 L4
guide: docs/guide/gc.md, host.md as affected
touches: crates/djs-core/src/compiler/parser.rs (MAX_DEPTH), crates/djs-core/src/regexp/ (depth), crates/djs-core/src/heap/gc.rs (next_threshold rounding), js/src/sandbox.ts + js/pi/ (error mapping), crates/djs-wasm (if needed)

## Goal
Small follow-ups collected from 0151, 0158, 0161:
1. **Parser MAX_DEPTH 550 (D2540)** was lowered partly because 0161's push_out of an 832 B
   FuncState grew debug parser frames; D2521's 16-word limit removed that. Re-measure the
   debug and release wasm nesting limits per construct and raise MAX_DEPTH (and the
   regexp limit) as far as the 10% stack-spare rule allows.
2. **GC thresholds rounded to 64 KiB pages from offset 0** make collection points depend
   on the base size (json-date's phase flips in D2443/D2445). Round the growth relative to
   BASE_END (or don't round the growth term) so shrinking the base no longer shifts
   collection points; measure with mem-perturb and instructions.
3. **A wasm stack overflow** reaches JS as a raw `RuntimeError: memory access out of
   bounds` instead of a SandboxError: map it in the Sandbox and the pi adapter to a clear
   error (and check whether it can be distinguished from other traps).

## Log

## Follow-ups

## Result
