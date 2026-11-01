# 0136: Keep a VM's first changes from shifting the base image (atom table, growable cells)

status: in-progress
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
- [ ] Explain the mechanism with the analyzer (which collection produced the dump, what
      moved and why).
- [ ] Fix so an empty Sandbox's and a fresh REPL's snapshot containers are a few KB, and
      pi waiting sessions don't get worse; measured with scripts/snapshot-research/wasm.ts.
- [ ] Image size (size ratchet: the base image is in the module), vm_new time and
      determinism unchanged or trades recorded.

## Log

## Follow-ups

## Result
