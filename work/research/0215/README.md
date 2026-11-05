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

## After batches 1-2 (main 1ddec501) and the builtins substring (M instructions)

| | 0198 dump | dump | dump, check off | 0198 load | load | re-dump | re-dump, check off |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| corpus | 12,262 | 16,282 | 8,603 | 9,076 | 9,597 | 22,292 | 14,804 |
| records | 2,311 | 4,131 | ~3,760 | 4,226 | 4,368 | 5,922 | ~4,700 |
| log-5mb | 1,279 | 1,653 | | 895 | 821 | 1,486 | |
| binary | 128 | 218 | | 159 | 164 | 295 | |

Where the time goes now:
- Corpus dump: the check ~47% (its compile ~75% of that: parse ~45%, resolve and
  emit ~55%), plain::write ~24% (pjs's canonical K generation ~9%), visit ~7%.
- Corpus re-dump: the check ~47%, plain::write ~19%, visit ~8%, `BaseTable::new` ~4%
  (a fresh VM builds the builtin table again).
- Corpus load: the dump's compile ~85% (recording its maps for the next dump ~11% of
  the load); the rest runs it.
- records dump: plain::write ~50% (scan of each item ~30%, schedule ~30%), the
  Dumper's own rendering of the data (format-2 IR lines) ~20%, visit ~11%.

Later items (not done):
1. The check's compile without code generation (parse and resolve only, the free
   names from the scopes): roughly halves the check (~-20% of a dump, -15% of a
   re-dump). Compiler work.
2. Writing plain directly: the Dumper renders format-2-shaped lines that plain::write
   reads back (classifies, scans for references, schedules). Rendering items with their
   references already known would drop most of the scan (~30% of a data-heavy dump) and
   the read-back. A rewrite of the Dumper/writer boundary.
3. Loads compile every function body of the dump, also those that never run (the
   factories' closures, the origins written for re-dumps). Lazy function compilation
   would cut the ~85% compile share of a load. Compiler work, large.
4. The base builtin table per fresh VM (`BaseTable`, ~4% of a re-dump in pi's fresh-VM
   model): carry it in the dump cache a host passes on (`Vm::set_dump_cache`).

## pi's dump side in wasm (main 94a24f49, scripts/dump-costs/pi-sessions.ts, N=800, node --cpu-prof)

A dump in pi's adapter at steady state costs ~150 µs at the median (dumps ~330 B). Inside
`vm_dump`, as shares:
- visit ~44%, ~30% of it the suspension maps the dump compiles for the activations'
  scripts (`maps_entry`/`compile_maps`; on demand since 0218 batch 3; repeated code is
  0219's compile cache);
- plain::write ~34%: generating the continuations ~22% (the canonical K ~13% of it:
  `canon::flatten`, `layout_of`), emitting items ~14%;
- `Dumper::new` ~7% (after the base names were borrowed instead of copied: it was ~25%);
- the check ~0%: pi's dumps write few function texts (the body is `$djs.source`), so on
  or off makes no difference there;
- host staging and string copies (dumpSession minus vm_dump) ~9%.

setUp per load (~40-45 µs): a deterministic run rekeys the hash at every restore
(`setRandomSeed` -> `set_hash_key` walks the heap, ~55% of setUp; the prelude image is
shared across seeds, so the key cannot be in it), the `setOrigin` call (~20%), the
adapter's state. A non-deterministic session (pi in production) reseeds Math.random only:
~20 µs.
