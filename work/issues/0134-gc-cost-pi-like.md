# 0134: GC cost in pi-like: faster old-region walk, young-sized threshold

status: open
agent: -
branch: issue/0134-gc-cost
depends-on: 0132
guide: docs/guide/gc.md
touches: crates/djs-core/src/heap/ (gc, walk, old region, scheduling)

## Goal
heap's profile (work/scratch/0132/gc-profile.md): GC is 9.1% of pi-like's wasm
instructions (15% of wasm time); 164 minor + 10 full collections; the old-region walk is
the largest piece (36–38%). Two items, in this order:
1. Faster old-region walk: per-kind slot loops with the young-target check inlined
   instead of the generic walker's per-slot call (~−1.7 to −2.3% pi-like, also props).
2. Collect less often without growing small heaps: size the next threshold from the young
   part (e.g. old + k * young-live) instead of 2x all live (~−4% pi-like, ~+0.5 MiB peak
   arena on pi-like at most); measured with mem-perturb (24 runs) on all workloads.

## Acceptance criteria
- [ ] Both measured on all workloads (wasm instructions; peak memory via mem-perturb);
      no workload slower than +1%; peak memory trade recorded if any.
- [ ] Determinism kept (thresholds are image state); size ratchet respected.
- [ ] gc.md updated.

## Log

## Follow-ups

## Result
