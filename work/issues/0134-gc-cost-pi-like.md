# 0134: GC cost in pi-like: faster old-region walk, young-sized threshold

status: done
agent: heap
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
- [x] Both measured on all workloads (wasm instructions; peak memory via mem-perturb);
      no workload slower than +1%; peak memory trade recorded if any.
- [x] Determinism kept (thresholds are image state); size ratchet respected.
- [x] gc.md updated.

## Log
- Item 1: `compactor::PLANS`, per-kind slot plans built by a `const fn` from LAYOUTS.
  `scan_old` loads and compares each slot inline and calls a cold `old_hit` only for
  young targets; value tails get a tight word loop. A test checks the plans against
  `for_each_slot` for every kind and size. The first table (256 wide entries) cost +16 KB
  raw / +621 B gzip, over the size ceiling; packed to 28 × 16 B: +459 B gzip.
  - Wasm instructions: pi-like −3.3%, props −3.3%, jsonh −1.6%, the rest ±1%.
  - The `base_region_scan_cost` bench now seals its base (a full collection no longer
    makes everything old since 0120).
- Item 2: growth = max(live, MIN_GROWTH) + k·young (k = 1, 2, 3), and a variant with
  the extra capped at MIN_GROWTH (and at half of it).
  - Instructions −2 to −7% on pi-like and the JSON workloads.
  - Peak memory, 24-run means: +0.3 to +0.9 MiB with the cap (pi-like +17%), up to
    +6 MiB uncapped.
  - Not adopted (D2056). The trade is recorded and can be reopened.
- Checked:
  - e2e plain, heap and vm unit tests;
  - js tests and native-vs-wasm with the debug build;
  - difftest codemode and console with determinism, GC stress and suspension.
- Raw numbers: work/scratch/0134 (pt-*.txt).
- Guide gc.md: the slot plans in "The old region", the card trade-off's cost updated, a
  new trade-off about the threshold.

## Follow-ups

## Result
- **Landed:** the old-region walk uses per-kind slot plans with the young check inlined
  (D2055). pi-like and props −3.3% wasm instructions; no workload slower beyond noise;
  peak memory unchanged (the 24-run distributions match main's); +459 B gzip, under the
  ceiling by 35 B.
- **Measured, not adopted:** a threshold that adds the young part (D2056). −2 to −4%
  instructions with the cap, for +0.3 to +0.9 MiB peak (pi-like 2.81 → 3.30 MiB). It can
  be reopened if per-VM memory matters less than instructions.
