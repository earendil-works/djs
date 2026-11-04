# 0198: Make dumping and loading fast

status: in-progress
agent: dumpperf
branch: issue/0198-dump-perf
depends-on: 0196
guide: docs/guide/snapshots.md (dump costs) if numbers change
touches: crates/djs-core/src/vm/dump*.rs, crates/djs-core/src/compiler/ (recording, maps), crates/djs-wasm (dump paths), js/src/sandbox.ts (dump module use), scripts/dump-costs, docs/continuation-dumps.md

## Owner
"After the architecture change, get an agent to optimize the dump performance. I'm sure
there is a lot of opportunity left to make that really fast."

## Where we are (main after 0196, D2735)
- Format 2 dumps are named-slot state machines (D2720, D2731–D2735); a load compiles the
  dump text once with recording on (load_dump ~450 ms native over the 1,164 harvested
  dumps, 23 MB; was ~800 ms).
- Last wasm measurements (0184, before SM): corpus p50/p90 dump 1.84/3.65 ms (packaging B),
  resume ~0.9 ms; data-heavy cases tens of ms (records 21/38 ms dump/resume, 5 MB log
  68/81 ms). scripts/dump-costs/dump-costs.ts measures it.
- Known opportunities (sm, critic):
  - the exporter recompiles each script per dump (script_maps/body_site): a per-VM map
    cache;
  - big data dumps compile at ~100 ns/byte (12 MB → 1.2 s native): data sections could
    skip the compiler (a fast literal/JSON path for data-only statements, or a separate
    data encoding the loader parses directly);
  - drop executed body text (critic's report §6; needs a decision, changes toString of a
    body only the run holds);
  - skip re-dumping an unchanged run (dump-after-load is byte-identical);
  - the dumper's graph walk, string escaping, number formatting, allocation;
  - packaging B's image transport between modules.

## Goal
Profile first (native and wasm, the corpus distribution and the data-heavy cases), rank
the costs, then land the wins in small merges with numbers before/after. Keep the format a
plain Script unless a decision says otherwise; keep every gate check green (goldens,
dump-every-idle, idle-point counts at 100%).

## Log
- Harnesses. Native: `vm/dump_perf.rs` (ignored test, `DJS_DUMP_PERF=all`, release): the
  harvested corpus as dump_harvest's idle run, plus the data-heavy cases of
  scripts/dump-costs/native (fake tools answering through `later`, data made in JS);
  best of 3 dumps and loads per idle point. Wasm: scripts/dump-costs/dump-costs.ts, image
  columns dropped (images are gone), corpus for both packagings, best of `--reps`.
- Baseline, native release (quiet machine): corpus 1,164 idle points (23 MB: the native
  harness dumps the corpus prelude too) dump total 3,086 ms, p50/p90/p99 2.23/3.18/13.6
  ms; load total 441 ms, p50 0.31 ms. records dump/load 12.0/13.2 ms, log-5mb 52.9/32.9,
  binary 15.6/28.8, empty 2.5/0.36.
- Baseline, wasm (node 25; corpus 1,063 idle points as Sandbox bodies): dump p50/p90/p99
  0.92/2.58/35.5 ms (A), 0.97/2.66/35.6 (B), totals 2,130/2,181 ms; resume p50 0.44 (A) /
  0.78 (B). records 17.5/22.0 ms dump/resume, log-5mb 60/39, binary 27/60, big-log 26/12.
- Profile (macOS `sample`, native corpus): 75% of dump time was in `source_slice`, which
  converted the whole script source to a String for every function text the dumper read
  (closure visit, render, the checker): quadratic in a script's functions. Next: the
  checker's compile (~10%), the suspension maps' compile (~4%), emit.
- Win 1: a per-dump cache of script source texts (`Sources`), function texts sliced from
  it. Output byte-identical over all corpus and case dumps. Native corpus dump total
  3,086 -> ~965 ms (p50 2.23 -> 0.64 ms, p99 13.6 -> 4.2). Wasm corpus dump p50/p99
  0.92/35.5 -> 0.61/8.5 ms (A), total 2,130 -> 988 ms; B 2,181 -> 1,087 ms.
