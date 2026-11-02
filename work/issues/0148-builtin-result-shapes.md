# 0148: Order-independent shapes for objects that builtins create

status: in-progress
agent: heap
branch: issue/0148-builtin-result-shapes
depends-on: 0147
guide: docs/guide/determinism.md, snapshots.md (the session-image guarantee)
touches: crates/djs-core/src/heap/shape.rs, crates/djs-core/src/vm/ (prepare_base_image warm-up), crates/djs-prelude/

## Goal
0147 lands with the guarantee "builtins leave no trace in the prefix; shapes of objects
created by builtin calls are session state, ordered by first creation". The owner's goal is
that the order and time of builtin use don't matter for the session image. Objects that
builtins create (iterator results {value, done}, RegExp match results, property
descriptors, ...) get their shapes in the session region in creation order (shapes never
die, D1831), so `new Map(..)` and `/a/.exec()` leave order-dependent shapes.

Steps, smallest first:
1. Pre-create at base build the shapes that builtins produce (from a survey of builtin
   result objects plus the harvested corpus), so builtin calls use prefix shapes.
2. Only if needed: transitions from prefix shapes to new session shapes kept in a per-VM
   side table (back to one shared root shape); never append descriptors in place to a
   prefix descriptor array.
Measure how often scripts extend builtin-created objects (harvested corpus) before doing 2.

## Acceptance criteria
- [ ] Order-independence test extended: runs using Map/Set/RegExp/iterators/descriptors in
      different orders give identical session images and canonical hashes.
- [ ] Instructions (±1%), module size, base image size measured.
- [ ] Guarantee restated in the guide.

## Log

## Follow-ups

## Result
