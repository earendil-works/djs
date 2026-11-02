# 0142: Goldilocks snapshots: minimal, and a chance to survive djs/prelude/globals changes

status: done
agent: portable
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

- Read js/pi/vm.ts (preludeKey, prepare, startVm, resumeVm) and pi's prelude
  (pi-private packages/codemode/src/runtime/prelude-source.ts, read-only; identical to
  tests/pi/prelude-source.ts), its protocol (VmStartData) and its git history (8 commits,
  2026-09-29 .. 10-12).
- Extended the 0141 crate (scripts/snapshot-research/portable): pi.rs (snaplab's pi
  session, now with tools/globals/store inputs and a factored-prelude path), pitools.rs
  (snaplab's fake tools), goldi.rs (`goldi-images`, `goldi-tier2`, `goldi-import-all`,
  `goldi-reloc`), factor-prelude.py (pi's prelude split into stage 1 + configure),
  refs.py, churn-tiers.py.
- Layered reference: memory limit does not change the image (removed from the key);
  the unmodified prelude with empty inputs as reference costs +15 KB; the factored
  prelude +1.6/+3.8 KB (small/realistic inputs). Found two causes of moved cells: handles
  dropped after the reference was sealed (dead cells inside the reference), and growing
  shape transition lists (warmed in stage 1; engine follow-up).
- Tier 2 for pi: prelude objects by path (Proxy `<target>`), pending calls by id with
  `adopt` emulated, prelude state by binding name, `run` re-subscription, interface
  check, refusal of suspended prelude frames. 133/144 same build (agent-loop refused:
  async generator); all 8 pi prelude versions onto the current and back; djs 1b6c2098
  (rebuilt in work/scratch/0142/old) <-> today with first/current preludes: 133/133.
- Importer GC bug (D2351): offsets held across VM runs moved; fixed with handles, also in
  0141's import.rs.
- Relocation option: measured references from session cells into the reference and the
  changed reference cells; churn per tier and per release window.
- Used 0143's numbers (registry, engine-level lockdown) per the coordinator.
- Wrote docs/goldilocks-snapshots.md. No engine changes; guide unaffected (snapshots.md
  describes what is built).

## Follow-ups

- Adapter: drop the memory limit from `preludeKey` (images do not depend on it).
- pi-private: split the prelude factory into stage 1 + `configure`; later add
  `state()`/`restore()`/`adopt(id)` and keep "no awaits in prelude code".
- djs-core: `Vm::seal` puts shape transition lists last (as D2057 does for the global
  table), so per-session shapes do not move the reference.
- Tier 2 in djs-core with 0143's builtin registry (0141's phase 1 plus pi naming);
  async generators if ever needed.

## Result

docs/goldilocks-snapshots.md: layered references (the reference keyed only by build +
prelude version, per-run inputs carried by the session for +1.6–3.8 KB), tier 1 = today's
image delta, tier 2 = by-meaning export of session-owned state restored onto a fresh
prelude run (prototyped: survives every pi prelude version in history and a 406-commit
djs jump, same size as today's deltas, import 3.0 ms native or ~0.7 ms with a cached
stage-1 image or engine lockdown), what pi must keep (global names, settle/run/stalled,
no prelude awaits, a state/restore/adopt contract; interface unchanged in its history),
churn numbers, and an explicit evaluation of image-with-symbolic-relocations (few
relocations, but it freezes layouts/opcodes/atoms: survives 8% of 10-issue releases
during development). Phases A (days), B (with 0143 steps 1–2), C (3–4 weeks).
