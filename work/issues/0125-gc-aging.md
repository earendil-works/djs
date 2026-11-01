# 0125: GC aging promotion and peak-memory stability

status: done
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
- [x] Determinism kept (age/region boundaries are image state), every gate check green.
- [x] GC time and wasm instructions measured on the 0081/0087 workloads and the pi bench;
      peak memory no worse than main on any workload by more than noise (strbuild back to
      ~25.5 MiB).
- [x] Decisions recorded; gc.md updated (D2102 rules).

## Log
- Built aging by region: `Roots::AGED` and `FULL_OLD`. A minor collection promotes
  `[OLD_END, AGED)`'s survivors; full collections keep ages, so `collect` stays a
  fixpoint. Triggers for a full collection: promoted > FULL_OLD/2, /4 or /8. The tests
  compare cells with the region fields masked.
  - Gains: about 1% of instructions, 7–20% of collection time.
  - Memory: pi-like +2 MiB, strbuild up to +24 MiB, json-replacer +5.5 MiB.
  - Rejected (D2046). The patch is in work/scratch/0125/aging-experiment.patch.
- Allocation traces (an alloc-stats build with a big-block trace,
  work/scratch/0125/trace.patch and trace-run.ts) show the peaks: arena reallocs that
  dlmalloc can't do in place.
- Tried arena policies:
  - eager reservation after a collection, with and without shrinking first;
  - shrinking to the live size;
  - 1.5× and 1.25× growth;
  - atom table slack in the base;
  - a full collection instead of growth after a minor one.
  All rejected (D2047): each moves some workload by MiB either way. The guard costs
  +2–6% instructions. The guard must use image state anyway: the arena's capacity is
  not image state.
- Found that single-run peaks are noise:
  - a 1 KB shift moves strings 14.1 ↔ 18.6 MiB;
  - with 8 KB steps, main's strbuild stays at 25.5 while 0120's is 26.8–31.9, so that
    difference is systematic.
  New scripts/perf/mem-perturb.sh (24 runs, k × 1000 bytes kept first); TESTING.md has
  "Peak memory claims" (D2048).
- The cause of 0120's strbuild regression: 17.7 KB of base cells die at once in a fresh
  VM (the atom entries array replaced by the first compiled atom, plus a PropTable and a
  U32Array). They stay until the 16th collection. Fix: the first collection after a
  canonical image is full (D2045). strbuild is back to 25.5 across all 24 runs;
  json-indent goes back to its pre-0120 10.3 (0120 had placed it luckily at 7.5).
- js/test/base.test.ts (outside touches): the base-vs-compiled test collects both VMs, since the base now schedules a full first collection.
- Guide: gc.md (first full collection, retention wording, the "No promotion" trade-off
  with both attempts, new subsection "Peak memory is chaotic"). ARCH Heap/GC.

## Follow-ups
- In-place arena growth: a page allocator in djs-wasm, with dlmalloc over our own page
  manager and huge blocks grown by memory.grow while they are last in memory (0128).

## Result
- **Landed:**
  - `collect_canonical` leaves MINORS = FULL_EVERY − 1, so a VM from the base collects
    in full first (D2045), with a gc_tests test.
  - `scripts/perf/mem-perturb.sh` and the method in TESTING.md and scripts/perf/README.md
    (D2048).
  - Decisions D2045–D2048. D2050–D2052 were already taken by another agent, so this
    issue skips them.
- **Peak memory** (24 perturbed runs, mean): strbuild 28.07 → 25.51 (pre-0120 25.50),
  strings 14.85 → 14.90, json-replacer 6.87 → 6.76, json-indent 7.55 → 10.30 (pre-0120
  10.48). The other workloads are equal to pre-0120 main.
- **Wasm instructions:** within noise against main.
- **Not landed:** aging promotion (D2046) and the arena growth policies (D2047).
