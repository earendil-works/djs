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

- Batch 4: writing plain directly, step 1: built, validated, measured as a loss (records
  4,045 -> 4,166 M; the data lines' scan is ~9% of a data-heavy dump at most), dropped;
  notes in work/research/0215/direct-plain.md. pjz's class-with-async-method refusal:
  a later loop iteration's not-yet-declared bindings held the earlier iteration's
  values (no TDZ marker needed by the code) and were visited; now written as in their
  TDZ (corpus codemode/redump-stale-locals.js). pjz's await-using-continue-after-catch
  (batch 3b, "loop" scaffold record).

- pi end to end (wasm, next mode, a dump at every idle point; phase timings with
  work/research/0215/pi-phases.patch, scripts/dump-costs/pi-sessions.ts for profiles):
  per idle point a dump in a fresh instance's first `vm_dump` ~1.24 ms median, mostly
  the base walk (`BaseTable`), plus `provideBase` hashing the base per instance; the
  very first dump of a process ~5-10 ms (wasm tier-up, not djs). Fixed: D3020 (base
  names from the build; no base needed): vm_dump ~0.3 ms. Host: prelude image keys kept
  per start data, `$djs.source`'s wrapper and host globals read only when the dump names
  them.

- Array.fromAsync in flight (pjz, D3021): the dump runtime's `fromAsync` resumes the
  machine; corpus codemode/redump-from-async.js.
- pi loads (scripts/dump-costs/pi-sessions.ts, N=300, medians, same machine, A/B in one
  sitting): the dump runtime with its hooks and the host globals' values made with the
  prelude, in its image: runtime+hosts 118-140 -> 5 µs, load 580-700 -> 370-390 µs; a
  pool of up to two instances whose VM was disposed: new instance 103 -> 11 µs, load ->
  ~230 µs; made as new before reuse (memory restored and zeroed, D3023): ~17 µs per
  1.4 MB instance, new instance ~32 µs. Gate: D3022 (clean djs-core/djs-prelude after
  incremental corruption). Left: the `$djs.source` wrapper's compile (33-58 µs; per code, not per start,
  so not in the prelude image), setUp (~42 µs), the dump's compile (lazy, 0218).
- Dump setup: base globals compared by atom against the build's list (sorted by atom), a
  name made only for the VM's own globals; host expressions made only for the host values
  a dump writes (`HostExpr`). Same output.
- pjz's gaps (D3024): arrows reading a waiting frame's `this`/`arguments`/`new.target`
  were refused (`this`) or read K's own empty `arguments` after a load. The boxes are the
  frame's now; `arguments` is renamed to `$djs_arguments` in the arrows' texts by the
  syntax's offsets. K is called as before. Corpus codemode/arrows-share-this-arguments.js.
- pjz's two more: an accessor among an array's dense elements was written as a hole and
  never defined (the literal's index filter skipped every dense index; now only those it
  wrote). A `break`/`continue` through an awaiting finally in a later iteration (K's
  copied text) was refused at the re-dump: origin tracking took the frame's completion
  kind for the finally scaffold's constant (f-coded) though K was in a copy; `in_copy`
  now applies there too. Corpus codemode/redump-jumps-through-finally.js.

