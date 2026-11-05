# 0231: Performance: sort with a comparator, Array.from, join/concat/push, Map/Set string keys

status: open
agent: -
branch: issue/0231-perf-arrays-strings
depends-on: -
guide: builtins.md if a fast-path story changes
touches: builtins (array, string, collections), interp natives for sort/join, heap string hashing for Map/Set

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- Array.prototype.sort with a comparator 5.3-6.1x slower (native sort re-entering the VM per
  comparison); default string sort 1.7-1.9x.
- Array.from({length}, fn) 2.3x.
- Array.prototype.join of 100k strings 1.7x (3.2x on 30k lines); string a+b concat 2.2x; push
  1.7-1.9x.
- Map.set / Set.add with string keys 2.5-2.7x (string hashing per op?).
Find each root cause and fix; target at least parity with QuickJS.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

## Follow-ups

## Result
