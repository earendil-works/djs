# 0233: Performance: regexp exec/replace/matchAll and compile

status: open
agent: -
branch: issue/0233-perf-regexp
depends-on: -
guide: builtins.md (regexp) if it changes
touches: crates/djs-core/src/regexp/, builtins/regexp.ts, string replace/matchAll

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- replace(/\s+/g, ' ') 3.0x slower than Bellard QuickJS (djs beats NG 1.6x); exec of
  /^([^:]+):(\d+):(.*)$/ per line 2.2x; matchAll with /gm 2.0x; test /\b(TODO|FIXME)\b/ 1.6x;
  new RegExp compile 4.3-4.9x (after 0224's fix of the follow-set regression).
These are the text-processing workloads (8% of codemode bodies; the "loop" workload is 1.7x
slower because of the replace). Find root causes (matcher inner loop, captures allocation, the
replace/matchAll builtins in bytecode, compile allocations) and fix; target parity with QuickJS.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

## Follow-ups

## Result
