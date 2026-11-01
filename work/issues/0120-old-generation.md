# 0120: Old generation, phase 1 (sticky non-moving base prefix)

status: done
agent: heap
branch: issue/0120-old-generation
depends-on: 0101
guide: docs/guide/gc.md, docs/guide/determinism.md, docs/guide/snapshots.md (if images change)
touches: crates/djs-core/src/heap/ (gc, mark-compact, roots), crates/djs-core/src/vm/ (Vm::seal), crates/djs-wasm/ (vm_seal), js/pi/ (calling seal after the prelude), docs/old-generation.md, ARCH.md (Heap/GC)

## Goal
heap's design (docs/old-generation.md, on main via 0101): treat the arena prefix [0, B)
as old (live, not moved); minor collections trace young cells from the roots plus all
old→young references found by a linear walk of the old region (no write barrier); full
collections at seal/snapshot points and on a deterministic schedule; B in the image;
`Vm::seal()` after a full collection (pi calls it after its prelude). Expected: GC time
−55–65% on base-dominated workloads (props), −3–6% on pi-like/json/strings.

## Acceptance criteria
- [x] Phase 1 as designed, with determinism kept (B and the schedule are image state; raw
      images equal for equal histories; canonical_hash unaffected) and every gate check
      green.
- [x] Measured: GC time and total wasm instructions on the 0081/0087 workloads and the pi
      bench, peak memory unchanged or better. Phase 2 (cards + barrier) decided from the
      numbers: propose, don't build.
- [x] Also, if cheap: 0101's follow-up, sizing the arena once before compiling the prelude
      for cold pi starts.
- [x] Guide pages updated (D2102 rules).

## Log
- Built phase 1 in compactor.rs: `mark_compact(old)`; `old == 0` is the full
  collection. The minor path starts from `mark_old`, a walk over `[0, B)` that marks
  young targets of old slots. Bitmap and block table are relative to B (`Live`); old
  ephemeron tables are found by hop walks; old collections with young object/symbol keys
  are flagged (`gc_bits::REHASH`, formerly the unused `OLD`) and rehashed.
- First policy tried: full collections promote (B = after), plus a full collection when
  young live > B. Collection time fell further, but peak memory rose up to 76%
  (higher-order 28.4 → 45.3 MiB, jsonh 6.8 → 12.0, pi-like 4.2 → 6.3). Rejected (D2042):
  full collections now shrink B to the live bytes below it; only `seal` and canonical
  collections set B = len.
- The first version walked the old region twice (mark, then update): minor 0.155 ms vs
  full 0.177 ms on a base-only heap. A remembered list of old slots with young targets
  (fixed 16 KiB; overflow falls back to the second walk; the stack-cap test hook shrinks it
  so the fallback is tested) brought it to 0.080 ms (scan alone 0.043).
- `release_top` can truncate below B if a full collection ran between a suspension
  record's two runs: it lowers B then.
- `Heap::verify` rejects a B that is not a cell boundary.
- Cold start sizing: reserving after `vm_new` moves the arena, which is the hole itself
  (2.9–4.1 MiB). Sizing at creation works: `vm_new_reserved(limit, live)`; pi passes the
  size its previous cold start sealed. Cold peak 3.00 → 2.25 MiB from the second cold
  start in a process on.
- strbuild peak memory 25.5 → 31.9 MiB, with arena sizes within 0.6% of main. A
  FULL_EVERY=1 build gives 25.5. A build whose first collection is full gives 25.5 for
  strbuild but 18.6 (from 12.9) for strings. So these are host allocator placement
  effects of ~17 KB of retained dead old cells (the realm tables grown early), not more
  live memory.
- Numbers: D2041; raw notes in work/scratch/0120/numbers.md.
- Guide: gc.md (new section "The old region", In short, schedule), determinism.md
  (minor/full choice is image state; canonical hash independent of B), snapshots.md
  (images carry B and the schedule; base is old; pi seals; cold-start hint; the deltas
  trade-off wording). ARCH Heap/GC and Snapshots/Phases; docs/old-generation.md section
  10 "As built".

## Follow-ups
- Promotion with aging (cells that survived two minor collections become old) for
  young-dominated workloads (arrays, higher-order, json), where a minor collection costs
  about a full one today. It has to keep peak memory (D2042).
- Peak wasm memory is sensitive to tiny arena size shifts (strbuild ±6 MiB from 17 KB).
  Reserving the arena's capacity target eagerly after a collection, at a moment with few
  Rust allocations live, might make it robust. Measure it.
- Phase 2 (cards) only if the sealed old region grows to several MB (D2044).

## Result
- **Old region:** `Roots::OLD_END` / `MINORS` (Roots min size 80).
  - The safepoints call `Heap::collect_auto`: minor while B > 0, full every 16th, and
    the next one full when a minor collection leaves more than half the limit in use.
  - `Heap::collect` / `Vm::collect` / `vm_collect` stay full and shrink B to its live
    part. `Heap::seal` / `Vm::seal` / `vm_seal` is full, then B = len.
  - `collect_canonical` seals, so the base image is all old and canonical hashes ignore B.
  - Tests: gc_tests (minor == full graph, with stack overflow and remembered-list
    fallback; scheduled collections then a full collection equal a single full
    collection; the schedule survives image round trips; retention; old WeakMap/Map with
    young keys; old ropes flattened later) and vm tests (a sealed VM under suspension and
    GC stress; reserved bases).
- **pi:** seals after its prelude. Its later cold starts use `vm_new_reserved` with the
  last sealed size (D2043).
- **Measured:**
  - Native collection time −8% to −51%: props −51%, strings −42%, pi-like −38%,
    strbuild −30%.
  - Wasm instructions: pi-like −5.7%, props −5.3%, strings −1.4%, the rest about ±0;
    json +1–2%, half of it also in a control build.
  - pi bench tool-heavy 3.65 → 3.28 ms. pi cold peak 3.00 → 2.25 MiB (from the second
    cold start on), warm unchanged.
  - Peak memory is unchanged on 17 of 22 workloads, +0.1–0.2 MiB on four. strbuild
    +6.4 / strings −1.2 MiB are allocator placement, see the log.
  - Wasm +5.3 KB (+2.1 KB gzip).
- **Phase 2 (cards):** not proposed for now (D2044). It could save at most the ~0.04 ms
  old walk per minor collection (≤2.5%), against a barrier on every traced store.
- **Decisions:** D2041–D2044.
