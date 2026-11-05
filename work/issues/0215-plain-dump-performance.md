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

## Follow-ups

## Result
