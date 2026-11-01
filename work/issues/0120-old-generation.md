# 0120: Old generation, phase 1 (sticky non-moving base prefix)

status: open
agent: -
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
- [ ] Phase 1 as designed, with determinism kept (B and the schedule are image state; raw
      images equal for equal histories; canonical_hash unaffected) and every gate check
      green.
- [ ] Measured: GC time and total wasm instructions on the 0081/0087 workloads and the pi
      bench, peak memory unchanged or better. Phase 2 (cards + barrier) decided from the
      numbers: propose, don't build.
- [ ] Also, if cheap: 0101's follow-up, sizing the arena once before compiling the prelude
      for cold pi starts.
- [ ] Guide pages updated (D2102 rules).

## Log

## Follow-ups

## Result
