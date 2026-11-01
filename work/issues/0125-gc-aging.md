# 0125: GC aging promotion and peak-memory stability

status: in-progress
agent: heap
branch: issue/0125-gc-aging
depends-on: 0120
guide: docs/guide/gc.md
touches: crates/djs-core/src/heap/ (gc, compact, old region, scheduling), crates/djs-core/src/vm/ (if scheduling hooks change), docs/old-generation.md, ARCH.md (Heap/GC)

## Goal
0120's follow-ups (heap's report, D2041–D2044):
1. Promote cells after they survive two minor collections (age tracked without a per-cell
   header bit if possible, e.g. by region: survivors of one minor sit in a known range
   just above B), so workloads with large long-lived young data (arrays, higher-order,
   json) stop re-tracing it, without the +76% peak memory that promotion-at-full showed
   (D2042).
2. Make peak memory less sensitive to small arena shifts (strbuild 25.5 → 31.9 MiB in
   0120), e.g. reserving capacity right after a collection.

## Acceptance criteria
- [ ] Determinism kept (age/region boundaries are image state), every gate check green.
- [ ] GC time and wasm instructions measured on the 0081/0087 workloads and the pi bench;
      peak memory no worse than main on any workload by more than noise (strbuild back to
      ~25.5 MiB).
- [ ] Decisions recorded; gc.md updated (D2102 rules).

## Log

## Follow-ups

## Result
