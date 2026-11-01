# 0136: Keep a VM's first changes from shifting the base image (atom table, growable cells)

status: done
agent: heap
branch: issue/0136-base-layout-stability
depends-on: 0117, 0133 (analyzer), 0134
guide: docs/guide/snapshots.md (why snapshots are small), gc.md if compaction changes
touches: crates/djs-core/src/heap/ (atom table, shapes/transition lists, PropTable growth, compaction around the old region), crates/djs-core/src/vm/ (prepare_base_image), djs-prelude build; bloat (0135) is investigating the same dumps and may propose more; snapshot (0133) owns the analyzer

## Goal
snapshot's analyzer (`djs heap --diff base`) on a REPL dump: the session's own data is
7.4 KB new + 40 KB changed, of which 30.3 KB is the atom table's entries array copied by the
first new atom, plus globalThis's PropTable/index (4 KB); that one realloc shifts the
layout after 68 KB, so only 14.5% of words stay in place. Same cause as 0117's follow-up
"Sandbox snapshots at a few KB" (an empty Sandbox's snapshot is 15 KB, a pi waiting session
2–9 KB).

Make a VM's first changes leave the base layout in place: e.g. slack in the base's
growable cells (atom entries, root shape transitions, global PropTable) sized at image
build, an atom table that grows by chunks (append a new chunk instead of copying), or
new/grown versions of base cells allocated in the young region without the compaction
sliding the old region (the old region doesn't move since 0120, so check why it shifts
at all: the dumps may be canonical/Cheney-collected rather than in-place).

## Acceptance criteria
- [x] Explain the mechanism with the analyzer (which collection produced the dump, what
      moved and why).
- [x] Fix so an empty Sandbox's and a fresh REPL's snapshot containers are a few KB, and
      pi waiting sessions don't get worse; measured with scripts/snapshot-research/wasm.ts.
- [x] Image size (size ratchet: the base image is in the module), vm_new time and
      determinism unchanged or trades recorded.

## Log
- Mechanism (analyzer on a fresh REPL `.snapshot --raw`):
  - The dump comes from a full collection: the REPL, `Sandbox.bytes()` and pi's
    `snapshot()` all call `collect`, and full collections compact the old region to its
    live cells (D2041).
  - A fresh VM replaces the atom entries (30 KB at ~68 KB in the base, by the first new
    atom), the global object's PropTable and index (4 KB, by the first global) and the
    root shape's transitions. Dropping them slides everything after them: 14.7% of words
    in place.
- Fix (D2057), following bloat's 0135 proposals 1–2 and snapshot's note on pi's `pending`
  Map:
  - `Heap::first_grown` lists these cells (Maps/Sets first, atom entries last);
  - the base build puts them last in `order_for_image`;
  - `Vm::seal` copies them to the end before its collection, so pi's prelude image gets
    the same layout;
  - atom entries grow by 25%.
- Measured (main → branch):
  - Sandbox new 15.7 → 0.8 KB, with 1000 records 31.2 → 15.7 KB;
  - REPL fresh 16.0 → 0.8 KB; raw dumps −11 KB;
  - pi waiting sessions equal or −0.7 to −0.8 KB;
  - `vm_new` and pi starts unchanged.
  - Raw notes in work/scratch/0136 (sr-main, sr-b4 reports).
- Size: +588 B gzip in shipped code (relocation and quarter growth), cut down from +2.1 KB
  raw. Over the ceiling, so the trade is recorded (D2058, ceiling 455,619 → 456,219).
- Checked:
  - djs-core, prelude and wasm tests, plus the new seal test (Map iterator across a seal,
    ≥90% of words in place after a full collection);
  - e2e plain, js tests, native-vs-wasm, difftest determinism.
- Guide: snapshots.md (new section on growable cells, Sandbox number), gc.md (seal
  relocation).

## Follow-ups
- The global lexical record (`<global let>`) gets its PropTable at the first top-level
  `let`. It is new, not moved, so nothing shifts today; if it becomes common in base
  images, add it to `first_grown`.
- The size ceiling (D2058) could come back down if bloat's 0135 shrinks the realm.

## Result
- A VM's first changes no longer shift base or sealed images (D2057).
  - Empty Sandbox snapshot 15.7 → 0.8 KB, fresh REPL dump 16 → 0.8 KB, raw images −11 KB.
  - pi waiting sessions equal or slightly smaller.
  - `vm_new` and determinism unchanged.
- Size trade: +588 B gzip, ceiling raised to 456,219 (D2058).
