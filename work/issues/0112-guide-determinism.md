# 0112: Guide chapter: determinism

status: done
agent: infra
branch: issue/0112-guide-determinism
depends-on: 0109
guide: docs/guide/determinism.md (new); links from arena.md, heap.md, gc.md, suspension.md, host.md
touches: docs/guide/ only

## Goal
The determinism tour chapter, under docs/guide/README.md, linking the related material in
the docs agent's chapters (suspension.md: fuel, debt, safepoints D1740/D1712; gc.md:
D1601/D1710/D1090; heap.md: D1831, Map rehash, weak refs D1680; arena.md: D1690) rather than
repeating it. The chapter itself covers: the sources of nondeterminism and how each became
host input (clock, time zone, Math.random, weak refs: D1383/D1314/D1680), results as
macrotasks (link host.md), the canonical hash and journals (D1730), what the gate compares
(images across schedules, raw images, native vs wasm: D1720/D1725), and what is *not*
deterministic (host OOM, GC timing via WeakRef in the default mode, the backend choice in
fuel counts D1633).

## Acceptance criteria
- [x] determinism.md reviewed by the docs agent before merge; `djs-docs check` green.
- [x] Once merged, turn the plain-text "the chapter on determinism" references in arena,
      heap, gc, suspension (after the docs agent merges them) and host.md into links.

## Log
- Wrote docs/guide/determinism.md:
  - the inputs table (clock, time zone, Math.random, weak refs, results) with a gen:run
    example (clock 0, the default seed);
  - results as macrotasks (linking host.md);
  - "the same program, the same heap", which links suspension, gc, heap and arena for
    fuel debt, GC points, layout, rehash/shapes and OOM instead of repeating them;
  - deterministic mode, canonical hash and journals, what the gate compares, and where
    determinism ends;
  - three trade-offs.
- Linked it from SUMMARY.md, host.md (the plain-text reference), arena.md, heap.md,
  gc.md and suspension.md (their determinism mentions). `djs-docs check` is green.
- The docs agent approved (via the coordinator); its one fix, a wrap in heap.md, is applied.

## Follow-ups

## Result
docs/guide/determinism.md is in the tour. SUMMARY.md, host.md, arena.md, heap.md, gc.md
and suspension.md link to it.