- Writer pass (parked for the subclass bugs, on the branch): keys from the scan's spans,
  `$`-searches instead of substring searchers, use counts / providers / scan deduplication
  by hash (exact: providers and scan's sets verify), one walk state per activation's cycle
  check. Records dump 4.81k -> 4.12k M (target <= 4.0k), byte-identical (corpus and every
  dump_perf dump). Per records dump left: emit_items ~1.8 ms (continuation generation, inline
  passes), schedule ~1.3 ms (keys_in's scan, 0.8 ns/byte over the items), drop_unread
  ~0.4 ms, count_uses ~0.3 ms. The timing instrumentation is work/scratch/0215/instr.patch.
- pjz's subclass bugs (D3025): placed code reading a late class binding moved after it
  (subclass-before-declaration.js); a re-dumped subclass method's `<home>` and super
  prototype from its function (super-in-async-method-redump.js). Corpus guard
  codemode/redump-super-late-class.js. Fuzz regress both modes: only s13-s10173's crossing
  callbacks, as on main.

- Writer pass done: records dump 4.14k -> ~3.98k M (target <= 4.0k), byte-identical on
  the corpus and every dump_perf dump: scan's `,`/`:` and keyword fast paths, strings and
  `$`/`.` searches eight bytes at a time (`find_byte`), pending/hook names, factory member
  counts and the global filter by hash first, count_uses without a list per text, `$`
  searches in `has_ident` and canon's helper count.
- Fuzz regress after D3025 found three more (D3026): pjz's s14-s11245 program still
  crashed (a late binding reading the block's class local; the per-factory guard), a guard
  gap in the monotone move (seed 832780), lone-surrogate keys (seed 82489, also on main) and
  a subclass placed above its base's block (seed 668041, --lazy, also on main). Corpus cases
  codemode/redump-subclass-class-local.js, redump-placed-function-name.js,
  redump-lone-surrogate-key.js, redump-subclass-in-loop-block.js.

## Follow-ups
- An object literal's async method using `super` after an await is refused ("a method that
  uses super outside its object literal"), e.g. `{ __proto__: base, async m(n) { await x;
  await y; return super.hi(n); } }` at its second idle point. Older refusal, not 0215's.

## Result
Paused for the coordinator (status left in-progress: it may bring pjr back for a writer or
maps hot spot). Measurements and profiles: work/research/0215/README.md; tools:
scripts/dump-costs/ (`instructions.sh corpus|cases [filter]`, `pi-sessions.ts`, the native
`dump_perf` with `DJS_DUMP_PERF*`, which also re-dumps the loaded VM and counts refusals).

What landed (same output unless a fix says otherwise):
- The check: D2888 (compile the plain text with data lines stripped; `DumpOptions::check`)
  and D2889 (free names from parse and resolve, no code generation).
- Writer scheduling indexed (providers, factories, keys; the quadratic `keys_in`/`dep_items`
  lookups) and `scan` keeping only the names wanted; builtins found by substring.
- Base and setup: D3020 (djs-prelude `names`: `BASE_NAMES`/`BASE_GLOBALS` from the build; a
  dump needs no base VM; `vm_dump` returns 0/1), base builtins as a sorted, borrowed table
  (`BaseTable`), base globals compared by atom (names only for the VM's own), host
  expressions made only when written.
- Continuation generation: name checks by first byte, one-pass helper count, static pieces.
- Loads: base64 decoding by digit range; `Vm::dump_runtime_for`; a VM keeps the maps of a
  dump it loads; pi: D3021 (Array.fromAsync as `$djs.fromAsync`), the runtime and host
  globals in the prelude image, D3023 (an instance pool, renewed as new, ~17 µs).
- Gate: D3022 (clean djs-core/djs-prelude after incremental corruption).
- Fixes found on the way: the frame-closure refusal of every data case (`const rnd = ;`),
  stale TDZ locals in later loop iterations, the for-await `"loop"` mark (pjz's
  await-using continue), pjz's nested for-in re-dump (`in_copy`), D3024 (arrows sharing a
  waiting frame's `this`/`arguments`/`new.target`), an accessor among array elements, a
  jump through an awaiting finally in K's copy.

Numbers (native, M instructions retired, `instructions.sh`; corpus has grown by ~10 cases
since 0198's table):

| | 0198 (format 2) | 0215 start | now (main a3f8f572) |
| --- | ---: | ---: | ---: |
| corpus dump (check on / off) | 12,262 | 15,551 | 15,424 / 10,383 |
| corpus load | 9,076 | 13,348 | 8,905 |
| corpus re-dump (check on / off) | - | 28,935 | 14,209 / 9,244 |
| records dump | 2,311 | 10,130 | ~4,000 at best (9,794 now: a writer regression, pjs) |
| log-5mb dump / load | 1,279 / 895 | 1,758 / 833 | 2,462 / 830 |
| binary dump / load | 128 / 159 | 205 / 390 | 342 / 156 |

pi in wasm (`pi-sessions.ts`, medians): a dump ~1.24 ms -> ~0.13-0.3 ms; a load ~760 ->
~230 µs (runtime and hosts 118-140 -> 5 µs, new instance 103 -> ~32 µs).

The check costs ~35% of a corpus dump but ~0% of pi's (its dumps write few function
texts); whether production turns it off is the owner's call.

Left, with owners: the writer's emit and repeated scans (records regression: `drop_unread`,
`flush_hooks`/`part_reads`, the inline passes; pjs, details sent); suspension maps and
repeated code at dump and load (lazy, 0218/0219); the dump's compile at load (lazy); setUp
per load ~40-45 µs (hash rekey ~55%; a seed-per-image cache costs more than it saves);
continuation generation's remaining cost is the piece tree's allocation (pjs's item 11
touches it).
