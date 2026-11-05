# 0232: Performance: Promise.all, await, async calls

status: open
agent: -
branch: issue/0232-perf-async
depends-on: -
guide: suspension.md / interpreter.md if the async story changes
touches: builtins (promise), interp (await, async function call/resume), job queue

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- Promise.all over 20 promises 3.1-3.4x slower; await of a resolved value 1.9x; async function
  call + await 1.7x.
Codemode is async-heavy (fan-out over tools is 22% of real bodies), so this matters. Find the root
causes (reaction records, job queue, frame suspend/resume, Promise.all's per-element closures)
and fix; target parity with QuickJS. Dumps must keep working (the plain-dump difftest leg and
fuzz regress: node scripts/plain-fuzz/fuzz.mjs regress, with and without --lazy).
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

## Follow-ups

## Result
