# 0051: Remaining builtins: iterator helpers, WeakRef/FinalizationRegistry, disposables, Array.fromAsync, upsert

status: in-progress
agent: heap
branch: issue/0051-remaining-builtins
depends-on: 0046
touches: builtins/ (iterator.ts, weakref.ts, disposable.ts, …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/heap/ (weak references in the GC, additive), crates/djs-core/src/interp/realm.rs (additive), crates/djs-core/src/vm/ (the host hook for FinalizationRegistry cleanup scheduling, additive), tests/test262.toml (promotion), ratchets

## Goal
Most of what is left in test262's later group is builtins on top of machinery that now
exists (generators/iterators from 0029/0037/0042, the ephemeron GC from 0038):

## Acceptance criteria
- [ ] Iterator helpers (ES2025): the global `Iterator` constructor (abstract),
      `Iterator.from`, `Iterator.prototype.{map, filter, take, drop, flatMap, reduce,
      toArray, forEach, some, every, find}`, `%IteratorHelperPrototype%`,
      `%WrapForValidIteratorPrototype%`, `[Symbol.toStringTag]` and `constructor`
      accessors, and the `iterator-sequencing` / `Iterator.concat` features if they are
      in the test262 commit. Codemode scripts will use these on arrays' iterators.
- [ ] `Array.fromAsync` (the async-iteration builtin).
- [ ] `Map.prototype.getOrInsert / getOrInsertComputed` (and WeakMap) (upsert).
- [ ] WeakRef and FinalizationRegistry: weak references in the Cheney GC (cleared when the
      target dies; `deref` per spec, with the KeepDuringJob set cleared at the end of each
      job), and FinalizationRegistry cleanup callbacks scheduled as jobs after a
      collection. A host hook decides when (ARCH: the embedder controls
      nondeterminism, so with the default, cleanup runs as a job after a collect). The
      GC must stay deterministic (D1090).
- [ ] DisposableStack, AsyncDisposableStack, SuppressedError, and
      `Symbol.dispose/asyncDispose` (the runtime parts of explicit resource management;
      the `using` syntax is a later compiler issue).
- [ ] Promote everything implemented from later to active (and `__proto__` if 0044's
      accessor covers the tests). Report the numbers before and after the move.
- [ ] behavior.js / difftest / e2e (three modes) and the ratchets.

## Log

## Follow-ups

## Result
