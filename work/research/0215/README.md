# 0215: plain dump performance, first measurements

Native, release, `scripts/dump-costs/instructions.sh` (instructions retired, one dump, load
and re-dump per idle point; machine load 20-60, so no timings recorded). Baseline: 0198's
final commit 429184db (format 2), built in work/scratch/0215/base. Re-dump: the loaded
VM's first dump (new in dump_perf.rs).

| M instructions | 0198 dump | now dump | 0198 load | now load | now re-dump |
| --- | ---: | ---: | ---: | ---: | ---: |
| corpus (1,164 idle points) | 12,262 | 15,551 | 9,076 | 13,348 | 28,935 |
| records | 2,311 | 10,130 | 4,226 | 4,351 | 14,749 |
| log-5mb | 1,279 | 1,758 | 895 | 833 | 2,311 |
| binary | 128 | 205 | 159 | 390 -> 171 | 415 |
| empty | 19 | 22 | 7 | 11 | 26 |

Found on the way: every data case's dump was refused (`const rnd = ;`: a suspended
frame's closure that data reaches); fixed. dump_perf counted refusals as nothing.

## Where the time goes (macOS `sample`, inclusive)

- Corpus dump: the checker's compile of the skeleton (`Dumper::check`) ~40%,
  `plain::write` ~20% (`scan` ~10%), visit ~25%.
- Corpus re-dump: suspension maps of the dump's own script (`script_maps` compiles all
  of dump.js, data included, in a fresh VM) 30%, checker 33%, `plain::write` 15%.
- Corpus load: `Vm::dump_runtime` (compiling all ~45 helpers) ~40%, the dump's compile
  ~50%.
- records dump: `plain::write` 79%: `W::schedule` 50% (`keys_in`, `dep_items`: linear
  provider and factory lookups, `Vec::contains` of keys: quadratic in data items),
  `scan` 42% (each item's text scanned again per use).
- binary load: `Uint8Array.fromBase64` searched the alphabet per digit (fixed: 390 -> 171 M).
