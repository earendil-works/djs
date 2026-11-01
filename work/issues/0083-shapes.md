# 0083: Shapes (hidden classes) and a leaner object/image layout

status: in-progress
agent: heap
branch: issue/0083-shapes
depends-on: 0062, 0063
touches: crates/djs-core/src/heap/ (object layout, property tables, shapes as cells), crates/djs-core/src/interp/ (the 0062 IC hit checks), crates/djs-core/src/heap/gc.rs + walk.rs, js/src/layout.generated.ts + js/src/reader.ts (the reader must read the new layout), docs/objects.md, docs/wire.md, ARCH.md "Heap" (objects paragraph)

## Goal
docs/objects.md part 2 plus 0052's S2 (work/issues/0052-bloat-investigation.md): shared
shapes / hidden classes, which mainly pay off in memory per object and base-image size (a
4-property object ~144 B → ~56 B; the 424 builtin `length`/`name` tables become one shared
shape; 16-byte dictionary entries), plus cheaper IC hits (a shape compare instead of a key
re-check).

## Acceptance criteria
- [ ] A design update in docs/objects.md first (shape cells, transitions and their cache,
      dictionary mode, prototypes and shapes, interaction with the copying GC and with
      deterministic images (D1090: shape trees must be deterministic), snapshots, the JS
      reader, the 0062 ICs). **Send it to the coordinator before implementing**; it's an
      ARCH change.
- [ ] Implementation in parts, with no semantic changes: ratchets, stress modes,
      determinism checks (0072), and the native-vs-wasm image check all stay identical or
      better.
- [ ] Numbers: memory per object (benchmarks with many small objects, a JSON parse of
      10k records), base image size (base.img, wasm data section), the engine track,
      the pi bench. Recorded only on a quiet machine.

## Notes
- infra (0080) works on native builtins and the iteration fast paths, which use
  `get_element_fast`/`array_length` (elements, not named properties). Keep those
  entry points stable, or coordinate through the coordinator.
- 0070 follow-up for later: native builtins have no collect-and-retry yet.

## Log

## Follow-ups

## Result
