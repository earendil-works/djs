# 0166: Heap follow-ups: parser depth, GC threshold rounding, stack-overflow errors

status: done
agent: heap
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
- Item 1: probes (work/scratch/0166: nesting-probe.ts, regex-probe.ts) on debug and
  release builds with the limits lifted. D2521's 16-word push_out cap was already in
  0158's last measurement: blocks/trys still trap at 611/610 levels in debug, so the
  parser's 550 is at the 10% rule (≤ ~554). Regexp: 201 → 212 (211 groups; release
  lookaheads trap at 236). D2545.
- Item 2: thresholds round from BASE_END (D2544); the arena target keeps whole pages from
  offset 0 (without that strbuild +0.6 MiB). mem-perturb n=8 and instructions n=5 against
  main: numbers in D2544. Rejected: unrounded target, no rounding.
- Item 3: djs-wasm exports `__stack_pointer`; js/src/trap.ts `trapText` reports a stack
  overflow when the pointer wrapped past memory's end, before calling any export (the
  old path trapped again in `djs_panic_ptr`); Sandbox and pi engine use it; pi engine's
  dispose skips a trapped instance. Verified with a no-limit build (sp 0xFFFFFFC0 →
  "stack overflow: the engine's wasm stack is exhausted"); js/test/trap.test.ts. D2543.
- Guide: gc.md (thresholds from the prefix's end), host.md (stack overflow report).

## Follow-ups

## Result
- Parser MAX_DEPTH stays 550 (at the 10% rule); regexp nesting limit 212 (211 groups).
- GC thresholds count pages from BASE_END; the arena target stays page-aligned: base
  size no longer moves collection points; json-indent −0.8 MiB, json-replacer −0.64,
  pi-like −0.2, strbuild +0.2; instructions within ±1.4%.
- A wasm stack overflow is reported as "stack overflow: the engine's wasm stack is
  exhausted" (SandboxError kind `trap`, pi EngineTrap); needs a module with the
  `__stack_pointer` export (older modules report as before).
