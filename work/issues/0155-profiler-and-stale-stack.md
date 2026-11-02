# 0155: Fix the instruction profiler; no stale stack values in images

status: in-progress
agent: perf
branch: issue/0155-profiler-stack
depends-on: 0153
guide: docs/guide/snapshots.md / determinism.md if image content rules change
touches: scripts/perf/instr.py (+ its patches), scripts/perf/README.md, crates/djs-core/src/vm/ (image/session writers: session.rs, bytes, canonical copy), crates/djs-core/src/heap/ (only if the clearing belongs in the collector), tests; bloat (0151 L4) works in compiler/ and interp/ run_hot: coordinate via the coordinator before touching interp/

## Goal
jit's 0153 follow-ups:
1. **scripts/perf/instr.py is broken**: its `gc.rs` patch no longer applies (the collector
   changed in 0120–0148) and its fast-path counters read zero. Make it work on current main
   and robust against the next refactor (patch by anchors that are checked, fail loudly if a
   patch doesn't apply, a self-test in the script), and split interpreter time between user
   code and bytecode builtins (the split 0153 couldn't get).
2. **Stale values above `sp` in images**: images (and so session images, containers,
   canonical_hash input) include dead value-stack slots above the stack pointer. Equal
   histories still give equal bytes, but the dead values keep cells alive or at least
   cost bytes and make images depend on garbage. Clear them so images contain no data
   above `sp` (and other dead per-VM scratch, if the analyzer shows any: check `djs heap`
   on a REPL and a pi idle snapshot). Prefer doing it where the image is written (no
   runtime cost) if that's sound for the collector (dead slots must not be traced as
   roots); otherwise at suspension. Measure session-image and container sizes before and
   after, and check determinism/canonical_hash and the prefix barrier.

## Acceptance criteria
- [ ] instr.py runs on main, with the user-code vs builtin split, documented; a check
      that it applies (in its own self-test, not the gate unless cheap).
- [ ] Images have nothing above sp; a test asserts it; sizes reported.
- [ ] No workload slower than +1%; size ratchet respected.

## Log

## Follow-ups

## Result
