# 0184: Close the remaining dump refusals; measure dump size and load time

status: in-progress
agent: portable
branch: issue/0184-dump-coverage
depends-on: 0182
guide: -
touches: crates/djs-core/src/compiler/ (suspension maps for the unsupported constructs), crates/djs-core/src/vm/dump*.rs, tests/dump/v2/ (new golden programs under D2619), tests/smap pins, scripts/snapshot-research or scripts/dump-costs (measurements), docs/continuation-dumps.md

## Goal
Dumps are the persistence format (D2560); before images are retired (0177):
1. **Remaining refusals** — describe what a suspended frame holds at, and dump/resume:
   `switch` case tests (the discriminant twice on the stack), `finally` blocks (completion
   records), `using`/`await using` regions, destructuring, `yield*` (the delegated iterator),
   closing an async iterator, `for-in` (the key iterator), RegExp string iterators, iterator
   helpers if feasible. Each: the map roles, the loader's resume side, re-enactment, golden
   programs, every-idle-point tests. Refuse cleanly whatever stays out.
2. **Costs vs images** (the owner's comparison): dump size (raw and zstd) and write/load time
   (wasm, the dump module) next to session images/containers for: an empty Sandbox, typical
   pi waiting sessions (scripts/snapshot-research cases, the harvested corpus idle points:
   distribution), and data-heavy sessions (records, a 5 MB log, binary buffers). Recommend
   whether anything (e.g. zstd of dump text, slices/shared strings in dumps) is needed before
   0177 retires images.

## Log
- 2026-11-04 (portable): costs measured with scripts/dump-costs/dump-costs.ts (wasm
  profile builds; report work/scratch/dump-costs/report.md; interim note sent to the
  coordinator with the full tables). Corpus (300 cases, 1,063 idle points, packaging B)
  p50/p90/p99/max: dump raw 6.1/13.6/165/165 KB, zstd-3 1.7/3.1/17/17 KB, container
  5.1/9.7/34.5/34.6 KB, image 23/41/218/221 KB; dump 3.5/6.0/43/69 ms, resume
  1.3/2.3/13/29 ms, bytes() 0.7/1.0/5.8/15 ms, fromBytes 0.3/0.5/2.3/14 ms. Data-heavy:
  records dump 867 KB -> 236 zstd vs container 372; big-log 1,816 -> 394 vs 416; log-5mb
  3,749 -> 924 vs 930; binary 2,493 -> 1,570 vs 1,446 (typed arrays as decimal
  elements). Fixed floor ~2-3 ms per dump in wasm (0.8 ms native), 90% of it
  registry::reachable_names per dump. Recommendations: hosts zstd-3 dump text; typed
  arrays as base64 bytes; cache the builtin names per base; no shared strings needed.
  Only refusal in the cases: yield* (agent-loop, 3 of 11 points).

## Result
