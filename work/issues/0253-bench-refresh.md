# 0253: Re-record the benchmarks page after 0251/0252

status: done
agent: owner session
branch: issue/0253-bench-refresh
depends-on: 0252
touches: tests/scores/status.tsv, docs/guide/benchmarks.md, docs/guide/gen/
guide: benchmarks.md

## Goal
The benchmarks page was measured at 1268c57eeb (2026-11-06), before the generated
builtins (0251) and the size work (0252); its "Operation by operation" prose described
the small builtins as running in bytecode. Re-record and update the prose.

## Log
- `scripts/status.py engines` at 68954b21 (main). A first run had the workloads at load
  14.6 (other work on the machine) and was discarded; the second: workloads load 2.3,
  ops 5.9, bridge 5.2. Per-op ratios of the two runs agree within about ±5-10% (except
  one replaceAll outlier in the first).
- Prose: "Operation by operation" rewritten (toLowerCase faster; includes, replace, map,
  push at parity; the rest of the gap is the call, number formatting and short strings);
  the summary's "string builtins" now "string and number builtins". Other sections still
  read true; their numbers are generated.

## Result
benchmarks.md and its charts regenerated from the new status.tsv values.
