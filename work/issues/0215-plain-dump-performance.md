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

## Follow-ups

## Result
