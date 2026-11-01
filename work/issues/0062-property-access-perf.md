# 0062: Property access performance: shapes / inline caches (design first)

status: in-progress
agent: heap
branch: issue/0062-property-access-perf
depends-on: 0060
touches: docs/ (a design note first), then crates/djs-core/src/heap/ (object layout), crates/djs-core/src/interp/ (caches in the dispatch loop), crates/djs-core/src/bytecode/ (cache slots per instruction, if needed), js/src/layout.generated.ts + js/src/reader.ts (if the object layout changes), crates/djs-bench/

## Goal
After 0035, property access is the main structural interpreter cost: a hash or linear
probe per access (0035 Result: "props" at 2.5x qjs; builtin-language code is all Get/Set
intrinsics). pi codemode scripts are 4x slower than quickjs-wasi in script work (0049),
"all in the interpreter". Find the right design for djs's constraints (a
position-independent arena, a copying GC, deterministic snapshots, the JS-side reader)
and implement it.

## Acceptance criteria
- [ ] Measurements first: where property access time goes now (lookup, the Get/Set
      intrinsic call overhead from the builtin language, prototype-chain walks for
      methods), on the engine track, a pi-like codemode script and test262's slow tests.
- [ ] A design note (docs/objects.md or a section in ARCH): options compared with
      estimated gains. For example: hidden classes/shapes as heap cells shared between
      objects (transition trees, a shape cache invalidated on GC moves?), monomorphic
      inline caches per instruction (cache = (shape, slot)), a cheaper per-instruction
      cache keyed by property-table identity, prototype method caches with a validity
      cell, and what each does to snapshots, the GC (are shapes rooted, collected,
      deterministic?), the reader (0021) and memory per object. **Send it to the
      coordinator before implementing**: this is an ARCH-level change.
- [ ] Then the implementation, in parts, with before/after numbers (engine track, pi bench,
      test262 CPU time), no semantic changes (ratchets, stress modes identical), and
      determinism kept (D1090, D1601).

## Log

## Follow-ups

## Result