- Win 2: a per-VM cache of suspension maps (`dump::Cache` in `Vm::dump_cache`, 8 scripts,
  keyed by source text, parameters and option bits; also the codemode wrapper's maps in
  `body_site`). The maps are plain data and depend on nothing else, so they survive
  collections, runs and `replace_image` (packaging B keeps the dump module's VM, D2701).
  dump_perf restores the cache before each repetition, so each one is timed as the first
  dump at its idle point. A cache of the checker's whole skeleton was tried and dropped:
  ~0 hits (slot values and `$fN`/`$dump.nN` numbering change between idle points).
- Win 3 (compiler, outside `touches:` but one struct): the unit constant pool rehashed
  every shared constant at each table doubling, so a 3.7 MB string constant was hashed
  ~7 times per compile (63% of loading the 5 MB-log dump). The table keeps the hashes.
  Bytecode unchanged.
- After wins 2+3, native: corpus dump total ~965 -> 782 ms (p50 0.53 ms), load 420 ms;
  log-5mb dump/load 52.9/32.9 -> 31.6/15.0 ms. Wasm: corpus dump p50/p99 0.61/8.5 ->
  0.43/6.4 ms (A), totals 988 -> 708 ms (A), 1,087 -> 747 (B); log-5mb 62/39 -> 42/19
  ms (A); records 17.5/22 -> 16.7/20; resume corpus unchanged (0.43 A / 0.79 B).
- Ranking now (native profile, corpus dump): the checker's compile of the skeleton ~50%
  (the skeleton is 79% of the dump text: function texts), emit ~20%, visit ~15%.
  Packaging B's resume pays ~0.36 ms over A at the median (image transport).
- Win 4: the dumper's own ASCII conversion of heap strings and atom names (`text_of`,
  `atom_text`; core's `str_to_string` decodes unit by unit, and a fast path there costs
  the slim module 179 B gzip, over its ceiling). Native corpus dump 782 -> 648 ms.
- Win 5: strings quoted by runs of plain bytes (`quote_latin1`). log-5mb dump 32 -> 22 ms,
  records 9.1 -> 7.0 ms native.
- Win 6 (compiler): string constants were deduplicated through two sorted `Vec`s with
  inserts (str_pool per unit, str_consts per function; numbers likewise): quadratic in
  distinct constants, 25% of loading the records dump. Now one hash index for the unit's
  heap strings (a long string hashed by its length and ~256 sampled units) and no
  per-function dedup: resolution interns every PushConst by value (D2446), so the
  bytecode is the same (snapshots unchanged). Slim module -140 B gzip. compile/typescript.js
  52.8 -> 59.9 MB/s, prelude compile 6.4 -> 6.0 ms. records load 12.6 -> ~10.2 ms.
- Coordinator (2026-11-05): plainjs (0199) prototypes a plain-JS format; prefer wins that
  survive a format change, defer `$dump.resume`/recording-specific work.
- After wins 4-6, wasm (machine load ~20, noisy): corpus dump p50 0.36 ms (A) / 0.39 (B),
  totals 509-522 / 542-581 ms; records dump 12.1 ms, log-5mb 24 ms, big-log 13 ms.
- Win 7 (binary buffers): `$dump.bytes` decoded through UTF-16 units and the spec's
  chunk loop (half the load); now a table decoder for the dumper's padded base64 (else
  `atob`); the encoder writes bytes by 3-byte chunks; the dumper skipped a typed array's
  element keys only after `own_keys` made a million index atoms. binary dump/load
  15.6/28.8 -> 8.6/10.5 ms native.
- Win 8 (compiler): the unit pool's hash of a long (>1 KB) flat string is its length and
  ~256 sampled units, not SipHash of every unit (equal strings still hash equal; a match
  is compared whole). log-5mb load 15 -> 10.8 ms, binary 10.5 -> 6.0 ms. Slim module now
  376,652 B gzip of 376,679 (win 6 paid for it).
- Win 9: the checker reads each template's globals once, children first (it walked the
  subtree of every template), and compares opcodes, not names (~10% of the check).
- Win 10 (compiler, dump builds only, D2801): pattern scans stop at a literal element or
  property value; after a syntax error the source is parsed again with exact scans so
  errors are unchanged. records load 10.0 -> 8.8 ms native. Gated on `dump` because it
  costs the slim module ~470 B gzip otherwise (and only the dump module loads dumps).
- D2800 records win 6/8's constant changes, D2801 win 10.
- Win 11 (packaging B, D2802): one dump-module sandbox per dump module and memory limit
  in the process, not per sandbox, so `Sandbox.resume` no longer makes a second instance
  (the dump module's VM runs no host function, so sharing is safe). Corpus B resume
  minus A resume, totals: ~370 -> ~140 ms; p50 gap 0.36 -> ~0.16 ms (machine load ~10).
  Docs: host.md, guide/host.md.
- The machine got too busy for timings (same build 50% slower between runs), so
  scripts/dump-costs/instructions.sh counts instructions retired (`/usr/bin/time -l`) of
  dumping and of loading at every idle point (dump_perf with `DJS_DUMP_PERF_SKIP`);
  repeat runs agree within 0.2%.
- Win 12: the dumper's builtin table (paths, `Named`, the cell index, the base globals)
  is made once per VM (`BaseTable` in `Vm::base_names`); each dump copies the index.
- Win 13 (emit allocation): the dumper's own `cat!` sizes its string up front (pieces
  evaluated once, `Piece::size_hint` with the `dump` feature only), `join` too; arrays and
  object literals are written in place; object literal keys and member paths are kept
  per dump by atom; a key's name is made only for function values. Instructions:
  cases dump 4,933 -> 3,683 M (-25%), corpus dump 12,681 -> 12,254 M (the corpus dump
  is mostly the checker's compile).

## Result
