# 0142: Goldilocks snapshots: minimal, and a chance to survive djs/prelude/globals changes

status: open
agent: -
branch: issue/0142-goldilocks-snapshots
depends-on: 0117, 0136, 0140, 0141
guide: docs/guide/snapshots.md (later, if adopted)
touches: docs/goldilocks-snapshots.md (design report), scripts/snapshot-research/ (prototype, extending 0141's portable crate), work/scratch/0142/; js/pi/ only in scratch copies; no engine changes on main

## Owner's framing
"It's fine if heap snapshots eventually fail, but we want the goldilocks state where
snapshots are minimal and have a chance to survive djs updates. We probably won't update
djs often, but even changing globals probably invalidates the snapshot between runs?"

## Facts to start from
- Today's pi snapshot (D2260) is an image zstd-compressed against a reference image =
  the prelude image computed from (engine build, prelude source, start data: tools,
  globals, store, memory limit, deterministic mode) (js/pi/vm.ts preludeKey). So a
  different tool list, global set, `store` content, or memory limit gives a different
  reference, and restore is refused unless the host supplies the original start data.
  Every djs build and every prelude edit also invalidates it.
- 0141 (docs/portable-snapshots.md) shows a by-meaning export (code by source, awaits by
  position + roles, builtins by spec path) survives djs builds; it assumed the prelude
  text is unchanged and carried the corpus prelude as graph.

## Goal
Design the sweet spot, with a prototype and numbers, along these axes:
1. **Layered references**: make the reference depend only on what rarely changes
   (engine build, then prelude source), and carry per-run inputs (tools, globals, store,
   limits) as small session data instead of baking them into the reference image. What
   does that cost in size? Does it make "changing globals/tools/store between runs" a
   non-event for same-build restores?
2. **Survival tiers**: same build + same prelude → today's image delta (exact, ~1 ms);
   different djs build or prelude → a by-meaning export of only the **session-owned
   state** (the analyzer's "new/changed vs reference"), with everything else named
   (builtins by spec path, prelude entry points/records by name: `tools`, `text`,
   pending tool call #n, store), restored onto a fresh prelude run. Fails cleanly when a
   name is gone or a prelude function is suspended.
3. **What pi's prelude must look like** for tier 2 to work (read pi's prelude in
   ~/Development/pi-private, read-only, and js/pi): which prelude objects and frames are
   reachable from user state at real idle points (use `djs heap` on real pi idle
   snapshots from scripts/snapshot-research), whether prelude functions are suspended
   while waiting (tool wrappers as async functions?), and what small interface contract
   (names + pending-call record) pi would have to keep.
4. **Churn reality check**: how often did pi's prelude interface (not internals) change in
   pi-private's git history (read-only `git log` is fine), vs djs's build churn (0140).
5. Minimal size: can one format serve both tiers (e.g. the by-meaning export as the only
   format, compressed against a by-meaning export of the reference), and what would the
   sizes and times be compared with today's 2–9 KB / ~1 ms?

## Deliverable
docs/goldilocks-snapshots.md: the recommended design (format, layers, tiers, what pi must
keep stable, what fails and how), numbers from a prototype, cost in engine work and
module size, and phases. Be concrete and short; the owner wants a decision basis.

## Log

## Follow-ups

## Result
