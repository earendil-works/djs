# 0131: Size follow-ups: message builder, decomposition stream, composition tables

status: done
agent: bloat
branch: issue/0131-size-followups
depends-on: 0126
guide: - (unless a page describes the tables)
touches: crates/djs-core/src/unicode/ (normalization tables + generator), message construction sites (interp/run.rs, date.rs, errors: small edits, announce to perf who works in interp/ in 0129), scripts/size

## Goal
bloat's 0126 follow-ups, speed-free cuts toward the D020 budget (main 460.9 KB, 1.608x):
1. A small non-formatting string builder for the ~100 `format!` message sites (~2–3 KB gz).
2. QuickJS-style recoding of the normalization decomposition stream (~2–3 KB).
3. Derived composition tables with a runtime cache keeping localeCompare/normalize fast
   (~3 KB).

## Acceptance criteria
- [x] Each cut measured; no workload slower than +1%; size ratchet lowered.
- [x] Error messages byte-identical (difftest/test262 outputs unchanged).

## Log
- Baseline main 86f349ff: 1,065,090 raw, 460,874 gzip.
- Parts 2 and 3 (D2337, D2338): decomposition stream split into entries + targets with
  per-position prediction (11.6 -> ~8 KB gzip in isolation); composition tables derived
  from the decompositions into a per-heap cache (`Heap::compositions`, a small field in
  heap/mod.rs, outside touches: heap/ is heap's area). Module 460,874 -> 455,263 gzip
  (−5.6 KB), raw −3.96 KB. NormalizationTest.txt and the every-code-point quick-check test
  pass. Perf on a normalization-heavy scratch workload (work/scratch/0131/norm*.js:
  localeCompare sort + normalize in all forms): −20% instructions overall, NFD −15%,
  NFC −12% (the in-place decode and varint fast path); a first version without them was
  +14%, and INDEX_STEP 16 cost size; both dropped. Merged first (3dd7ca94).
- Part 1 (D2339): crate::text (`cat!`/`cat_to!`, pad/hex) replaces format!/write! on
  every reached site (a scratch converter rewrote ~100 sites mechanically; interp/ edits
  are one-liners, perf told beforehand). `alloc::fmt::format` is gone from the module;
  core::fmt itself stays, held by core's panics (unwrap on Debug errors, assert_eq!,
  bounds checks): ~6.5 KB raw. Only −0.7 KB gzip, much less than the 2–3 KB estimated
  in 0126: the formatting machinery was mostly reached from panic arguments, not from
  messages. Messages: lib tests, e2e, test262 Date/Error/Function.toString (751/751)
  unchanged; the text unit test checks pad/hex/integers against format!. Perf (wmeasure
  n=3/5): workloads within ±0.3%; a Date-formatting scratch workload −9%.

## Follow-ups

## Result
Three cuts, D2337–D2339: module 460,874 -> 454,583 bytes gzip (−6.3 KB), raw 1,065,090 ->
1,058,918. Decomposition stream with per-position prediction and derived, per-heap
cached compositions −5.6 KB (normalization also 12–20% faster); messages without
`format!` −0.7 KB (Date formatting 9% faster). Error messages byte-identical.
Known gap: core::fmt remains reachable through core's panics on stable.
