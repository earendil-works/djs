# 0147: Immutable builtin prefix: session-only images, hash, GC and determinism

status: open
agent: -
branch: issue/0147-immutable-prefix
depends-on: 0144
guide: docs/guide/gc.md, snapshots.md, determinism.md, arena.md
touches: crates/djs-core/src/heap/ (roots, compaction, old region, canonical_hash, image write/read, verifier), crates/djs-core/src/vm/ (per-VM state moved out of the base region, prepare_base_image, seal), crates/djs-wasm/, js/src/snapshot.ts (images from the session region), crates/djs-cli/src/heap/ (analyzer), scripts/snapshot-research; perf (0146) is in interp/ and builtins/ write paths

## Goal
docs/frozen-builtins.md phase 2: the base image becomes an immutable prefix of every arena
(same offsets in every VM of a build). Everything written at run time lives above it:
- first, as an early small merge: a `Roots::BASE_END` field (the prefix boundary, distinct
  from OLD_END), set by prepare_base_image, so 0146 can build on it;
- move the per-VM state out of the prefix (0144's write audit: value stack, interp
  registers/roots/jobs/handles, the atom table's index and counters, lazily computed string
  hashes, prototype header flags, shape descriptor counts, the global object's property
  table, the realm slot): into the session region at vm_new, or computed eagerly at
  image build (string hashes, flags);
- a debug/test write barrier (Arena::write in debug and the gate's stress modes) that
  traps any write below BASE_END;
- images, snapshots (D2260 containers), canonical_hash, collections (never mark/move the
  prefix) and determinism defined over the session region only; restore pastes the
  session region onto the build's prefix; the format records the prefix's build hash;
- the analyzer (`djs heap`) understands the split and prints builtin references by
  registry id when 0145 lands.

## Acceptance criteria
- [ ] Write barrier clean on the whole gate (e2e, difftest stress, test262 active, js tests,
      pi suite).
- [ ] Snapshot/image sizes, vm_new, GC time, wasm instructions and peak memory measured
      before/after (mem-perturb); no workload slower than +1% without a recorded trade.
- [ ] Order-independence test: two runs touching different builtins in different orders
      give identical session images and canonical hashes.
- [ ] Guide pages updated (D2102 rules).

## Log

## Follow-ups

## Result
