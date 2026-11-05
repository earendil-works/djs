# 0234: Memory footprint: idle and per-run linear memory vs QuickJS

status: open
agent: -
branch: issue/0234-memory-footprint
depends-on: -
guide: gc.md, limits.md, arena.md if behaviour changes
touches: heap (GC thresholds, arena growth), the base image layout, djs-wasm memory growth; research first

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
Linear memory after a run, djs vs QuickJS: idle 0.6 vs 0.3 MiB; sequential 0.9 vs 0.4; text
1.3 vs 0.6; json 1.8 vs 1.1; heavy 4.8 vs 2.4 MiB. Suspected causes: the base image in the arena,
the tracing GC's threshold (next collection at ~2x live) vs QuickJS's refcounting freeing at once,
a 950 KB __DATA string and split arrays kept until a collection, wasm memory growth granularity.
Research first: account for the bytes (base image, arena slack, malloc'd buffers outside the
arena, wasm pages), send the coordinator a short ranked list with costs (GC tuning trades CPU for
memory: measure both), then fix the clear wins.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

## Follow-ups

## Result
