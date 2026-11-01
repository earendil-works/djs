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

- Measurements and design note: docs/objects.md (tools in work/scratch/0062: micro.py,
  classify.py, leaf.py, opcount.patch). Property access costs 10-30 ns vs ~1 ns in qjs,
  globals up to ~100 ns; but in a pi-like codemode script 97% of executed bytecodes are
  the builtin language and property lookup is 3.5% of the time. Proposal: cheaper lookups
  plus verified-index inline caches (globals, own properties, prototype methods with an
  epoch) now; shapes later as their own issue merged with 0052 S2 (memory/image), after
  0063. Sent to the coordinator for approval before implementing. Approved (globals
  first; caches in the heap; warm = cold restore test; stress modes identical).
- Part 1a, global caches (D1620): the `Cache` immediate on the six property/global ops
  (bytecode format, so OPS_HASH and the build hash change), FuncTemplate IC_COUNT/IC,
  interp/run/ic.rs, Heap::own_entry_index/data_at/set_data_at, realm GlobalLexEpoch.
  Tests: e2e global_caches.js (one qjs bug avoided: qjs keeps throwing after a
  read-only global becomes writable), vm/tests.rs global_caches_follow_every_change and
  warm_and_cold_inline_caches_behave_the_same. Measurements (LTO, cgu=1, interleaved,
  scratch ab.py/micro_ab.py): global read 10.4 -> 1.3 ns; test262 decodeURI test -13%;
  engine track within noise (float -4%: its time is the prelude Math functions). Found
  that default release builds vary ±15% between builds from layout (noted in D1620).

## Follow-ups

## Result
