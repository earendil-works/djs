# 0215: Plain dump performance: export and load times against format 2's

status: in-progress
agent: pjr
branch: issue/0215-plain-dump-performance
depends-on: 0205
guide: -
touches: scripts/dump-costs/ (measurement), then the hot spots found (coordinate: pjs owns dump_cont.rs/dump_origin.rs/dump_plain.rs's writer for 0207 item 4)

## Goal
0198 made format 2 fast (D2800-D2802: wasm corpus dump p50 1.42 -> 0.43 ms, totals 3,243 -> 610
ms; data cases 2-6x faster; tools in scripts/dump-costs/). The plain exporter replaced it, with
continuation generation, origin tracking, re-dump regeneration and a Script compile at load.
Measure where plain dumps stand now and fix what regressed:

1. Bring scripts/dump-costs/ up to plain dumps (it may still measure format 2 paths): per case
   and totals, native and wasm, for export (Vm::dump), load (running the dump Script with the
   hooks), and a re-dump of a loaded run; the data-heavy cases (big arrays/objects/strings) and
   the pi suite's dumps. Compare with 0198's recorded numbers.
2. Profile the slowest (scripts/dump-costs/instructions.sh) and write the findings to
   work/research/0215/README.md; send the coordinator a short summary with proposed fixes.
3. Fix the clear hot spots in batched merges, outside the files pjs is changing unless the
   coordinator sequences it. Record bench numbers only on a quiet machine (load < ~4).

## Log
- Measured (pjr): dump_perf now re-dumps the loaded VM and counts failures; first numbers
  and profiles in work/research/0215/README.md. Every data case had been refused
  (`const rnd = ;`, a frame closure data reaches): fixed in dump.rs.
- Batch 1: frame-closure fix; base64 decode by range (+ whole groups, dump builds only:
  +250 B slim otherwise); loads compile only the runtime helpers the dump names
  (`Vm::dump_runtime_for`, `scriptFor`); a VM keeps the maps of a dump it compiles for its
  next dump; pjz's nested for-in re-dump (a copied inner loop read the scaffold's stale
  iterator; dump_origin `origin_temp_k`, not `record()`). Native instructions, corpus:
  load 13,348 -> 9,621 M, re-dump 29,117 -> 21,120 M; binary load 390 -> 165 M.

- Batch 2: plain::write's scheduling by index (scan dedups by set and keeps only
  wanted names; providers, factories, activation items and line records by map; output
  byte-identical): records dump 10,130 -> 4,394 M, re-dump 14,749 -> 6,106 M. The check
  compiles the plain text (data lines left out) instead of the format-2 skeleton, same
  cost (D2888); `DumpOptions::check` (default on): corpus dump 16.2k -> 8.6k M without
  it, re-dump 22.3k -> 14.8k M.

- Batch 3: the builtins a dump reads by substring; the check's names from parse and
  resolution alone (D2889; validated equal to the compiled code's on the corpus): corpus
  dump 16,282 -> 13,123 M, re-dump 22,292 -> 19,229 M; records dump 4,131 -> 4,045 M.
  BaseTable per fresh VM (~4% of a re-dump): skipped, wasm hosts cannot carry the cache
  across VMs. Load-side maps recording: kept (11% of a load saves 27% of the re-dump).

## Follow-ups

## Result
