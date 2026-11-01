# 0132: Initial arena room for VMs from the base image (json peak regression)

status: done
agent: heap
branch: issue/0132-initial-arena-room
depends-on: 0128
guide: docs/guide/gc.md (peak memory section) if behavior changes
touches: crates/djs-core/src/heap/ (initial capacity), crates/djs-core/src/vm/ (vm_new paths), crates/djs-wasm/src/allocator.rs

## Goal
0128 follow-up: json 6.2 → 6.8 MiB and json-replacer 4.43 → 4.83 MiB, because one of
dlmalloc's first segments lands right after the young arena, so the arena's first growth
can't extend in place. Give a VM made from the base image room up to its first
collection threshold at creation (or place the first dlmalloc segment before the arena),
measured with scripts/perf/mem-perturb.sh on all workloads and pi warm/cold.

## Acceptance criteria
- [ ] (not met; regression accepted, D2054) json and json-replacer back to main-before-0128 or better; no workload worse
      beyond noise; pi warm/cold memory and vm_new time not worse.
- [x] Decision recorded.

## Log
- Page-manager log of all workloads (a scratch build: work/scratch/0132/plog.py,
  plog-run.ts). Every growing workload: dlmalloc 1 page, decoded base 5, arena 11, then
  dlmalloc 2 pages right after the arena, then one arena move to the end. json ends with
  its arena at page 27; pre-0128 dlmalloc moved it down to ~14 at its last move.
  dlmalloc needs 3–9 pages per run.
- Initial room: the arena already gets its first threshold plus slack; json overshoots it
  in one native operation, and more room up front costs every small VM.
- dlmalloc-only pool at instance start, 2/4/6 pages: 2 is identical; 4 and 6 make json
  worse (7.4/7.6) and move other workloads 5–10 MiB either way, small VMs +0.1–0.2.
- Reported to the coordinator; decision (a): accept. D2054.

## Follow-ups
- Moving the arena down at collections (option b): when `after_collect`'s
  `shrink_capacity` (or a realloc to the same size) runs, the page allocator would move a
  huge block into the lowest free run that fits it with its threshold room. The copy goes
  into free pages, so the peak doesn't rise, and the arena can settle low as it did under
  dlmalloc.
  - Size: ~40 lines in allocator.rs (a lower-run search on shrink and same-size
    realloc) plus a heap hook to realloc to the capacity target after a collection.
  - Cost: one arena copy per collection that finds a lower run (bounded by live size;
    rare once the arena is low).
  - Measure: the 24-run mem-perturb sweep over all workloads, pi warm/cold and
    wmeasure. Accept only if no workload's mean or max rises beyond noise.

## Result
- Regression accepted: json +0.6 / json-replacer +0.4 MiB against 2× wins elsewhere
  (D2054). No code change. The research task on pi-like's GC instructions is below in
  the log.
