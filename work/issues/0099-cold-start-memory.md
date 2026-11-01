# 0099: Cold-start wasm memory (the post-prelude collection) and verify memory

status: in-progress
agent: num
branch: issue/0099-cold-start-memory
depends-on: 0094
touches: crates/djs-core/src/heap/ (gc, verify), crates/djs-wasm/, js/pi/engine.ts (only if the cold-start sequence changes), docs/host.md

## Goal
0094 cut warm starts to 2.13 MiB, but a pi cold start (running the prelude) is still 3.81
MiB, dominated by the one collection after the prelude, which holds both semispaces at
once. Also the image verify pass's temporary list (~80 KiB).

## Acceptance criteria
- [ ] Options measured for that one collection: an in-place sliding compaction for
      "collect then snapshot" (D1890's post-prelude collect), collecting into the staging
      buffer the image will be stored in anyway, or anything better. Determinism (D1090:
      the collected bytes must be identical to a Cheney collection's, or the change is
      recorded as a new canonical form across the gate's determinism checks).
- [ ] A bitmap instead of the temporary list in verify (~80 KiB → ~9 KiB).
- [ ] Peak wasm memory for pi cold start and Sandbox creation before/after; no time
      regression for vm_new or warm starts.

## Log

## Follow-ups

## Result
