# 0099: Cold-start wasm memory (the post-prelude collection) and verify memory

status: done
agent: num
branch: issue/0099-cold-start-memory
depends-on: 0094
touches: crates/djs-core/src/heap/ (gc, verify), crates/djs-wasm/, js/pi/engine.ts (only if the cold-start sequence changes), docs/host.md

## Goal
0094 cut warm starts to 2.13 MiB, but a pi cold start (running the prelude) is still 3.81
MiB, dominated by the one collection after the prelude, which holds both semispaces at
once. Also the image verify pass's temporary list (~80 KiB).

## Acceptance criteria
- [x] Options measured for that one collection: an in-place sliding compaction for
      "collect then snapshot" (D1890's post-prelude collect), collecting into the staging
      buffer the image will be stored in anyway, or anything better. Determinism (D1090:
      the collected bytes must be identical to a Cheney collection's, or the change is
      recorded as a new canonical form across the gate's determinism checks).
- [x] A bitmap instead of the temporary list in verify (~80 KiB → ~9 KiB).
- [x] Peak wasm memory for pi cold start and Sandbox creation before/after; no time
      regression for vm_new or warm starts.

## Log
- Accounting (djs_alloc_stats, work/scratch/0099/cold.ts = pi bench cold path with export
  logging): the 3.81 MiB cold peak is an ordinary GC inside the prelude run, not D1890's
  post-prelude collect: arena exactly at its threshold (832K) -> overshoot before the
  safepoint reallocates it to 1040K (dlmalloc move, 832K hole) -> to-space 1040K.
- Slack variants (all / restore-only), to-space sized for the next threshold (5.5 MiB),
  reserve after freeing the from-space (4.0 MiB): see D1990's table. Taken: slack at
  base/restore only.
- verify: bitmap of cell starts + bitmaps for the proto-cycle check (D1991);
  vm_from_bytes no longer grows wasm memory.
- Checks: djs-core/prelude/wasm tests, js tests (48), native-vs-wasm.

## Follow-ups
- In-place (sliding, mark-compact) collector for the VM's own collections, design-first, to
  heap after 0098 (coordinator). Input in the Result below.

## Result
Landed (D1990, D1991): a base/restored arena reserves its GC threshold plus a slack, so the
first threshold crossing does not move it; `verify` uses bitmaps. Peak wasm memory, MiB:

| case | before | after |
| --- | ---: | ---: |
| pi warm, `return 1` | 2.13 | 2.06 |
| pi warm, tool-heavy | 2.13 | 2.06 |
| pi cold, `return 1` | 3.81 | 2.81 |
| pi cold, tool-heavy | 3.81 | 4.00 |
| Sandbox creation | 1.69 | 1.75 |

`vm_new` and warm-start times are unchanged.

**Input for the mark-compact issue.** What remains of cold-start memory is structural:
- Cheney needs the from-space and the to-space at once, about twice the live data.
- dlmalloc in wasm never returns memory. Its realloc grows in place only if the top
  chunk already has room, so every arena growth past its capacity is a move that leaves
  the old block as a hole.
- The holes are too small for the next, larger arena, so the high-water mark climbs:
  cold tool-heavy is hole + arena + to-space + a grown arena.
- Placement experiments cannot fix this; they lose in other cases. Two were tried:
  - a to-space sized for the next threshold: 5.5 MiB;
  - an immediate reserve after the collection: 4.0 MiB.

A sliding mark-compact for the VM's own collections would remove the to-space:
- **Memory:** a mark bitmap plus per-block forwarding offsets, about len/32 bytes. The
  cold peak would drop to about arena + growth (estimated 2.0 MiB for `return 1`), and
  every workload's GC peak by about one semispace.
- **Determinism:**
  - `canonical_hash` (D1730) would keep Cheney on a copy, so journals stay
    graph-canonical.
  - The compactor is deterministic and preserves address order. Raw images are already
    identical across slicing and restore (0071), so the gate's collected-image
    equalities hold.
  - Collected images change form (address order instead of BFS); the build hash changes
    anyway.
- **Cost:** a second collector (~400–500 lines, with ephemerons, weak refs, the
  flattened-rope short-circuit, Map rehash and IC offset clearing) and three passes
  instead of one.
