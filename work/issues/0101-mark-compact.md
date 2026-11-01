# 0101: In-place mark-compact collection (design first; ARCH-level)

status: in-progress
agent: heap
branch: issue/0101-mark-compact
depends-on: 0098, 0099
guide: docs/guide/gc.md, docs/guide/testing.md (the collected-image layout), docs/guide/snapshots.md, docs/guide/determinism.md (the layout bullet, the gate section)
touches: docs/ (a design note first), crates/djs-core/src/heap/gc.rs (+ a new compactor module), crates/djs-core/src/heap/ (marking bitmaps), ARCH.md "Heap" (after approval)

## Goal
num's 0099 measured that what remains of wasm peak memory is fundamental to Cheney: from-
and to-space coexist during a collection (~2x live), plus dlmalloc fragmentation from Vec
moves (dlmalloc never returns memory; realloc grows in place only with top-chunk room).
An in-place sliding (mark-compact) collector for the VM's own collections would drop
every heavy workload's GC peak by about one semispace and bring a pi cold start to ~2.0
MiB (estimate). See work/issues/0099-cold-start-memory.md (Result) for the numbers.

## Acceptance criteria
- [ ] **Design first** (send to the coordinator before implementing): the algorithm (mark
      bitmap, per-block offsets ~len/32 bytes, sliding order), ephemerons and weak refs,
      the flattened-rope short-circuit, Map/Set rehash, IC offset clearing, D1700/D1600
      interplay, determinism: canonical_hash keeps a Cheney copy so journals stay
      graph-canonical (D1730), while raw images change to address order, history-identical
      across the gate's modes (D1090/D1710). Also when to use which collector (for example
      compaction for in-VM collections, Cheney for image canonicalization), the speed cost
      (3 passes), and the code size cost (D016).
- [ ] After approval: implementation, GC tests (collect twice identical bytes, the 0071
      slice tests, native-vs-wasm), and memory/time numbers on the 0081/0087 workloads and
      the pi bench.

## Log

- Design note: docs/mark-compact.md (Compressor-style sliding collector: header mark bit,
  live-word bitmap, 512-byte block table, bounded mark stack with overflow rescan; Cheney
  kept for canonical_hash and image canonicalization). Sent to the coordinator for review.
- Coordinator: prototype on the branch and measure; don't merge before the owner decides.
- Prototype (heap/compactor.rs; Heap::collect = mark-compact, Heap::collect_canonical =
  Cheney for canonical_hash, the base image, vm_new_compiled and the djs-prelude test).
  - The first version was 10x slower on a long list: a 64 K mark stack plus full
    rescans, and ropes accumulated on the stack.
  - Fixed with a growable stack (up to len/64), leaf cells not pushed, and rescans from
    the lowest unpushed cell.
  - Then: rope slots rewritten while marking, slides by runs, the dense prefix.
  - Numbers in docs/mark-compact.md section 10.

## Follow-ups

## Result
